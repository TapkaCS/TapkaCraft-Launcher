//! Downloads a specific Java runtime component (as resolved by
//! `manifest.rs`) and lays it out on disk exactly as Mojang's own
//! per-runtime manifest describes: files (downloaded and hash-verified
//! through the same `DownloadManager` every other download in this
//! launcher goes through), directories, and symlinks. Recreates Mojang's
//! own JRE layout, executable bits included, so nothing downstream needs
//! to know these runtimes came from anywhere different than a normal
//! system Java install.

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use serde::Deserialize;

use super::detect;
use super::manifest::{
    current_platform_key, fetch_runtime_manifest, resolve_component, RuntimeFileRef,
};
use super::runtime::DetectedRuntime;
use super::JavaInstallError;
use crate::core::downloads::{download_all, DownloadTask, ProgressSink};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeFilesManifest {
    pub files: HashMap<String, RuntimeFileEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum RuntimeFileEntry {
    File {
        #[serde(default)]
        executable: bool,
        downloads: RuntimeFileDownloads,
    },
    Directory,
    Link {
        target: String,
    },
}

/// Only `raw` is used - `lzma` is an optional smaller alternative the
/// official launcher decompresses client-side; downloading `raw` directly
/// gets the same bytes without needing an LZMA decoder in this codebase.
#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeFileDownloads {
    pub raw: RuntimeFileRef,
}

pub async fn fetch_files_manifest(
    client: &reqwest::Client,
    url: &str,
) -> Result<RuntimeFilesManifest, JavaInstallError> {
    let response = client
        .get(url)
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|err| JavaInstallError::Network(err.to_string()))?;

    if !response.status().is_success() {
        return Err(JavaInstallError::Network(format!(
            "Java runtime file manifest request failed with HTTP {}",
            response.status()
        )));
    }

    response
        .json()
        .await
        .map_err(|err| JavaInstallError::Parse(err.to_string()))
}

fn java_executable_name() -> &'static str {
    if cfg!(windows) {
        "java.exe"
    } else {
        "java"
    }
}

/// Downloads and lays out the runtime `component` needs into
/// `runtimes_dir/java{required_major}/`, then verifies the result by
/// actually running the resulting `java -version` - never trusting a
/// completed download alone. Idempotent: files that already exist with the
/// right hash are skipped by `DownloadManager`, so retrying after a
/// partial failure just resumes.
///
/// `runtime_manifest_url` is explicit (production callers pass
/// `manifest::RUNTIME_MANIFEST_URL`) so tests can point this whole function
/// at a local mock server instead of the real Mojang endpoint, the same
/// pattern every other Mojang-facing fetch in this codebase uses.
pub async fn install_runtime(
    client: &reqwest::Client,
    runtime_manifest_url: &str,
    runtimes_dir: &Path,
    component: &str,
    required_major: u32,
    concurrency: usize,
    events: ProgressSink,
) -> Result<DetectedRuntime, JavaInstallError> {
    let platform_key = current_platform_key().ok_or(JavaInstallError::UnsupportedPlatform)?;

    let top_manifest = fetch_runtime_manifest(client, runtime_manifest_url).await?;
    let entry = resolve_component(&top_manifest, platform_key, component).ok_or_else(|| {
        JavaInstallError::ComponentUnavailable {
            component: component.to_string(),
        }
    })?;
    let files_manifest = fetch_files_manifest(client, &entry.manifest.url).await?;

    let install_dir = runtimes_dir.join(format!("java{required_major}"));

    let mut tasks = Vec::new();
    let mut executables = Vec::new();
    let mut symlinks = Vec::new();

    for (rel_path, file_entry) in &files_manifest.files {
        let dest = install_dir.join(rel_path);
        match file_entry {
            RuntimeFileEntry::Directory => std::fs::create_dir_all(&dest)?,
            RuntimeFileEntry::File {
                executable,
                downloads,
            } => {
                tasks.push(DownloadTask {
                    url: downloads.raw.url.clone(),
                    dest: dest.clone(),
                    expected_sha1: Some(downloads.raw.sha1.clone()),
                    label: rel_path.clone(),
                });
                if *executable {
                    executables.push(dest);
                }
            }
            RuntimeFileEntry::Link { target } => symlinks.push((dest, target.clone())),
        }
    }

    let summary = download_all(client, tasks, concurrency.max(1), events).await;
    if summary.failed > 0 {
        return Err(JavaInstallError::DownloadFailed {
            failed: summary.failed,
            total: summary.total,
        });
    }

    for path in &executables {
        set_executable(path)?;
    }
    for (link, target) in &symlinks {
        create_symlink(link, target)?;
    }

    let java_path = install_dir.join("bin").join(java_executable_name());
    detect::verify(&java_path).ok_or(JavaInstallError::VerificationFailed { path: java_path })
}

