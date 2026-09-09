//! Ties `VersionService`'s metadata resolution to `core::downloads` to
//! actually install a Vanilla version: version JSON (cached to disk),
//! client jar, rule-filtered libraries, the asset index + every asset,
//! and native-library extraction. Loader installers (Fabric/Forge/...)
//! build on top of this in a later phase; this function only knows Vanilla.
//!
//! Known limitation: pre-1.7 versions used a different ("legacy") asset
//! layout addressed by real file paths instead of content hashes. That
//! format isn't implemented - only the hash-addressed layout every
//! version from ~2013 onward uses.

use std::path::{Path, PathBuf};

use super::assets::fetch_asset_index;
use super::manifest::VersionManifest;
use super::rules::RuleContext;
use super::version_json::fetch_version_info;
use super::{version_json::VersionInfo, VersionError};
use crate::core::downloads::{download_all, DownloadTask, ProgressSink};

/// The subset of `AppPaths` install needs, passed explicitly rather than
/// the whole struct so this stays testable without constructing a real
/// `AppPaths`.
pub struct InstallPaths {
    pub versions_dir: PathBuf,
    pub libraries_dir: PathBuf,
    pub assets_dir: PathBuf,
}

pub struct InstalledVersion {
    pub version_info: VersionInfo,
    pub client_jar: PathBuf,
    pub natives_dir: PathBuf,
}

/// Installs (or verifies an already-complete install of) one Vanilla
/// version. Idempotent: files that already exist with the right hash are
/// skipped by `DownloadManager`, so calling this again after an
/// interrupted install just resumes.
pub async fn ensure_installed(
    client: &reqwest::Client,
    paths: &InstallPaths,
    manifest: &VersionManifest,
    version_id: &str,
    concurrency: usize,
    events: ProgressSink,
) -> Result<InstalledVersion, VersionError> {
    let entry = manifest
        .find(version_id)
        .ok_or_else(|| VersionError::NotFound(version_id.to_string()))?;

    let version_dir = paths.versions_dir.join(version_id);
    std::fs::create_dir_all(&version_dir)?;
    let version_json_path = version_dir.join(format!("{version_id}.json"));

    let version_info = match std::fs::read(&version_json_path) {
        Ok(bytes) => {
            serde_json::from_slice(&bytes).map_err(|err| VersionError::Parse(err.to_string()))?
        }
        Err(_) => {
            let info = fetch_version_info(client, &entry.url).await?;
            let json = serde_json::to_vec_pretty(&info)
                .map_err(|err| VersionError::Parse(err.to_string()))?;
            std::fs::write(&version_json_path, json)?;
            info
        }
    };

    // Custom-resolution doesn't affect which files get installed (only
    // launch-time argument resolution does), so `false` is correct here
    // regardless of the user's actual settings.
    let ctx = RuleContext::current(false);

    let mut tasks = Vec::new();

    let client_jar_path = version_dir.join(format!("{version_id}.jar"));
    tasks.push(DownloadTask {
        url: version_info.downloads.client.url.clone(),
        dest: client_jar_path.clone(),
        expected_sha1: Some(version_info.downloads.client.sha1.clone()),
        label: format!("{version_id}.jar"),
    });

    for artifact in version_info.classpath_artifacts(&ctx) {
        tasks.push(DownloadTask {
            url: artifact.url.clone(),
            dest: paths.libraries_dir.join(&artifact.path),
            expected_sha1: Some(artifact.sha1.clone()),
            label: file_label(&artifact.path),
        });
    }

    let native_artifacts = version_info.native_artifacts(&ctx);
    for (artifact, _exclude) in &native_artifacts {
        tasks.push(DownloadTask {
            url: artifact.url.clone(),
            dest: paths.libraries_dir.join(&artifact.path),
            expected_sha1: Some(artifact.sha1.clone()),
            label: file_label(&artifact.path),
        });
    }

    let asset_index = fetch_asset_index(client, &version_info.asset_index.url).await?;
    let asset_index_path = paths
        .assets_dir
        .join("indexes")
        .join(format!("{}.json", version_info.asset_index.id));
    if let Some(parent) = asset_index_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let asset_index_json =
        serde_json::to_vec(&asset_index).map_err(|err| VersionError::Parse(err.to_string()))?;
    std::fs::write(&asset_index_path, asset_index_json)?;

    for object in asset_index.objects.values() {
        tasks.push(DownloadTask {
            url: object.download_url(),
            dest: paths.assets_dir.join(object.relative_path()),
            expected_sha1: Some(object.hash.clone()),
            label: object.hash.clone(),
        });
    }

    let summary = download_all(client, tasks, concurrency.max(1), events).await;
    if summary.failed > 0 {
        return Err(VersionError::Network(format!(
            "{} of {} files failed to download",
            summary.failed, summary.total
        )));
    }

    let natives_dir = version_dir.join("natives");
    std::fs::create_dir_all(&natives_dir)?;
    for (artifact, exclude) in &native_artifacts {
        let jar_path = paths.libraries_dir.join(&artifact.path);
        extract_natives_jar(&jar_path, &natives_dir, exclude)?;
    }

    Ok(InstalledVersion {
        version_info,
        client_jar: client_jar_path,
        natives_dir,
    })
}

