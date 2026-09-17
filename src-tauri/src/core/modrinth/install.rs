//! Installs a chosen Modrinth version's file into an instance's directory
//! (`mods/`, `resourcepacks/` or `shaderpacks/`, per `ContentKind`), then
//! recursively resolves and installs every "required" dependency's own
//! compatible version - so choosing one mod doesn't leave an instance with
//! missing prerequisites the game would otherwise fail to start with.
//! Tracks what's installed in a small per-instance, per-content-kind
//! manifest so the UI can show "already installed" without re-hashing
//! every file on every search.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::api::{list_project_versions, ModrinthVersion};
use super::{ContentKind, ModrinthError};
use crate::core::downloads::{download_all, DownloadTask, ProgressSink};

/// This launcher's own record, not a mirror of any Modrinth API shape -
/// camelCase to match every other struct this codebase sends over IPC.
/// Shared by mods, resource packs and shaders - the shape (which project,
/// which version, what the file ended up called) is identical for all
/// three.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledContent {
    pub project_id: String,
    pub version_id: String,
    pub title: String,
    pub file_name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstalledContentList {
    /// Keyed by Modrinth project id.
    pub items: HashMap<String, InstalledContent>,
}

fn manifest_path(instance_dir: &Path, content_kind: ContentKind) -> PathBuf {
    instance_dir.join(format!("tapkacraft-{}.json", content_kind.install_subdir()))
}

/// Reads the instance's installed-content manifest for `content_kind`,
/// defaulting to empty on any failure (missing file, corrupted JSON) - a
/// disposable local index of what this launcher itself installed, not the
/// source of truth for what's actually on disk (a user can always
/// add/remove files by hand).
pub fn read_installed(instance_dir: &Path, content_kind: ContentKind) -> InstalledContentList {
    std::fs::read(manifest_path(instance_dir, content_kind))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn write_installed(
    instance_dir: &Path,
    content_kind: ContentKind,
    installed: &InstalledContentList,
) -> Result<(), ModrinthError> {
    let path = manifest_path(instance_dir, content_kind);
    let json = serde_json::to_vec_pretty(installed)
        .map_err(|err| ModrinthError::Parse(err.to_string()))?;
    let tmp_path = path.with_extension("json.tmp");
    std::fs::write(&tmp_path, json)?;
    std::fs::rename(&tmp_path, &path)?;
    Ok(())
}

pub struct InstallResult {
    /// Everything actually installed by this call, the requested item plus
    /// whatever required dependencies weren't already present.
    pub installed: Vec<InstalledContent>,
    /// Required dependencies Modrinth has no compatible build of for this
    /// target - installed anyway (best effort), but flagged rather than
    /// silently left out, since the item may not work without them.
    pub skipped_dependencies: Vec<String>,
}

/// What every version fetched during dependency resolution has to be
/// compatible with - travels together through `install_version`, so it's
/// one parameter instead of three. `loader` is `None` for content kinds
/// that aren't tied to a mod loader (see `ContentKind::uses_loader_facet`).
pub struct CompatibilityTarget<'a> {
    pub base_url: &'a str,
    pub loader: Option<&'a str>,
    pub game_version: &'a str,
}

