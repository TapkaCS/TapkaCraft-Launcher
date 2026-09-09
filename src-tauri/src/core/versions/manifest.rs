//! Fetches and caches Mojang's version manifest - the index of every
//! release/snapshot and where to find each one's full metadata JSON.

use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::VersionError;

pub const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const MANIFEST_CACHE_FILE: &str = "version_manifest_v2.json";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionManifest {
    pub latest: LatestVersions,
    pub versions: Vec<VersionManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestVersions {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionManifestEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: VersionKind,
    pub url: String,
    pub sha1: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VersionKind {
    Release,
    Snapshot,
    OldBeta,
    OldAlpha,
}

impl VersionManifest {
    pub fn find(&self, id: &str) -> Option<&VersionManifestEntry> {
        self.versions.iter().find(|v| v.id == id)
    }

    /// Release versions only, newest first (the manifest is already sorted
    /// this way, but we don't want to depend on that silently).
    pub fn releases(&self) -> Vec<&VersionManifestEntry> {
        let mut releases: Vec<&VersionManifestEntry> = self
            .versions
            .iter()
            .filter(|v| v.kind == VersionKind::Release)
            .collect();
        releases.sort_by(|a, b| b.release_time.cmp(&a.release_time));
        releases
    }
}

/// Fetches the manifest from `url` (production callers pass `MANIFEST_URL`;
/// tests point this at a local mock server instead) and writes it to
/// `cache_dir` on success. Network errors are reported via
/// `VersionError::Network` with the underlying message intact - never a
/// bare "something went wrong".
pub async fn fetch_manifest(
    client: &reqwest::Client,
    url: &str,
    cache_dir: &Path,
) -> Result<VersionManifest, VersionError> {
    let response = client
        .get(url)
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|err| VersionError::Network(err.to_string()))?;

    if !response.status().is_success() {
        return Err(VersionError::Network(format!(
            "Mojang version manifest request failed with HTTP {}",
            response.status()
        )));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|err| VersionError::Network(err.to_string()))?;

    let manifest: VersionManifest =
        serde_json::from_slice(&bytes).map_err(|err| VersionError::Parse(err.to_string()))?;

    let _ = std::fs::create_dir_all(cache_dir);
    let _ = std::fs::write(cache_dir.join(MANIFEST_CACHE_FILE), &bytes);

    Ok(manifest)
}

/// Reads the last successfully fetched manifest from disk, for offline
/// startup or when Mojang is briefly unreachable. `None` if nothing has
/// ever been cached.
pub fn read_cached_manifest(cache_dir: &Path) -> Option<VersionManifest> {
    let bytes = std::fs::read(cache_dir.join(MANIFEST_CACHE_FILE)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Fetches the manifest, falling back to the last successfully cached copy
/// if the network request fails (offline, Mojang briefly unreachable, this
/// sandbox's egress policy, ...). Errors only when neither succeeds.
pub async fn fetch_or_cached(
    client: &reqwest::Client,
    url: &str,
    cache_dir: &Path,
) -> Result<VersionManifest, VersionError> {
    match fetch_manifest(client, url, cache_dir).await {
        Ok(manifest) => Ok(manifest),
        Err(err) => read_cached_manifest(cache_dir).ok_or(err),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::test_support::spawn_mock_server;

    fn sample_manifest_json() -> &'static str {
        r#"{
            "latest": { "release": "1.21.1", "snapshot": "24w45a" },
            "versions": [
                { "id": "24w45a", "type": "snapshot", "url": "https://example/24w45a.json", "sha1": "aaa", "releaseTime": "2024-11-10T00:00:00+00:00" },
                { "id": "1.21.1", "type": "release", "url": "https://example/1.21.1.json", "sha1": "bbb", "releaseTime": "2024-08-08T00:00:00+00:00" },
                { "id": "1.21", "type": "release", "url": "https://example/1.21.json", "sha1": "ccc", "releaseTime": "2024-06-13T00:00:00+00:00" },
                { "id": "1.8.9", "type": "release", "url": "https://example/1.8.9.json", "sha1": "ddd", "releaseTime": "2015-12-09T00:00:00+00:00" }
            ]
        }"#
    }

    #[test]
    fn parses_the_real_manifest_shape() {
        let manifest: VersionManifest = serde_json::from_str(sample_manifest_json()).unwrap();
        assert_eq!(manifest.latest.release, "1.21.1");
        assert_eq!(manifest.versions.len(), 4);
    }

    #[test]
    fn find_looks_up_by_id() {
        let manifest: VersionManifest = serde_json::from_str(sample_manifest_json()).unwrap();
        assert_eq!(manifest.find("1.21").unwrap().sha1, "ccc");
        assert!(manifest.find("nonexistent").is_none());
    }

    #[test]
    fn releases_excludes_snapshots_and_sorts_newest_first() {
        let manifest: VersionManifest = serde_json::from_str(sample_manifest_json()).unwrap();
        let releases = manifest.releases();
        let ids: Vec<&str> = releases.iter().map(|v| v.id.as_str()).collect();
        assert_eq!(ids, vec!["1.21.1", "1.21", "1.8.9"]);
    }

    #[test]
    fn cache_round_trips_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let manifest: VersionManifest = serde_json::from_str(sample_manifest_json()).unwrap();
        std::fs::write(
            dir.path().join(MANIFEST_CACHE_FILE),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();

        let cached = read_cached_manifest(dir.path()).unwrap();
        assert_eq!(cached.latest.release, manifest.latest.release);
    }

    #[test]
    fn missing_cache_returns_none_instead_of_panicking() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_cached_manifest(dir.path()).is_none());
    }

    #[tokio::test]
    async fn fetch_manifest_fetches_and_caches_against_a_real_local_server() {
        let mut files = HashMap::new();
        files.insert("/manifest.json".to_string(), sample_manifest_json().into());
        let base = spawn_mock_server(files).await;

        let dir = tempfile::tempdir().unwrap();
        let client = reqwest::Client::new();
        let manifest = fetch_manifest(&client, &format!("{base}/manifest.json"), dir.path())
            .await
            .unwrap();

        assert_eq!(manifest.latest.release, "1.21.1");
        // Written to the cache as a side effect of a successful fetch.
        assert!(read_cached_manifest(dir.path()).is_some());
    }

    #[tokio::test]
    async fn fetch_manifest_reports_a_typed_network_error_on_http_failure_status() {
        let base = spawn_mock_server(HashMap::new()).await; // nothing registered -> 404 for any path
        let dir = tempfile::tempdir().unwrap();
        let client = reqwest::Client::new();
        let err = fetch_manifest(&client, &format!("{base}/missing.json"), dir.path())
            .await
            .unwrap_err();
        assert!(matches!(err, VersionError::Network(_)));
    }

    #[tokio::test]
    async fn fetch_or_cached_falls_back_to_the_cache_when_the_request_fails() {
        let dir = tempfile::tempdir().unwrap();
        let manifest: VersionManifest = serde_json::from_str(sample_manifest_json()).unwrap();
        std::fs::write(
            dir.path().join(MANIFEST_CACHE_FILE),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();

        // A URL nothing is listening on - `fetch_manifest` must fail here so
        // the fallback path actually runs, not just compile.
        let client = reqwest::Client::new();
        let result = fetch_or_cached(&client, "http://127.0.0.1:1/unreachable", dir.path())
            .await
            .unwrap();
        assert_eq!(result.latest.release, manifest.latest.release);
    }

    #[tokio::test]
    async fn fetch_or_cached_prefers_a_live_fetch_over_a_stale_cache() {
        let mut files = HashMap::new();
        files.insert("/manifest.json".to_string(), sample_manifest_json().into());
        let base = spawn_mock_server(files).await;

        let dir = tempfile::tempdir().unwrap();
        // A stale cache with a different "latest" than the live server's -
        // the live value must win.
        let stale = VersionManifest {
            latest: LatestVersions {
                release: "0.0.0-stale".into(),
                snapshot: "0.0.0-stale".into(),
            },
            versions: vec![],
        };
        std::fs::write(
            dir.path().join(MANIFEST_CACHE_FILE),
            serde_json::to_vec(&stale).unwrap(),
        )
        .unwrap();

        let client = reqwest::Client::new();
        let result = fetch_or_cached(&client, &format!("{base}/manifest.json"), dir.path())
            .await
            .unwrap();
        assert_eq!(result.latest.release, "1.21.1");
    }

    #[tokio::test]
    async fn fetch_or_cached_reports_the_network_error_when_there_is_no_cache_either() {
        let dir = tempfile::tempdir().unwrap();
        let client = reqwest::Client::new();
        let err = fetch_or_cached(&client, "http://127.0.0.1:1/unreachable", dir.path())
            .await
            .unwrap_err();
        assert!(matches!(err, VersionError::Network(_)));
    }
}
