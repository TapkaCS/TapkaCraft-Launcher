//! Concurrency-limited orchestration over many individual `download_one`
//! calls.

use std::sync::Arc;

use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use super::task::{download_one, DownloadTask};
use super::{DownloadEvent, ProgressSink};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownloadSummary {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
}

impl DownloadSummary {
    pub fn all_succeeded(&self) -> bool {
        self.failed == 0
    }
}

/// Downloads every task, running at most `concurrency` at once. Never
/// aborts early on a single failure - every task gets attempted (with its
/// own internal retries), and the summary reports how many of each.
pub async fn download_all(
    client: &reqwest::Client,
    tasks: Vec<DownloadTask>,
    concurrency: usize,
    events: ProgressSink,
) -> DownloadSummary {
    let total = tasks.len();
    events.send(DownloadEvent::Started { total_files: total });

    let semaphore = Arc::new(Semaphore::new(concurrency.max(1)));
    let mut join_set = JoinSet::new();

    for task in tasks {
        let semaphore = semaphore.clone();
        let client = client.clone();
        let events = events.clone();
        join_set.spawn(async move {
            let _permit = semaphore
                .acquire_owned()
                .await
                .expect("download semaphore was closed early");
            download_one(&client, &task, &events).await
        });
    }

    let mut succeeded = 0;
    let mut failed = 0;
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(())) => succeeded += 1,
            // A panicking download task (Err from join) is still just one
            // failed file, not a reason to crash the whole batch.
            Ok(Err(_)) | Err(_) => failed += 1,
        }
    }

    events.send(DownloadEvent::Finished { succeeded, failed });
    DownloadSummary {
        total,
        succeeded,
        failed,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::*;

    /// A minimal HTTP/1.1 server for tests: serves fixed byte content per
    /// path from a lookup table, 404s anything else, and keeps running
    /// (one task per connection) until the test's temp resources drop.
    /// Lets DownloadManager be verified for real - concurrency, hashing,
    /// atomic writes - without depending on reaching Mojang's actual CDN
    /// from this sandbox (which the network policy here blocks).
    async fn spawn_mock_server(files: HashMap<&'static str, Vec<u8>>) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let files = Arc::new(files);

        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                let files = files.clone();
                tokio::spawn(async move {
                    let mut buf = [0u8; 4096];
                    let Ok(n) = socket.read(&mut buf).await else {
                        return;
                    };
                    let request = String::from_utf8_lossy(&buf[..n]);
                    let path = request.split_whitespace().nth(1).unwrap_or("/").to_string();

                    match files.get(path.as_str()) {
                        Some(content) => {
                            let header = format!(
                                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                content.len()
                            );
                            let _ = socket.write_all(header.as_bytes()).await;
                            let _ = socket.write_all(content).await;
                        }
                        None => {
                            let _ = socket
                                .write_all(b"HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n")
                                .await;
                        }
                    }
                });
            }
        });

        addr
    }

    fn sha1_hex(data: &[u8]) -> String {
        use sha1::{Digest, Sha1};
        let mut hasher = Sha1::new();
        hasher.update(data);
        hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    #[tokio::test]
    async fn downloads_multiple_files_concurrently_and_verifies_hashes() {
        let file_a = b"hello from file a".to_vec();
        let file_b = b"a rather different second file".to_vec();
        let mut files = HashMap::new();
        files.insert("/a.txt", file_a.clone());
        files.insert("/b.txt", file_b.clone());
        let addr = spawn_mock_server(files).await;

        let dir = tempfile::tempdir().unwrap();
        let tasks = vec![
            DownloadTask {
                url: format!("http://{addr}/a.txt"),
                dest: dir.path().join("a.txt"),
                expected_sha1: Some(sha1_hex(&file_a)),
                label: "a.txt".into(),
            },
            DownloadTask {
                url: format!("http://{addr}/b.txt"),
                dest: dir.path().join("b.txt"),
                expected_sha1: Some(sha1_hex(&file_b)),
                label: "b.txt".into(),
            },
        ];

        let client = reqwest::Client::new();
        let summary = download_all(&client, tasks, 4, ProgressSink::none()).await;

        assert!(summary.all_succeeded());
        assert_eq!(summary.total, 2);
        assert_eq!(std::fs::read(dir.path().join("a.txt")).unwrap(), file_a);
        assert_eq!(std::fs::read(dir.path().join("b.txt")).unwrap(), file_b);
    }

    #[tokio::test]
    async fn a_hash_mismatch_fails_that_file_without_writing_it_to_the_destination() {
        let mut files = HashMap::new();
        files.insert("/corrupt.bin", b"actual content".to_vec());
        let addr = spawn_mock_server(files).await;

        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("corrupt.bin");
        let tasks = vec![DownloadTask {
            url: format!("http://{addr}/corrupt.bin"),
            dest: dest.clone(),
            expected_sha1: Some("0000000000000000000000000000000000000".into()),
            label: "corrupt.bin".into(),
        }];

        let client = reqwest::Client::new();
        let summary = download_all(&client, tasks, 1, ProgressSink::none()).await;

        assert_eq!(summary.failed, 1);
        assert!(
            !dest.exists(),
            "a hash-mismatched file must never be left at the destination path"
        );
    }

    #[tokio::test]
    async fn a_404_is_reported_as_a_failure_not_a_panic() {
        let addr = spawn_mock_server(HashMap::new()).await;
        let dir = tempfile::tempdir().unwrap();
        let tasks = vec![DownloadTask {
            url: format!("http://{addr}/does-not-exist.bin"),
            dest: dir.path().join("does-not-exist.bin"),
            expected_sha1: None,
            label: "does-not-exist.bin".into(),
        }];

        let client = reqwest::Client::new();
        let summary = download_all(&client, tasks, 1, ProgressSink::none()).await;
        assert_eq!(summary.failed, 1);
    }

    #[tokio::test]
    async fn already_present_and_correctly_hashed_files_are_treated_as_cached() {
        let content = b"already have this one".to_vec();
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("cached.bin");
        std::fs::write(&dest, &content).unwrap();

        // Point at a server that would fail the test if actually hit -
        // proving the cache check short-circuits the network entirely.
        let requests = Arc::new(AtomicUsize::new(0));
        let counted_requests = requests.clone();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                counted_requests.fetch_add(1, Ordering::SeqCst);
                let _ = socket
                    .write_all(b"HTTP/1.1 500 Internal Server Error\r\n\r\n")
                    .await;
            }
        });

        let tasks = vec![DownloadTask {
            url: format!("http://{addr}/cached.bin"),
            dest: dest.clone(),
            expected_sha1: Some(sha1_hex(&content)),
            label: "cached.bin".into(),
        }];

        let client = reqwest::Client::new();
        let summary = download_all(&client, tasks, 1, ProgressSink::none()).await;

        assert!(summary.all_succeeded());
        assert_eq!(
            requests.load(Ordering::SeqCst),
            0,
            "must not touch the network for an already-valid file"
        );
    }
}