fn file_label(library_path: &str) -> String {
    library_path
        .rsplit('/')
        .next()
        .unwrap_or(library_path)
        .to_string()
}

/// Extracts every file from a natives jar into `dest_dir`, flattening
/// paths to just the file name. That's not a shortcut: native jars are
/// flat in practice, and discarding any directory component from the
/// archive is exactly what stops a malformed/malicious entry (`../../..`)
/// from writing outside `dest_dir` (zip-slip).
fn extract_natives_jar(
    jar_path: &Path,
    dest_dir: &Path,
    exclude: &[String],
) -> Result<(), VersionError> {
    let file = std::fs::File::open(jar_path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|err| VersionError::Parse(err.to_string()))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|err| VersionError::Parse(err.to_string()))?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        if exclude
            .iter()
            .any(|pattern| name.starts_with(pattern.as_str()))
        {
            continue;
        }
        let Some(file_name) = Path::new(&name).file_name() else {
            continue;
        };

        let out_path = dest_dir.join(file_name);
        let mut out_file = std::fs::File::create(&out_path)?;
        std::io::copy(&mut entry, &mut out_file)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::io::Write;

    use sha1::{Digest, Sha1};

    use super::*;
    use crate::core::versions::manifest::{LatestVersions, VersionKind, VersionManifestEntry};
    use crate::test_support::{bind_mock_server, serve_mock_files};

    fn sha1_hex(data: &[u8]) -> String {
        let mut hasher = Sha1::new();
        hasher.update(data);
        hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    /// Builds a real (not fixture-text) zip archive in memory containing
    /// one file, for testing native-jar extraction against actual zip
    /// bytes rather than trusting the format by assumption.
    fn build_test_zip(entry_name: &str, content: &[u8]) -> Vec<u8> {
        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer.start_file(entry_name, options).unwrap();
            writer.write_all(content).unwrap();
            writer.finish().unwrap();
        }
        buffer.into_inner()
    }

    #[tokio::test]
    async fn installs_a_full_synthetic_version_end_to_end_against_a_local_server() {
        let client_jar_bytes = b"fake client jar contents".to_vec();
        let library_bytes = b"fake library jar".to_vec();
        let natives_jar_bytes = build_test_zip("liblwjgl.so", b"fake native library bytes");

        let (listener, addr) = bind_mock_server().await;
        let base = format!("http://{addr}");

        let mut files: HashMap<String, Vec<u8>> = HashMap::new();
        files.insert("/client.jar".into(), client_jar_bytes.clone());
        files.insert("/brigadier.jar".into(), library_bytes.clone());
        files.insert("/lwjgl-natives-linux.jar".into(), natives_jar_bytes.clone());

        // Real per-object asset URLs are hardcoded to Mojang's production
        // CDN (`AssetObject::download_url`), unreachable from this
        // sandboxed test - so the index below is intentionally empty.
        // `download_url`'s own correctness is covered directly by
        // `assets::tests::relative_path_uses_the_first_two_hex_chars_as_a_bucket`,
        // and the loop that turns each object into a `DownloadTask` here
        // is structurally identical to the (covered) library loop below.
        let asset_index_json = serde_json::json!({ "objects": {} });
        files.insert(
            "/assets/index.json".into(),
            serde_json::to_vec(&asset_index_json).unwrap(),
        );

        let version_json = serde_json::json!({
            "id": "1.20.1-test",
            "mainClass": "net.minecraft.client.main.Main",
            "downloads": { "client": { "url": format!("{base}/client.jar"), "sha1": sha1_hex(&client_jar_bytes), "size": client_jar_bytes.len() } },
            "assetIndex": { "id": "test-index", "url": format!("{base}/assets/index.json"), "sha1": "unused", "size": 1 },
            "javaVersion": { "component": "java-runtime-gamma", "majorVersion": 17 },
            "libraries": [
                {
                    "name": "com.mojang:brigadier:1.0.18",
                    "downloads": { "artifact": { "path": "com/mojang/brigadier/1.0.18/brigadier.jar", "url": format!("{base}/brigadier.jar"), "sha1": sha1_hex(&library_bytes), "size": library_bytes.len() } }
                },
                {
                    "name": "org.lwjgl:lwjgl:3.3.1:natives-linux",
                    "rules": [ { "action": "allow", "os": { "name": "linux" } } ],
                    "natives": { "linux": "natives-linux" },
                    "downloads": {
                        "classifiers": {
                            "natives-linux": { "path": "org/lwjgl/lwjgl/3.3.1/lwjgl-natives-linux.jar", "url": format!("{base}/lwjgl-natives-linux.jar"), "sha1": sha1_hex(&natives_jar_bytes), "size": natives_jar_bytes.len() }
                        }
                    }
                }
            ]
        });
        files.insert(
            "/1.20.1-test.json".into(),
            serde_json::to_vec(&version_json).unwrap(),
        );

        serve_mock_files(listener, files);

        let manifest = VersionManifest {
            latest: LatestVersions {
                release: "1.20.1-test".into(),
                snapshot: "1.20.1-test".into(),
            },
            versions: vec![VersionManifestEntry {
                id: "1.20.1-test".into(),
                kind: VersionKind::Release,
                url: format!("{base}/1.20.1-test.json"),
                sha1: "unused".into(),
                release_time: "2023-01-01T00:00:00+00:00".into(),
            }],
        };

        let dir = tempfile::tempdir().unwrap();
        let paths = InstallPaths {
            versions_dir: dir.path().join("versions"),
            libraries_dir: dir.path().join("libraries"),
            assets_dir: dir.path().join("assets"),
        };

        let client = reqwest::Client::new();
        let installed = ensure_installed(
            &client,
            &paths,
            &manifest,
            "1.20.1-test",
            4,
            ProgressSink::none(),
        )
        .await
        .unwrap();

        // Client jar landed where expected, with the right bytes.
        assert_eq!(
            std::fs::read(&installed.client_jar).unwrap(),
            client_jar_bytes
        );

        // The always-applicable library is on disk under libraries_dir.
        let lib_path = paths
            .libraries_dir
            .join("com/mojang/brigadier/1.0.18/brigadier.jar");
        assert_eq!(std::fs::read(lib_path).unwrap(), library_bytes);

        // The (empty) asset index itself was still fetched and cached to
        // disk - the fetch/cache code path runs the same regardless of
        // how many objects it lists.
        assert!(paths.assets_dir.join("indexes/test-index.json").is_file());

        // Natives were extracted (not just downloaded as a jar) into the
        // version's natives dir, flattened to a bare file name.
        let extracted_native = installed.natives_dir.join("liblwjgl.so");
        assert_eq!(
            std::fs::read(extracted_native).unwrap(),
            b"fake native library bytes"
        );

        // Calling it again must succeed by hitting the cache, not
        // re-parsing a version JSON the mock server no longer serves
        // under a fresh path.
        let reinstalled = ensure_installed(
            &client,
            &paths,
            &manifest,
            "1.20.1-test",
            4,
            ProgressSink::none(),
        )
        .await
        .unwrap();
        assert_eq!(reinstalled.version_info.id, "1.20.1-test");
    }

    #[test]
    fn extract_natives_jar_flattens_paths_and_respects_exclusions() {
        let dir = tempfile::tempdir().unwrap();
        let jar_path = dir.path().join("natives.jar");

        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer.start_file("nested/dir/lib.so", options).unwrap();
            writer.write_all(b"native payload").unwrap();
            writer.start_file("META-INF/MANIFEST.MF", options).unwrap();
            writer.write_all(b"should be excluded").unwrap();
            writer.finish().unwrap();
        }
        std::fs::write(&jar_path, buffer.into_inner()).unwrap();

        let dest_dir = dir.path().join("natives");
        std::fs::create_dir_all(&dest_dir).unwrap();
        extract_natives_jar(&jar_path, &dest_dir, &["META-INF/".to_string()]).unwrap();

        // Nested path was flattened to just the file name.
        assert_eq!(
            std::fs::read(dest_dir.join("lib.so")).unwrap(),
            b"native payload"
        );
        // Excluded prefix was not extracted at all.
        assert!(!dest_dir.join("MANIFEST.MF").exists());
    }

    #[test]
    fn install_paths_are_plain_data_and_do_not_require_a_real_app_paths() {
        let paths = InstallPaths {
            versions_dir: PathBuf::from("/tmp/versions"),
            libraries_dir: PathBuf::from("/tmp/libraries"),
            assets_dir: PathBuf::from("/tmp/assets"),
        };
        assert_eq!(paths.versions_dir, PathBuf::from("/tmp/versions"));
    }
}
