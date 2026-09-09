//! Fetches Mojang's Java runtime manifest - the index of every runtime
//! component (`java-runtime-gamma`, `jre-legacy`, ...) Mojang distributes,
//! per platform, and resolves the one a given component name needs for the
//! machine this is running on.

use std::collections::HashMap;
use std::time::Duration;

use serde::Deserialize;

use super::JavaInstallError;

/// Stable for years across every third-party launcher that relies on it
/// (PrismLauncher, HMCL, ...) - not a per-release URL, despite the
/// hash-shaped path segment.
pub const RUNTIME_MANIFEST_URL: &str =
    "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// `{ platform: { component: [ { manifest, version } ] } }`. Each component
/// maps to a one-element array in practice; a `Vec` (rather than assuming
/// exactly one) tolerates a platform Mojang ever ships more than one build
/// for without this failing to parse.
#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeManifest(pub HashMap<String, HashMap<String, Vec<RuntimeManifestEntry>>>);

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeManifestEntry {
    pub manifest: RuntimeFileRef,
}

/// A `{sha1, size, url}` reference shape reused for both the per-runtime
/// `manifest.json` pointer and, in `install.rs`, every individual file
/// inside it.
#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeFileRef {
    pub sha1: String,
    #[allow(dead_code)] // not needed to install - kept because Mojang sends it
    pub size: u64,
    pub url: String,
}

/// Mojang's platform key for the machine this is actually running on, or
/// `None` on a platform/arch combination Mojang doesn't publish a runtime
/// for (e.g. Linux on ARM) - callers report that honestly rather than
/// pretending an install is possible.
pub fn current_platform_key() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => Some("windows-x64"),
        ("windows", "x86") => Some("windows-x86"),
        ("windows", "aarch64") => Some("windows-arm64"),
        ("linux", "x86_64") => Some("linux"),
        ("linux", "x86") => Some("linux-i386"),
        ("macos", "x86_64") => Some("mac-os"),
        ("macos", "aarch64") => Some("mac-os-arm64"),
        _ => None,
    }
}

pub async fn fetch_runtime_manifest(
    client: &reqwest::Client,
    url: &str,
) -> Result<RuntimeManifest, JavaInstallError> {
    let response = client
        .get(url)
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|err| JavaInstallError::Network(err.to_string()))?;

    if !response.status().is_success() {
        return Err(JavaInstallError::Network(format!(
            "Java runtime manifest request failed with HTTP {}",
            response.status()
        )));
    }

    response
        .json()
        .await
        .map_err(|err| JavaInstallError::Parse(err.to_string()))
}

/// The single entry Mojang publishes for `component` on this machine's
/// platform - `None` (not an error) when the platform simply doesn't have
/// that component, which does happen (e.g. very old `jre-legacy` isn't
/// published for every architecture).
pub fn resolve_component<'a>(
    manifest: &'a RuntimeManifest,
    platform_key: &str,
    component: &str,
) -> Option<&'a RuntimeManifestEntry> {
    manifest
        .0
        .get(platform_key)
        .and_then(|components| components.get(component))
        .and_then(|entries| entries.first())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::spawn_mock_server;
    use std::collections::HashMap as StdHashMap;

    fn sample_manifest_json() -> serde_json::Value {
        serde_json::json!({
            "linux": {
                "java-runtime-gamma": [
                    {
                        "availability": { "group": 1, "progress": 100 },
                        "manifest": { "sha1": "abc123", "size": 42, "url": "https://example/manifest.json" },
                        "version": { "name": "21.0.5+11" }
                    }
                ],
                "jre-legacy": []
            },
            "windows-x64": {
                "java-runtime-gamma": [
                    {
                        "availability": { "group": 1, "progress": 100 },
                        "manifest": { "sha1": "def456", "size": 43, "url": "https://example/win-manifest.json" },
                        "version": { "name": "21.0.5+11" }
                    }
                ]
            }
        })
    }

    #[test]
    fn current_platform_key_recognizes_this_sandbox_s_own_platform() {
        // This sandbox is genuinely linux/x86_64 - a real, not hypothetical,
        // assertion about the environment the test actually runs in.
        if std::env::consts::OS == "linux" && std::env::consts::ARCH == "x86_64" {
            assert_eq!(current_platform_key(), Some("linux"));
        }
    }

    #[test]
    fn resolve_component_finds_the_entry_for_platform_and_component() {
        let manifest: RuntimeManifest = serde_json::from_value(sample_manifest_json()).unwrap();
        let entry = resolve_component(&manifest, "linux", "java-runtime-gamma").unwrap();
        assert_eq!(entry.manifest.url, "https://example/manifest.json");
    }

    #[test]
    fn resolve_component_is_none_for_an_empty_component_array() {
        let manifest: RuntimeManifest = serde_json::from_value(sample_manifest_json()).unwrap();
        assert!(resolve_component(&manifest, "linux", "jre-legacy").is_none());
    }

    #[test]
    fn resolve_component_is_none_for_an_unknown_platform_or_component() {
        let manifest: RuntimeManifest = serde_json::from_value(sample_manifest_json()).unwrap();
        assert!(resolve_component(&manifest, "mac-os", "java-runtime-gamma").is_none());
        assert!(resolve_component(&manifest, "linux", "java-runtime-nonexistent").is_none());
    }

    #[tokio::test]
    async fn fetch_runtime_manifest_fetches_and_parses_a_real_shaped_response() {
        let mut files: StdHashMap<String, Vec<u8>> = StdHashMap::new();
        files.insert(
            "/all.json".into(),
            serde_json::to_vec(&sample_manifest_json()).unwrap(),
        );
        let base = spawn_mock_server(files).await;

        let client = reqwest::Client::new();
        let manifest = fetch_runtime_manifest(&client, &format!("{base}/all.json"))
            .await
            .unwrap();
        let entry = resolve_component(&manifest, "windows-x64", "java-runtime-gamma").unwrap();
        assert_eq!(entry.manifest.sha1, "def456");
    }

    #[tokio::test]
    async fn fetch_runtime_manifest_reports_a_typed_network_error_on_http_failure_status() {
        let files: StdHashMap<String, Vec<u8>> = StdHashMap::new();
        let base = spawn_mock_server(files).await;

        let client = reqwest::Client::new();
        let result = fetch_runtime_manifest(&client, &format!("{base}/missing.json")).await;
        assert!(matches!(result, Err(JavaInstallError::Network(_))));
    }
}