/// Installs `version` into `instance_dir`'s content-kind-appropriate
/// subdirectory, then walks its dependency graph (breadth-first,
/// cycle-safe) installing every "required" dependency's own compatible
/// version. Items already recorded as installed (by a previous call) are
/// left untouched.
pub async fn install_version(
    client: &reqwest::Client,
    target: &CompatibilityTarget<'_>,
    instance_dir: &Path,
    content_kind: ContentKind,
    version: ModrinthVersion,
    concurrency: usize,
    events: ProgressSink,
) -> Result<InstallResult, ModrinthError> {
    let content_dir = instance_dir.join(content_kind.install_subdir());
    let mut installed = read_installed(instance_dir, content_kind);

    let mut queue = vec![version];
    let mut queued_projects: HashSet<String> = HashSet::new();
    let mut tasks = Vec::new();
    let mut pending_records = Vec::new();
    let mut skipped_dependencies = Vec::new();

    while let Some(version) = queue.pop() {
        if !queued_projects.insert(version.project_id.clone())
            || installed.items.contains_key(&version.project_id)
        {
            continue;
        }

        if let Some(file) = version
            .files
            .iter()
            .find(|f| f.primary)
            .or_else(|| version.files.first())
        {
            tasks.push(DownloadTask {
                url: file.url.clone(),
                dest: content_dir.join(&file.filename),
                expected_sha1: Some(file.hashes.sha1.clone()),
                label: version.name.clone(),
            });
            pending_records.push(InstalledContent {
                project_id: version.project_id.clone(),
                version_id: version.id.clone(),
                title: version.name.clone(),
                file_name: file.filename.clone(),
            });
        }

        for dependency in &version.dependencies {
            if !dependency.is_required() {
                continue;
            }
            let Some(dep_project_id) = &dependency.project_id else {
                continue;
            };
            if installed.items.contains_key(dep_project_id)
                || queued_projects.contains(dep_project_id)
            {
                continue;
            }
            let dep_versions = list_project_versions(
                client,
                target.base_url,
                dep_project_id,
                target.loader,
                target.game_version,
            )
            .await?;
            match dep_versions.into_iter().next() {
                Some(dep_version) => queue.push(dep_version),
                None => skipped_dependencies.push(dep_project_id.clone()),
            }
        }
    }

    let summary = download_all(client, tasks, concurrency.max(1), events).await;
    if summary.failed > 0 {
        return Err(ModrinthError::DownloadFailed {
            failed: summary.failed,
            total: summary.total,
        });
    }

    let mut newly_installed = Vec::new();
    for record in pending_records {
        installed
            .items
            .insert(record.project_id.clone(), record.clone());
        newly_installed.push(record);
    }
    write_installed(instance_dir, content_kind, &installed)?;

    Ok(InstallResult {
        installed: newly_installed,
        skipped_dependencies,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::modrinth::api::{ModrinthDependency, ModrinthFile, ModrinthHashes};
    use crate::test_support::{bind_mock_server, serve_mock_files};
    use std::collections::HashMap as StdHashMap;

    fn version(
        project_id: &str,
        file_url: &str,
        sha1: &str,
        deps: Vec<ModrinthDependency>,
    ) -> ModrinthVersion {
        ModrinthVersion {
            id: format!("{project_id}-ver"),
            project_id: project_id.to_string(),
            name: format!("{project_id} 1.0"),
            version_number: "1.0".into(),
            game_versions: vec!["1.20.1".into()],
            loaders: vec!["fabric".into()],
            dependencies: deps,
            files: vec![ModrinthFile {
                url: file_url.to_string(),
                filename: format!("{project_id}.jar"),
                primary: true,
                hashes: ModrinthHashes {
                    sha1: sha1.to_string(),
                },
            }],
        }
    }

    fn required_dep(project_id: &str) -> ModrinthDependency {
        ModrinthDependency {
            project_id: Some(project_id.to_string()),
            version_id: None,
            dependency_type: "required".into(),
        }
    }

    #[tokio::test]
    async fn install_version_downloads_the_mod_and_records_it_in_the_manifest() {
        let mut files: StdHashMap<String, Vec<u8>> = StdHashMap::new();
        files.insert("/sodium.jar".into(), b"fake jar bytes".to_vec());
        let base = crate::test_support::spawn_mock_server(files).await;

        let client = reqwest::Client::new();
        let dir = tempfile::tempdir().unwrap();
        let sha1 = sha1_hex(b"fake jar bytes");
        let ver = version("AANobbMI", &format!("{base}/sodium.jar"), &sha1, vec![]);

        let target = CompatibilityTarget {
            base_url: "http://unused.invalid",
            loader: Some("fabric"),
            game_version: "1.20.1",
        };
        let result = install_version(
            &client,
            &target,
            dir.path(),
            ContentKind::Mod,
            ver,
            4,
            ProgressSink::none(),
        )
        .await
        .unwrap();

        assert_eq!(result.installed.len(), 1);
        assert!(result.skipped_dependencies.is_empty());
        assert!(dir.path().join("mods/AANobbMI.jar").is_file());

        let manifest = read_installed(dir.path(), ContentKind::Mod);
        assert!(manifest.items.contains_key("AANobbMI"));

        // Idempotent: installing the exact same version again is a no-op
        // that doesn't re-download or duplicate the manifest entry.
        let ver_again = version("AANobbMI", &format!("{base}/sodium.jar"), &sha1, vec![]);
        let second = install_version(
            &client,
            &target,
            dir.path(),
            ContentKind::Mod,
            ver_again,
            4,
            ProgressSink::none(),
        )
        .await
        .unwrap();
        assert!(second.installed.is_empty());
    }

    #[tokio::test]
    async fn install_version_installs_a_resourcepack_into_its_own_directory_and_manifest() {
        let mut files: StdHashMap<String, Vec<u8>> = StdHashMap::new();
        files.insert("/faithful.zip".into(), b"fake resourcepack bytes".to_vec());
        let base = crate::test_support::spawn_mock_server(files).await;

        let client = reqwest::Client::new();
        let dir = tempfile::tempdir().unwrap();
        let sha1 = sha1_hex(b"fake resourcepack bytes");
        let ver = ModrinthVersion {
            id: "faithful-ver".into(),
            project_id: "faithful".into(),
            name: "Faithful 1.0".into(),
            version_number: "1.0".into(),
            game_versions: vec!["1.20.1".into()],
            loaders: vec![],
            dependencies: vec![],
            files: vec![ModrinthFile {
                url: format!("{base}/faithful.zip"),
                filename: "faithful.zip".into(),
                primary: true,
                hashes: ModrinthHashes { sha1 },
            }],
        };

        // Resource packs aren't loader-gated - proves that reaches all the
        // way through without a loader value at all, not just that `None`
        // type-checks.
        let target = CompatibilityTarget {
            base_url: "http://unused.invalid",
            loader: None,
            game_version: "1.20.1",
        };
        let result = install_version(
            &client,
            &target,
            dir.path(),
            ContentKind::Resourcepack,
            ver,
            4,
            ProgressSink::none(),
        )
        .await
        .unwrap();

        assert_eq!(result.installed.len(), 1);
        assert!(dir.path().join("resourcepacks/faithful.zip").is_file());
        // Never in `mods/` - a different content kind, a different folder.
        assert!(!dir.path().join("mods/faithful.zip").is_file());

        // Tracked in its own manifest, entirely separate from mods' -
        // so a mod and a resource pack sharing a project id (unlikely, but
        // the manifests are keyed by content kind specifically so it can
        // never matter) wouldn't collide.
        let resourcepacks = read_installed(dir.path(), ContentKind::Resourcepack);
        assert!(resourcepacks.items.contains_key("faithful"));
        let mods = read_installed(dir.path(), ContentKind::Mod);
        assert!(mods.items.is_empty());
    }

    #[tokio::test]
    async fn install_version_recursively_installs_a_required_dependency() {
        let (listener, addr) = bind_mock_server().await;
        let base = format!("http://{addr}");

        let dep_sha1 = sha1_hex(b"dependency jar bytes");
        let dep_version_json = serde_json::to_vec(&vec![serde_json::json!({
            "id": "dep-ver",
            "project_id": "P7dR8mSH",
            "name": "Fabric API 1.0",
            "version_number": "1.0",
            "game_versions": ["1.20.1"],
            "loaders": ["fabric"],
            "dependencies": [],
            "files": [{
                "url": format!("{base}/fabric-api.jar"),
                "filename": "fabric-api.jar",
                "primary": true,
                "hashes": { "sha1": dep_sha1 }
            }]
        })])
        .unwrap();

        let mut files: StdHashMap<String, Vec<u8>> = StdHashMap::new();
        files.insert("/project/P7dR8mSH/version".into(), dep_version_json);
        files.insert("/fabric-api.jar".into(), b"dependency jar bytes".to_vec());
        files.insert("/sodium.jar".into(), b"fake jar bytes".to_vec());
        serve_mock_files(listener, files);

        let client = reqwest::Client::new();
        let dir = tempfile::tempdir().unwrap();
        let main_sha1 = sha1_hex(b"fake jar bytes");
        let ver = version(
            "AANobbMI",
            &format!("{base}/sodium.jar"),
            &main_sha1,
            vec![required_dep("P7dR8mSH")],
        );

        let target = CompatibilityTarget {
            base_url: &base,
            loader: Some("fabric"),
            game_version: "1.20.1",
        };
        let result = install_version(
            &client,
            &target,
            dir.path(),
            ContentKind::Mod,
            ver,
            4,
            ProgressSink::none(),
        )
        .await
        .unwrap();

        assert_eq!(result.installed.len(), 2);
        assert!(result.skipped_dependencies.is_empty());
        assert!(dir.path().join("mods/AANobbMI.jar").is_file());
        // Filename comes from the dependency's own file entry, not a
        // project-id-derived name - "fabric-api.jar" per the fixture above.
        assert!(dir.path().join("mods/fabric-api.jar").is_file());

        let manifest = read_installed(dir.path(), ContentKind::Mod);
        assert!(manifest.items.contains_key("AANobbMI"));
        assert!(manifest.items.contains_key("P7dR8mSH"));
    }

    #[tokio::test]
    async fn install_version_flags_a_required_dependency_with_no_compatible_build() {
        let (listener, addr) = bind_mock_server().await;
        let base = format!("http://{addr}");

        let mut files: StdHashMap<String, Vec<u8>> = StdHashMap::new();
        // No entry for /project/missing-dep/version - the mock server 404s,
        // which list_project_versions would normally treat as an error;
        // here it's registered with an empty array so this test isolates
        // "no compatible version" from "network failure".
        files.insert(
            "/project/missing-dep/version".into(),
            serde_json::to_vec(&Vec::<serde_json::Value>::new()).unwrap(),
        );
        files.insert("/sodium.jar".into(), b"fake jar bytes".to_vec());
        serve_mock_files(listener, files);

        let client = reqwest::Client::new();
        let dir = tempfile::tempdir().unwrap();
        let main_sha1 = sha1_hex(b"fake jar bytes");
        let ver = version(
            "AANobbMI",
            &format!("{base}/sodium.jar"),
            &main_sha1,
            vec![required_dep("missing-dep")],
        );

        let target = CompatibilityTarget {
            base_url: &base,
            loader: Some("fabric"),
            game_version: "1.20.1",
        };
        let result = install_version(
            &client,
            &target,
            dir.path(),
            ContentKind::Mod,
            ver,
            4,
            ProgressSink::none(),
        )
        .await
        .unwrap();

        assert_eq!(result.installed.len(), 1); // the mod itself, not the unresolvable dep
        assert_eq!(result.skipped_dependencies, vec!["missing-dep".to_string()]);
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
}
