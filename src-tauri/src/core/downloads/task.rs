//! A single file to fetch, and the logic to fetch it: stream to a temp
//! file (hashing as it writes), verify, then atomically rename into place.
//! Used for every kind of download this launcher does - client jars,
//! libraries, assets, and (later) Modrinth content - so the same
//! guarantees (atomic writes, hash verification, retry) apply everywhere.

use std::path::PathBuf;
use std::time::Duration;

use sha1::{Digest, Sha1};
use tokio::io::AsyncWriteExt;

use super::{DownloadError, DownloadEvent, ProgressSink};

const MAX_ATTEMPTS: u32 = 3;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub url: String,
    pub dest: PathBuf,
    /// Expected SHA1 hex digest, when the source provides one (Mojang
    /// always does for client/library/asset downloads).
    pub expected_sha1: Option<String>,
    /// Human-readable name for progress UI - a filename, not the full URL.
    pub label: String,
}

/// Hex-encodes a SHA1 digest the same way Mojang's manifests do (lowercase).
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn sha1_hex_of_file(path: &std::path::Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha1::new();
    hasher.update(&bytes);
    Ok(hex_encode(&hasher.finalize()))
}

/// `true` if `dest` already exists and (when a hash is known) matches it -
/// the file doesn't need downloading again.
pub fn already_satisfied(task: &DownloadTask) -> bool {
    if !task.dest.is_file() {
        return false;
    }
    match &task.expected_sha1 {
        Some(expected) => sha1_hex_of_file(&task.dest)
            .map(|actual| &actual == expected)
            .unwrap_or(false),
        None => true,
    }
}

/// Downloads one file with up to `MAX_ATTEMPTS` tries, reporting progress
/// on `events`. Never leaves a partial/corrupt file at `task.dest` - all
/// writes go to a temp file first, verified, then renamed into place.
pub async fn download_one(
    client: &reqwest::Client,
    task: &DownloadTask,
    events: &ProgressSink,
) -> Result<(), DownloadError> {
    if already_satisfied(task) {
        events.send(DownloadEvent::FileCompleted {
            label: task.label.clone(),
            cached: true,
        });
        return Ok(());
    }

    let mut last_error = None;
    for attempt in 1..=MAX_ATTEMPTS {
        match try_download_once(client, task, events).await {
            Ok(()) => {
                events.send(DownloadEvent::FileCompleted {
                    label: task.label.clone(),
                    cached: false,
                });
                return Ok(());
            }
            Err(err) => {
                last_error = Some(err);
                if attempt < MAX_ATTEMPTS {
                    let backoff = Duration::from_millis(300 * 2u64.pow(attempt - 1));
                    tokio::time::sleep(backoff).await;
                }
            }
        }
    }

    let error = last_error
        .unwrap_or_else(|| DownloadError::Other("download failed for an unknown reason".into()));
    events.send(DownloadEvent::FileFailed {
        label: task.label.clone(),
        error: error.to_string(),
    });
    Err(error)
}

async fn try_download_once(
    client: &reqwest::Client,
    task: &DownloadTask,
    events: &ProgressSink,
) -> Result<(), DownloadError> {
    if let Some(parent) = task.dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let response = client
        .get(&task.url)
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|err| DownloadError::Network(err.to_string()))?;

    if !response.status().is_success() {
        return Err(DownloadError::Network(format!(
            "HTTP {} for {}",
            response.status(),
            task.url
        )));
    }

    let total_bytes = response.content_length();
    let mut tmp_name = task.dest.as_os_str().to_os_string();
    tmp_name.push(".part");
    let tmp_path = PathBuf::from(tmp_name);

    let mut file = tokio::fs::File::create(&tmp_path).await?;
    let mut hasher = Sha1::new();
    let mut downloaded: u64 = 0;

    use futures_util::StreamExt;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|err| DownloadError::Network(err.to_string()))?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        events.send(DownloadEvent::FileProgress {
            label: task.label.clone(),
            bytes_downloaded: downloaded,
            total_bytes,
        });
    }
    file.flush().await?;
    drop(file);

    if let Some(expected) = &task.expected_sha1 {
        let actual = hex_encode(&hasher.finalize());
        if &actual != expected {
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return Err(DownloadError::HashMismatch {
                file: task.label.clone(),
                expected: expected.clone(),
                actual,
            });
        }
    }

    tokio::fs::rename(&tmp_path, &task.dest).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_encode_matches_known_sha1_of_empty_input() {
        let mut hasher = Sha1::new();
        hasher.update(b"");
        assert_eq!(
            hex_encode(&hasher.finalize()),
            "da39a3ee5e6b4b0d3255bfef95601890afd80709"
        );
    }

    #[test]
    fn already_satisfied_is_false_when_the_file_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let task = DownloadTask {
            url: "http://example/x".into(),
            dest: dir.path().join("missing.bin"),
            expected_sha1: None,
            label: "x".into(),
        };
        assert!(!already_satisfied(&task));
    }

    #[test]
    fn already_satisfied_is_true_without_a_hash_when_the_file_merely_exists() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("present.bin");
        std::fs::write(&dest, b"anything").unwrap();
        let task = DownloadTask {
            url: "http://example/x".into(),
            dest,
            expected_sha1: None,
            label: "x".into(),
        };
        assert!(already_satisfied(&task));
    }

    #[test]
    fn already_satisfied_checks_the_hash_when_one_is_known() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("present.bin");
        std::fs::write(&dest, b"hello world").unwrap();

        let mut hasher = Sha1::new();
        hasher.update(b"hello world");
        let correct_hash = hex_encode(&hasher.finalize());

        let good = DownloadTask {
            url: "http://example/x".into(),
            dest: dest.clone(),
            expected_sha1: Some(correct_hash),
            label: "x".into(),
        };
        assert!(already_satisfied(&good));

        let bad = DownloadTask {
            url: "http://example/x".into(),
            dest,
            expected_sha1: Some("0000000000000000000000000000000000000".into()),
            label: "x".into(),
        };
        assert!(!already_satisfied(&bad));
    }
}