#[cfg(unix)]
fn set_executable(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(perms.mode() | 0o755);
    std::fs::set_permissions(path, perms)
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> std::io::Result<()> {
    // Windows has no POSIX executable bit - runnability comes from the
    // file extension (.exe) alone, which Mojang's own filenames carry.
    Ok(())
}

#[cfg(unix)]
fn create_symlink(link: &Path, target: &str) -> std::io::Result<()> {
    if let Some(parent) = link.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // A previous partial/interrupted install may have left a stale file
    // here; a fresh symlink can't be created on top of one.
    let _ = std::fs::remove_file(link);
    std::os::unix::fs::symlink(target, link)
}

#[cfg(not(unix))]
fn create_symlink(_link: &Path, _target: &str) -> std::io::Result<()> {
    // Mojang's Windows runtime bundles are not expected to contain "link"
    // entries (Windows has no equivalent of the relative symlinks these
    // represent) - fails loudly rather than silently if that ever changes.
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "this platform's Java runtime archives are not expected to contain symlinks",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{bind_mock_server, serve_mock_files};
    use std::collections::HashMap as StdHashMap;

    #[test]
    fn parses_a_real_shaped_files_manifest() {
        let json = serde_json::json!({
            "files": {
                "bin/java": {
                    "type": "file",
                    "executable": true,
                    "downloads": {
                        "raw": { "sha1": "aaa", "size": 10, "url": "https://example/java" },
                        "lzma": { "sha1": "bbb", "size": 5, "url": "https://example/java.lzma" }
                    }
                },
                "lib": { "type": "directory" },
                "jre.bundle/Contents/Home": { "type": "link", "target": "../../.." }
            }
        });
        let manifest: RuntimeFilesManifest = serde_json::from_value(json).unwrap();
        assert_eq!(manifest.files.len(), 3);
        assert!(matches!(manifest.files["lib"], RuntimeFileEntry::Directory));
        match &manifest.files["bin/java"] {
            RuntimeFileEntry::File {
                executable,
                downloads,
            } => {
                assert!(*executable);
                assert_eq!(downloads.raw.sha1, "aaa");
            }
            other => panic!("expected a File entry, got {other:?}"),
        }
        match &manifest.files["jre.bundle/Contents/Home"] {
            RuntimeFileEntry::Link { target } => assert_eq!(target, "../../.."),
            other => panic!("expected a Link entry, got {other:?}"),
        }
    }

    fn sha1_hex(bytes: &[u8]) -> String {
        use sha1::{Digest, Sha1};
        let mut hasher = Sha1::new();
        hasher.update(bytes);
        hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    /// Bakes a two-level Mojang-shaped mock (top-level `all.json` pointing
    /// at a per-runtime `manifest.json` pointing at real file bytes) and
    /// returns the top-level manifest URL - three separate listeners bound
    /// up front (same reason `core::versions::install`'s tests do this: a
    /// URL can't be baked into an earlier server's payload before the
    /// later server it names is actually listening).
    async fn spawn_runtime_mock(component: &str, java_file_bytes: &[u8]) -> String {
        let (java_listener, java_addr) = bind_mock_server().await;
        let (fm_listener, fm_addr) = bind_mock_server().await;
        let (top_listener, top_addr) = bind_mock_server().await;

        let mut java_files: StdHashMap<String, Vec<u8>> = StdHashMap::new();
        java_files.insert("/java".into(), java_file_bytes.to_vec());
        serve_mock_files(java_listener, java_files);

        let files_manifest_json = serde_json::json!({
            "files": {
                "bin/java": {
                    "type": "file",
                    "executable": true,
                    "downloads": {
                        "raw": {
                            "sha1": sha1_hex(java_file_bytes),
                            "size": java_file_bytes.len(),
                            "url": format!("http://{java_addr}/java"),
                        }
                    }
                },
                "lib": { "type": "directory" }
            }
        });
        let mut fm_files: StdHashMap<String, Vec<u8>> = StdHashMap::new();
        fm_files.insert(
            "/manifest.json".into(),
            serde_json::to_vec(&files_manifest_json).unwrap(),
        );
        serve_mock_files(fm_listener, fm_files);

        let top_json = serde_json::json!({
            "linux": {
                component: [{
                    "availability": { "group": 1, "progress": 100 },
                    "manifest": {
                        "sha1": "unused",
                        "size": 1,
                        "url": format!("http://{fm_addr}/manifest.json"),
                    },
                    "version": { "name": "25.0.1+9" }
                }]
            }
        });
        let mut top_files: StdHashMap<String, Vec<u8>> = StdHashMap::new();
        top_files.insert("/all.json".into(), serde_json::to_vec(&top_json).unwrap());
        serve_mock_files(top_listener, top_files);

        format!("http://{top_addr}/all.json")
    }

    /// The full real path through `install_runtime` itself - manifest
    /// fetch, per-runtime manifest fetch, download, directory creation,
    /// executable bit, and the final `detect::verify` - using a fake
    /// "java" that's actually a shell script printing realistic
    /// `-version` output, so `detect::verify` genuinely executes it rather
    /// than this test only checking that bytes landed on disk. Unix-only:
    /// the executable bit and shebang-script trick are POSIX-specific.
    #[cfg(unix)]
    #[tokio::test]
    async fn install_runtime_end_to_end_installs_and_verifies_a_working_runtime() {
        assert_eq!(current_platform_key(), Some("linux"));

        let fake_java_script = b"#!/bin/sh\n\
            echo 'openjdk version \"25.0.1\" 2026-01-20' 1>&2\n\
            echo 'OpenJDK Runtime Environment Temurin-25.0.1+9' 1>&2\n\
            echo 'OpenJDK 64-Bit Server VM Temurin-25.0.1+9 (mixed mode)' 1>&2\n"
            .to_vec();
        let manifest_url = spawn_runtime_mock("java-runtime-gamma", &fake_java_script).await;

        let client = reqwest::Client::new();
        let dir = tempfile::tempdir().unwrap();

        let detected = install_runtime(
            &client,
            &manifest_url,
            dir.path(),
            "java-runtime-gamma",
            25,
            4,
            ProgressSink::none(),
        )
        .await
        .unwrap();

        assert_eq!(detected.major_version, 25);
        assert_eq!(detected.version_string, "25.0.1");
        assert!(detected.is_64_bit);

        let install_dir = dir.path().join("java25");
        assert!(install_dir.join("lib").is_dir());
        let java_bin = install_dir.join("bin/java");
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&java_bin).unwrap().permissions().mode();
        assert_eq!(mode & 0o111, 0o111, "java binary should be executable");

        // Idempotent: installing again (e.g. after an interrupted first
        // run) succeeds and doesn't re-download an already-correct file.
        let second = install_runtime(
            &client,
            &manifest_url,
            dir.path(),
            "java-runtime-gamma",
            25,
            4,
            ProgressSink::none(),
        )
        .await
        .unwrap();
        assert_eq!(second.major_version, 25);
    }

    #[tokio::test]
    async fn install_runtime_reports_component_unavailable_for_a_platform_the_manifest_lacks() {
        let manifest_url = spawn_runtime_mock("java-runtime-gamma", b"#!/bin/sh\n").await;
        let client = reqwest::Client::new();
        let dir = tempfile::tempdir().unwrap();

        let result = install_runtime(
            &client,
            &manifest_url,
            dir.path(),
            "jre-legacy", // not present in this mock's manifest
            8,
            4,
            ProgressSink::none(),
        )
        .await;
        assert!(matches!(
            result,
            Err(JavaInstallError::ComponentUnavailable { component }) if component == "jre-legacy"
        ));
    }

    #[tokio::test]
    async fn install_runtime_reports_a_network_error_for_an_unreachable_manifest() {
        let client = reqwest::Client::new();
        let dir = tempfile::tempdir().unwrap();
        let (listener, addr) = bind_mock_server().await;
        drop(listener); // bound then immediately dropped: nothing is listening

        let result = install_runtime(
            &client,
            &format!("http://{addr}/all.json"),
            dir.path(),
            "java-runtime-gamma",
            21,
            4,
            ProgressSink::none(),
        )
        .await;
        assert!(matches!(result, Err(JavaInstallError::Network(_))));
    }

    #[tokio::test]
    async fn spawn_runtime_mock_produces_a_manifest_fetch_all_json_can_parse() {
        // Sanity check on the test fixture itself, independent of
        // `install_runtime`: the real `fetch_runtime_manifest` +
        // `resolve_component` pair (exercised directly, not through
        // `install_runtime`) can find the component this helper bakes in.
        let manifest_url = spawn_runtime_mock("java-runtime-gamma", b"#!/bin/sh\n").await;
        let client = reqwest::Client::new();
        let manifest = fetch_runtime_manifest(&client, &manifest_url)
            .await
            .unwrap();
        assert!(resolve_component(&manifest, "linux", "java-runtime-gamma").is_some());
    }
}
