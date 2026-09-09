//! Client for Fabric's meta API (`meta.fabricmc.net`): the list of loader
//! builds available for a Minecraft version, and the "profile" JSON
//! (mainClass, extra JVM/game arguments, extra libraries) that gets layered
//! on top of the Vanilla version it `inheritsFrom` - the same document
//! Fabric's own installer writes into `.minecraft/versions/`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;

use super::LoaderInstallError;
use crate::core::downloads::{download_all, DownloadTask, ProgressSink};

pub const FABRIC_META_URL: &str = "https://meta.fabricmc.net";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Deserialize)]
struct LoaderVersionsEntry {
    loader: FabricLoaderBuild,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FabricLoaderBuild {
    pub version: String,
    pub stable: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FabricProfile {
    #[allow(dead_code)] // not needed to launch - kept because Fabric sends it
    pub id: String,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    #[serde(default)]
    pub arguments: FabricArguments,
    pub libraries: Vec<FabricLibrary>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct FabricArguments {
    #[serde(default)]
    pub game: Vec<String>,
    #[serde(default)]
    pub jvm: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FabricLibrary {
    pub name: String,
    pub url: String,
}

impl FabricLibrary {
    /// This library's path under a standard Maven repository layout,
    /// derived from its `group:artifact:version[:classifier]` coordinate -
    /// the same scheme Mojang's own libraries use, just without a hash to
    /// verify against (Fabric's meta API doesn't publish one for these).
    pub fn maven_path(&self) -> PathBuf {
        let mut parts = self.name.splitn(4, ':');
        let group = parts.next().unwrap_or_default();
        let artifact = parts.next().unwrap_or_default();
        let version = parts.next().unwrap_or_default();
        let classifier = parts.next();

        let mut file_name = format!("{artifact}-{version}");
        if let Some(classifier) = classifier {
            file_name.push('-');
            file_name.push_str(classifier);
        }
        file_name.push_str(".jar");

        let mut path = PathBuf::new();
        for segment in group.split('.') {
            path.push(segment);
        }
        path.push(artifact);
        path.push(version);
        path.push(file_name);
        path
    }

    pub fn download_url(&self) -> String {
        let base = if self.url.ends_with('/') {
            self.url.clone()
        } else {
            format!("{}/", self.url)
        };
        format!(
            "{base}{}",
            self.maven_path().to_string_lossy().replace('\\', "/")
        )
    }
}

fn build_url(base_url: &str, segments: &[&str]) -> Result<String, LoaderInstallError> {
    let mut url = reqwest::Url::parse(base_url)
        .map_err(|err| LoaderInstallError::Parse(format!("invalid loader meta URL: {err}")))?;
    {
        let mut path = url.path_segments_mut().map_err(|_| {
            LoaderInstallError::Parse("loader meta URL cannot be a base".to_string())
        })?;
        path.pop_if_empty();
        for segment in segments {
            path.push(segment);
        }
    }
    Ok(url.to_string())
}

async fn get_json<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
) -> Result<T, LoaderInstallError> {
    let response = client
        .get(url)
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|err| LoaderInstallError::Network(err.to_string()))?;

    if !response.status().is_success() {
        return Err(LoaderInstallError::Network(format!(
            "HTTP {} for {url}",
            response.status()
        )));
    }

    response
        .json()
        .await
        .map_err(|err| LoaderInstallError::Parse(err.to_string()))
}

pub async fn fetch_loader_versions(
    client: &reqwest::Client,
    base_url: &str,
    game_version: &str,
) -> Result<Vec<FabricLoaderBuild>, LoaderInstallError> {
    let url = build_url(base_url, &["v2", "versions", "loader", game_version])?;
    let entries: Vec<LoaderVersionsEntry> = get_json(client, &url).await?;
    Ok(entries.into_iter().map(|entry| entry.loader).collect())
}

/// Resolves `"latest"` to the newest stable build (falling back to the
/// newest build of any kind if none is marked stable - some very fresh
/// game versions briefly have only unstable builds), or looks up a
/// specific version by exact match otherwise.
pub async fn resolve_loader_version(
    client: &reqwest::Client,
    base_url: &str,
    game_version: &str,
    requested: &str,
) -> Result<String, LoaderInstallError> {
    let builds = fetch_loader_versions(client, base_url, game_version).await?;
    if builds.is_empty() {
        return Err(LoaderInstallError::NoLoaderForGameVersion {
            game_version: game_version.to_string(),
        });
    }

    if requested == "latest" {
        let chosen = builds
            .iter()
            .find(|build| build.stable)
            .unwrap_or(&builds[0]);
        return Ok(chosen.version.clone());
    }

    builds
        .iter()
        .find(|build| build.version == requested)
        .map(|build| build.version.clone())
        .ok_or_else(|| LoaderInstallError::UnknownLoaderVersion {
            loader_version: requested.to_string(),
        })
}

pub async fn fetch_profile(
    client: &reqwest::Client,
    base_url: &str,
    game_version: &str,
    loader_version: &str,
) -> Result<FabricProfile, LoaderInstallError> {
    let url = build_url(
        base_url,
        &[
            "v2",
            "versions",
            "loader",
            game_version,
            loader_version,
            "profile",
            "json",
        ],
    )?;
    get_json(client, &url).await
}

/// Resolves `requested_loader_version` (`"latest"` or an exact version),
/// fetches its profile, and downloads its libraries - the full sequence
/// both `install_instance` and `launch_instance` need, so neither
/// duplicates it. Returns the profile so a caller building a launch (not
/// just installing files) has `mainClass`/`arguments` without a second
/// fetch.
pub async fn resolve_and_install(
    client: &reqwest::Client,
    base_url: &str,
    libraries_dir: &Path,
    game_version: &str,
    requested_loader_version: &str,
    concurrency: usize,
    events: ProgressSink,
) -> Result<FabricProfile, LoaderInstallError> {
    let loader_version =
        resolve_loader_version(client, base_url, game_version, requested_loader_version).await?;
    let profile = fetch_profile(client, base_url, game_version, &loader_version).await?;
    install_libraries(client, libraries_dir, &profile, concurrency, events).await?;
    Ok(profile)
}

/// Downloads every library the resolved Fabric profile needs into
/// `libraries_dir` (the same directory Vanilla libraries share - Maven
/// coordinates never collide with Mojang's own artifact paths), through
/// the same `DownloadManager` every other download in this launcher goes
/// through.
pub async fn install_libraries(
    client: &reqwest::Client,
    libraries_dir: &Path,
    profile: &FabricProfile,
    concurrency: usize,
    events: ProgressSink,
) -> Result<(), LoaderInstallError> {
    let tasks: Vec<DownloadTask> = profile
        .libraries
        .iter()
        .map(|library| DownloadTask {
            url: library.download_url(),
            dest: libraries_dir.join(library.maven_path()),
            expected_sha1: None,
            label: library.name.clone(),
        })
        .collect();

    let summary = download_all(client, tasks, concurrency.max(1), events).await;
    if summary.failed > 0 {
        return Err(LoaderInstallError::DownloadFailed {
            failed: summary.failed,
            total: summary.total,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::spawn_mock_server;
    use std::collections::HashMap;

    fn library(name: &str, url: &str) -> FabricLibrary {
        FabricLibrary {
            name: name.to_string(),
            url: url.to_string(),
        }
    }

    #[test]
    fn maven_path_lays_out_group_artifact_version_the_standard_way() {
        let lib = library(
            "net.fabricmc:fabric-loader:0.16.9",
            "https://maven.fabricmc.net/",
        );
        assert_eq!(
            lib.maven_path(),
            PathBuf::from("net/fabricmc/fabric-loader/0.16.9/fabric-loader-0.16.9.jar")
        );
    }

    #[test]
    fn maven_path_handles_a_classifier() {
        let lib = library(
            "org.ow2.asm:asm:9.6:natives-linux",
            "https://maven.fabricmc.net/",
        );
        assert_eq!(
            lib.maven_path(),
            PathBuf::from("org/ow2/asm/asm/9.6/asm-9.6-natives-linux.jar")
        );
    }

    #[test]
    fn download_url_joins_the_repo_base_and_maven_path_with_exactly_one_slash() {
        let with_slash = library(
            "net.fabricmc:fabric-loader:0.16.9",
            "https://maven.fabricmc.net/",
        );
        let without_slash = library(
            "net.fabricmc:fabric-loader:0.16.9",
            "https://maven.fabricmc.net",
        );
        assert_eq!(with_slash.download_url(), without_slash.download_url());
        assert_eq!(
            with_slash.download_url(),
            "https://maven.fabricmc.net/net/fabricmc/fabric-loader/0.16.9/fabric-loader-0.16.9.jar"
        );
    }

    fn loader_versions_json() -> serde_json::Value {
        serde_json::json!([
            {
                "loader": { "separator": "+build.", "build": 132, "maven": "net.fabricmc:fabric-loader:0.14.24", "version": "0.14.24", "stable": true },
                "intermediary": { "maven": "net.fabricmc:intermediary:1.20.1", "version": "1.20.1", "stable": true }
            },
            {
                "loader": { "separator": "+build.", "build": 133, "maven": "net.fabricmc:fabric-loader:0.16.9", "version": "0.16.9", "stable": true },
                "intermediary": { "maven": "net.fabricmc:intermediary:1.20.1", "version": "1.20.1", "stable": true }
            },
            {
                "loader": { "separator": "+build.", "build": 134, "maven": "net.fabricmc:fabric-loader:0.17.0-beta.1", "version": "0.17.0-beta.1", "stable": false },
                "intermediary": { "maven": "net.fabricmc:intermediary:1.20.1", "version": "1.20.1", "stable": true }
            }
        ])
    }

    #[tokio::test]
    async fn fetch_loader_versions_fetches_and_extracts_the_loader_sub_object() {
        let mut files: HashMap<String, Vec<u8>> = HashMap::new();
        files.insert(
            "/v2/versions/loader/1.20.1".into(),
            serde_json::to_vec(&loader_versions_json()).unwrap(),
        );
        let base = spawn_mock_server(files).await;

        let client = reqwest::Client::new();
        let builds = fetch_loader_versions(&client, &base, "1.20.1")
            .await
            .unwrap();
        assert_eq!(builds.len(), 3);
        assert_eq!(builds[1].version, "0.16.9");
        assert!(builds[1].stable);
        assert!(!builds[2].stable);
    }

    #[tokio::test]
    async fn resolve_loader_version_picks_the_first_stable_build_for_latest() {
        let mut files: HashMap<String, Vec<u8>> = HashMap::new();
        files.insert(
            "/v2/versions/loader/1.20.1".into(),
            serde_json::to_vec(&loader_versions_json()).unwrap(),
        );
        let base = spawn_mock_server(files).await;

        let client = reqwest::Client::new();
        let resolved = resolve_loader_version(&client, &base, "1.20.1", "latest")
            .await
            .unwrap();
        assert_eq!(resolved, "0.14.24"); // first *stable* entry, not first overall
    }

    #[tokio::test]
    async fn resolve_loader_version_finds_an_exact_match() {
        let mut files: HashMap<String, Vec<u8>> = HashMap::new();
        files.insert(
            "/v2/versions/loader/1.20.1".into(),
            serde_json::to_vec(&loader_versions_json()).unwrap(),
        );
        let base = spawn_mock_server(files).await;

        let client = reqwest::Client::new();
        let resolved = resolve_loader_version(&client, &base, "1.20.1", "0.17.0-beta.1")
            .await
            .unwrap();
        assert_eq!(resolved, "0.17.0-beta.1");
    }

    #[tokio::test]
    async fn resolve_loader_version_reports_unknown_loader_version_on_a_typo() {
        let mut files: HashMap<String, Vec<u8>> = HashMap::new();
        files.insert(
            "/v2/versions/loader/1.20.1".into(),
            serde_json::to_vec(&loader_versions_json()).unwrap(),
        );
        let base = spawn_mock_server(files).await;

        let client = reqwest::Client::new();
        let result = resolve_loader_version(&client, &base, "1.20.1", "9.9.9-does-not-exist").await;
        assert!(matches!(
            result,
            Err(LoaderInstallError::UnknownLoaderVersion { .. })
        ));
    }

    #[tokio::test]
    async fn resolve_loader_version_reports_no_loader_for_game_version_when_empty() {
        let mut files: HashMap<String, Vec<u8>> = HashMap::new();
        files.insert(
            "/v2/versions/loader/1.0".into(),
            serde_json::to_vec(&serde_json::json!([])).unwrap(),
        );
        let base = spawn_mock_server(files).await;

        let client = reqwest::Client::new();
        let result = resolve_loader_version(&client, &base, "1.0", "latest").await;
        assert!(matches!(
            result,
            Err(LoaderInstallError::NoLoaderForGameVersion { .. })
        ));
    }

    fn profile_json() -> serde_json::Value {
        serde_json::json!({
            "id": "fabric-loader-0.16.9-1.20.1",
            "inheritsFrom": "1.20.1",
            "releaseTime": "2024-01-01T00:00:00+00:00",
            "time": "2024-01-01T00:00:00+00:00",
            "type": "release",
            "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
            "arguments": {
                "game": [],
                "jvm": ["-DFabricMcEmu=net.minecraft.client.main.Main "]
            },
            "libraries": [
                { "name": "net.fabricmc:fabric-loader:0.16.9", "url": "https://maven.fabricmc.net/" },
                { "name": "net.fabricmc:intermediary:1.20.1", "url": "https://maven.fabricmc.net/" }
            ]
        })
    }

    #[tokio::test]
    async fn fetch_profile_fetches_and_parses_a_real_shaped_response() {
        let mut files: HashMap<String, Vec<u8>> = HashMap::new();
        files.insert(
            "/v2/versions/loader/1.20.1/0.16.9/profile/json".into(),
            serde_json::to_vec(&profile_json()).unwrap(),
        );
        let base = spawn_mock_server(files).await;

        let client = reqwest::Client::new();
        let profile = fetch_profile(&client, &base, "1.20.1", "0.16.9")
            .await
            .unwrap();
        assert_eq!(
            profile.main_class,
            "net.fabricmc.loader.impl.launch.knot.KnotClient"
        );
        assert_eq!(profile.libraries.len(), 2);
        assert!(profile.arguments.game.is_empty());
        assert_eq!(profile.arguments.jvm.len(), 1);
    }

    #[tokio::test]
    async fn resolve_and_install_runs_the_full_chain_against_a_local_mock_server() {
        // Bind first (without serving yet) so the profile fixture below can
        // bake in this same server's own address as its libraries' Maven
        // repo URL - the exact chicken-and-egg problem
        // `bind_mock_server`/`serve_mock_files` (rather than the
        // bind-and-serve-immediately `spawn_mock_server`) exists to solve.
        let (listener, addr) = crate::test_support::bind_mock_server().await;
        let base = format!("http://{addr}");

        let mut profile = profile_json();
        for lib in profile["libraries"].as_array_mut().unwrap() {
            lib["url"] = serde_json::Value::String(format!("{base}/repo/"));
        }

        let mut files: HashMap<String, Vec<u8>> = HashMap::new();
        files.insert(
            "/v2/versions/loader/1.20.1".into(),
            serde_json::to_vec(&loader_versions_json()).unwrap(),
        );
        files.insert(
            "/v2/versions/loader/1.20.1/0.16.9/profile/json".into(),
            serde_json::to_vec(&profile).unwrap(),
        );
        files.insert(
            "/repo/net/fabricmc/fabric-loader/0.16.9/fabric-loader-0.16.9.jar".into(),
            b"fake loader jar".to_vec(),
        );
        files.insert(
            "/repo/net/fabricmc/intermediary/1.20.1/intermediary-1.20.1.jar".into(),
            b"fake intermediary jar".to_vec(),
        );
        crate::test_support::serve_mock_files(listener, files);

        let client = reqwest::Client::new();
        let dir = tempfile::tempdir().unwrap();
        // A specific version, not "latest" - this test is about the
        // resolve -> fetch profile -> download chain, not "latest"
        // selection (which `resolve_loader_version_picks_the_first_stable_build_for_latest`
        // already covers on its own).
        let resolved_profile = resolve_and_install(
            &client,
            &base,
            dir.path(),
            "1.20.1",
            "0.16.9",
            4,
            ProgressSink::none(),
        )
        .await
        .unwrap();

        assert_eq!(
            resolved_profile.main_class,
            "net.fabricmc.loader.impl.launch.knot.KnotClient"
        );
        assert!(dir
            .path()
            .join("net/fabricmc/fabric-loader/0.16.9/fabric-loader-0.16.9.jar")
            .is_file());
        assert!(dir
            .path()
            .join("net/fabricmc/intermediary/1.20.1/intermediary-1.20.1.jar")
            .is_file());
    }

    #[tokio::test]
    async fn install_libraries_downloads_every_library_to_its_maven_path() {
        let mut repo_files: HashMap<String, Vec<u8>> = HashMap::new();
        repo_files.insert(
            "/net/fabricmc/fabric-loader/0.16.9/fabric-loader-0.16.9.jar".into(),
            b"fake loader jar".to_vec(),
        );
        let repo_base = spawn_mock_server(repo_files).await;

        let profile = FabricProfile {
            id: "fabric-loader-0.16.9-1.20.1".into(),
            main_class: "net.fabricmc.loader.impl.launch.knot.KnotClient".into(),
            arguments: FabricArguments::default(),
            libraries: vec![library(
                "net.fabricmc:fabric-loader:0.16.9",
                &format!("{repo_base}/"),
            )],
        };

        let client = reqwest::Client::new();
        let dir = tempfile::tempdir().unwrap();
        install_libraries(&client, dir.path(), &profile, 4, ProgressSink::none())
            .await
            .unwrap();

        let installed = dir
            .path()
            .join("net/fabricmc/fabric-loader/0.16.9/fabric-loader-0.16.9.jar");
        assert!(installed.is_file());
        assert_eq!(std::fs::read(installed).unwrap(), b"fake loader jar");
    }
}
