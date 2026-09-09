//! Read-only calls against the public Modrinth API - no authentication of
//! any kind is required for search or metadata, unlike every other
//! external service this launcher talks to.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::ModrinthError;

pub const MODRINTH_API_URL: &str = "https://api.modrinth.com/v2";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Modrinth's JSON is snake_case throughout (unlike Mojang/Microsoft's
/// mixed-case APIs elsewhere in this codebase) - these field names match
/// it as written, no `rename_all` needed or wanted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub icon_url: Option<String>,
    pub downloads: u64,
    #[serde(default)]
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub hits: Vec<SearchHit>,
    pub total_hits: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthVersion {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<ModrinthDependency>,
    pub files: Vec<ModrinthFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthDependency {
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub version_id: Option<String>,
    pub dependency_type: String,
}

impl ModrinthDependency {
    pub fn is_required(&self) -> bool {
        self.dependency_type == "required"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthFile {
    pub url: String,
    pub filename: String,
    #[serde(default)]
    pub primary: bool,
    pub hashes: ModrinthHashes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthHashes {
    pub sha1: String,
}

async fn get_json<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
) -> Result<T, ModrinthError> {
    let response = client
        .get(url)
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|err| ModrinthError::Network(err.to_string()))?;

    if !response.status().is_success() {
        return Err(ModrinthError::Network(format!(
            "HTTP {} for {url}",
            response.status()
        )));
    }

    response
        .json()
        .await
        .map_err(|err| ModrinthError::Parse(err.to_string()))
}

fn build_url(base_url: &str, segments: &[&str]) -> Result<reqwest::Url, ModrinthError> {
    let mut url = reqwest::Url::parse(base_url)
        .map_err(|err| ModrinthError::Parse(format!("invalid Modrinth base URL: {err}")))?;
    {
        let mut path = url
            .path_segments_mut()
            .map_err(|_| ModrinthError::Parse("Modrinth base URL cannot be a base".to_string()))?;
        path.pop_if_empty();
        for segment in segments {
            path.push(segment);
        }
    }
    Ok(url)
}

/// Searches for mods compatible with `loader` and `game_version`. `query`
/// may be empty (an empty search still returns Modrinth's default-sorted
/// results, useful for "just show me something" browsing).
pub async fn search(
    client: &reqwest::Client,
    base_url: &str,
    query: &str,
    loader: &str,
    game_version: &str,
    limit: u32,
) -> Result<SearchResponse, ModrinthError> {
    let facets = serde_json::json!([
        ["project_type:mod"],
        [format!("categories:{loader}")],
        [format!("versions:{game_version}")],
    ]);
    let mut url = build_url(base_url, &["search"])?;
    url.query_pairs_mut()
        .append_pair("query", query)
        .append_pair("facets", &facets.to_string())
        .append_pair("limit", &limit.to_string());
    get_json(client, url.as_str()).await
}

/// Every version of `project_id` compatible with `loader` and
/// `game_version`, newest first (Modrinth's own default order).
pub async fn list_project_versions(
    client: &reqwest::Client,
    base_url: &str,
    project_id: &str,
    loader: &str,
    game_version: &str,
) -> Result<Vec<ModrinthVersion>, ModrinthError> {
    let mut url = build_url(base_url, &["project", project_id, "version"])?;
    url.query_pairs_mut()
        .append_pair("loaders", &serde_json::json!([loader]).to_string())
        .append_pair(
            "game_versions",
            &serde_json::json!([game_version]).to_string(),
        );
    get_json(client, url.as_str()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::spawn_mock_server;
    use std::collections::HashMap;

    fn sample_search_json() -> serde_json::Value {
        serde_json::json!({
            "hits": [
                {
                    "project_id": "AANobbMI",
                    "slug": "sodium",
                    "title": "Sodium",
                    "description": "A modern rendering engine",
                    "icon_url": "https://example/icon.png",
                    "downloads": 12345678,
                    "categories": ["optimization"]
                }
            ],
            "offset": 0,
            "limit": 20,
            "total_hits": 1
        })
    }

    #[tokio::test]
    async fn search_fetches_and_parses_a_real_shaped_response() {
        let mut files: HashMap<String, Vec<u8>> = HashMap::new();
        // The mock server matches path only (query string stripped), so
        // the real query-building logic still runs - and would 404 on a
        // wrong path - while the fixture only needs a path-keyed entry.
        files.insert(
            "/search".to_string(),
            serde_json::to_vec(&sample_search_json()).unwrap(),
        );
        let base = spawn_mock_server(files).await;

        let client = reqwest::Client::new();
        let result = search(&client, &base, "sodium", "fabric", "1.20.1", 20)
            .await
            .unwrap();
        assert_eq!(result.total_hits, 1);
        assert_eq!(result.hits[0].title, "Sodium");
        assert_eq!(result.hits[0].project_id, "AANobbMI");
    }

    #[tokio::test]
    async fn search_reports_a_network_error_on_http_failure() {
        let files: HashMap<String, Vec<u8>> = HashMap::new();
        let base = spawn_mock_server(files).await;

        let client = reqwest::Client::new();
        let result = search(&client, &base, "sodium", "fabric", "1.20.1", 20).await;
        assert!(matches!(result, Err(ModrinthError::Network(_))));
    }

    fn sample_versions_json() -> serde_json::Value {
        serde_json::json!([
            {
                "id": "ver1",
                "project_id": "AANobbMI",
                "name": "Sodium 0.5.8",
                "version_number": "0.5.8",
                "game_versions": ["1.20.1"],
                "loaders": ["fabric"],
                "dependencies": [
                    { "project_id": "P7dR8mSH", "version_id": null, "dependency_type": "required" },
                    { "project_id": "optionalDep", "version_id": null, "dependency_type": "optional" }
                ],
                "files": [
                    {
                        "url": "https://example/sodium-fabric-0.5.8.jar",
                        "filename": "sodium-fabric-0.5.8.jar",
                        "primary": true,
                        "hashes": { "sha1": "abc123" }
                    }
                ]
            }
        ])
    }

    #[tokio::test]
    async fn list_project_versions_fetches_and_parses_dependencies_and_files() {
        let mut files: HashMap<String, Vec<u8>> = HashMap::new();
        files.insert(
            "/project/AANobbMI/version".to_string(),
            serde_json::to_vec(&sample_versions_json()).unwrap(),
        );
        let base = spawn_mock_server(files).await;

        let client = reqwest::Client::new();
        let versions = list_project_versions(&client, &base, "AANobbMI", "fabric", "1.20.1")
            .await
            .unwrap();
        assert_eq!(versions.len(), 1);
        let version = &versions[0];
        assert_eq!(version.files[0].hashes.sha1, "abc123");
        assert!(version.dependencies[0].is_required());
        assert!(!version.dependencies[1].is_required());
    }
}
