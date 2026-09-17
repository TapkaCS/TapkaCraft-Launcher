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

/// Builds the `facets` query value: always `project_type` and
/// `game_version`, plus a `categories:<loader>` group only when `loader`
/// is given - omitted entirely for content (resource packs, shaders)
/// that isn't tied to a mod loader, since Modrinth's own facet AND's every
/// group together and a loader group real resource packs/shaders never
/// tag themselves with would just filter out everything. `categories`
/// (genre/theme tags like "adventure" or "kitchen-sink") are OR'd together
/// in their own group when given - deliberately never a hardcoded list
/// here: the caller only ever passes back a tag Modrinth itself already
/// returned on a real hit's `categories` field, so there's no risk of a
/// guessed/misspelled tag silently matching nothing.
fn build_search_facets(
    project_type: &str,
    loader: Option<&str>,
    game_version: &str,
    categories: &[String],
) -> String {
    let mut groups: Vec<Vec<String>> = vec![vec![format!("project_type:{project_type}")]];
    if let Some(loader) = loader {
        groups.push(vec![format!("categories:{loader}")]);
    }
    if !categories.is_empty() {
        groups.push(
            categories
                .iter()
                .map(|category| format!("categories:{category}"))
                .collect(),
        );
    }
    groups.push(vec![format!("versions:{game_version}")]);
    serde_json::to_string(&groups).expect("Vec<Vec<String>> always serializes")
}

/// Searches for projects of `project_type` (`"mod"`, `"resourcepack"`,
/// `"shader"` or `"modpack"`) compatible with `game_version`, and with
/// `loader` when one is given (see `ContentKind::uses_loader_facet`) and
/// `categories` when any are selected. `query` may be empty (an empty
/// search still returns Modrinth's default-sorted results, useful for
/// "just show me something" browsing).
pub async fn search(
    client: &reqwest::Client,
    base_url: &str,
    project_type: &str,
    query: &str,
    loader: Option<&str>,
    game_version: &str,
    categories: &[String],
    limit: u32,
) -> Result<SearchResponse, ModrinthError> {
    let facets = build_search_facets(project_type, loader, game_version, categories);
    let mut url = build_url(base_url, &["search"])?;
    url.query_pairs_mut()
        .append_pair("query", query)
        .append_pair("facets", &facets)
        .append_pair("limit", &limit.to_string());
    get_json(client, url.as_str()).await
}

/// Every version of `project_id` compatible with `game_version`, and with
/// `loader` when one is given, newest first (Modrinth's own default
/// order).
pub async fn list_project_versions(
    client: &reqwest::Client,
    base_url: &str,
    project_id: &str,
    loader: Option<&str>,
    game_version: &str,
) -> Result<Vec<ModrinthVersion>, ModrinthError> {
    let mut url = build_url(base_url, &["project", project_id, "version"])?;
    {
        let mut pairs = url.query_pairs_mut();
        if let Some(loader) = loader {
            pairs.append_pair("loaders", &serde_json::json!([loader]).to_string());
        }
        pairs.append_pair(
            "game_versions",
            &serde_json::json!([game_version]).to_string(),
        );
    }
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
        let result = search(&client, &base, "mod", "sodium", Some("fabric"), "1.20.1", &[], 20)
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
        let result =
            search(&client, &base, "mod", "sodium", Some("fabric"), "1.20.1", &[], 20).await;
        assert!(matches!(result, Err(ModrinthError::Network(_))));
    }

    #[test]
    fn build_search_facets_includes_the_loader_group_only_when_one_is_given() {
        assert_eq!(
            build_search_facets("mod", Some("fabric"), "1.20.1", &[]),
            r#"[["project_type:mod"],["categories:fabric"],["versions:1.20.1"]]"#
        );
        assert_eq!(
            build_search_facets("resourcepack", None, "1.20.1", &[]),
            r#"[["project_type:resourcepack"],["versions:1.20.1"]]"#
        );
    }

    #[test]
    fn build_search_facets_ors_every_selected_category_in_one_group() {
        let categories = vec!["adventure".to_string(), "kitchen-sink".to_string()];
        assert_eq!(
            build_search_facets("modpack", Some("fabric"), "1.20.1", &categories),
            r#"[["project_type:modpack"],["categories:fabric"],["categories:adventure","categories:kitchen-sink"],["versions:1.20.1"]]"#
        );
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
        let versions =
            list_project_versions(&client, &base, "AANobbMI", Some("fabric"), "1.20.1")
                .await
                .unwrap();
        assert_eq!(versions.len(), 1);
        let version = &versions[0];
        assert_eq!(version.files[0].hashes.sha1, "abc123");
        assert!(version.dependencies[0].is_required());
        assert!(!version.dependencies[1].is_required());
    }
}
