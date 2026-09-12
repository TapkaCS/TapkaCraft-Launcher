//! Installs the two things a parsed modpack still needs beyond the base
//! Minecraft version and loader (handled by `core::versions::install` and
//! `core::loaders::fabric` the same way a manually-created instance uses
//! them): every file the manifest lists, and the `overrides/` content.

use std::fs::{self, File};
use std::path::{Path, PathBuf};

use super::index::ModpackIndex;
use super::ModpackError;
use crate::core::downloads::{download_all, DownloadTask, ProgressSink};

/// Downloads every file `index.files` declares into `instance_dir`,
/// skipping entries explicitly marked unsupported on the client (a
/// server-only performance plugin, say) - everything else (`required` or
/// `optional`, or no `env` at all) is installed, matching how the official
/// Modrinth App and every other third-party launcher installs a client
/// pack by default.
pub async fn download_pack_files(
    client: &reqwest::Client,
    index: &ModpackIndex,
    instance_dir: &Path,
    concurrency: usize,
    events: ProgressSink,
) -> Result<(), ModpackError> {
    let mut tasks = Vec::new();
    for file in &index.files {
        if file
            .env
            .as_ref()
            .is_some_and(|env| env.client == "unsupported")
        {
            continue;
        }
        let Some(url) = file.downloads.first() else {
            continue;
        };
        let Some(dest) = safe_relative_path(instance_dir, &file.path) else {
            continue;
        };
        tasks.push(DownloadTask {
            url: url.clone(),
            dest,
            expected_sha1: file.hashes.get("sha1").cloned(),
            label: file_label(&file.path),
        });
    }

    let summary = download_all(client, tasks, concurrency.max(1), events).await;
    if summary.failed > 0 {
        return Err(ModpackError::DownloadFailed(format!(
            "{} of {} modpack files failed to download",
            summary.failed, summary.total
        )));
    }
    Ok(())
}

/// Extracts `overrides/` and `client-overrides/` directly into
/// `instance_dir`, preserving their relative paths so nested config files
/// land where the pack author put them. Never `server-overrides/` - this
/// launcher only ever installs the client.
pub fn extract_overrides(mrpack_path: &Path, instance_dir: &Path) -> Result<(), ModpackError> {
    let file = File::open(mrpack_path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|err| ModpackError::Zip(err.to_string()))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|err| ModpackError::Zip(err.to_string()))?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        let Some(relative) = name
            .strip_prefix("overrides/")
            .or_else(|| name.strip_prefix("client-overrides/"))
        else {
            continue;
        };
        let Some(dest) = safe_relative_path(instance_dir, relative) else {
            continue;
        };

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out_file = File::create(&dest)?;
        std::io::copy(&mut entry, &mut out_file)?;
    }
    Ok(())
}

fn file_label(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

/// Joins `relative` (a `/`-separated path from inside an externally-authored
/// zip - never trusted as-is, same posture as
/// `instances::service::resolve_instance_dir`) onto `base`, rejecting any
/// `..` component so a malicious or malformed entry can never write outside
/// the instance directory (zip-slip). Returns `None` instead of erroring
/// the whole install over one bad entry - it's simply skipped.
fn safe_relative_path(base: &Path, relative: &str) -> Option<PathBuf> {
    let mut result = base.to_path_buf();
    let mut wrote_any = false;
    for component in relative.split('/') {
        if component.is_empty() || component == "." {
            continue;
        }
        if component == ".." {
            return None;
        }
        result.push(component);
        wrote_any = true;
    }
    wrote_any.then_some(result)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::io::Write;

    use sha1::{Digest, Sha1};

    use super::*;
    use crate::core::modpacks::index::{ModpackEnv, ModpackFile, ModpackIndex};
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

    fn sample_index(base: &str, mod_bytes: &[u8]) -> ModpackIndex {
        let mut hashes = HashMap::new();
        hashes.insert("sha1".to_string(), sha1_hex(mod_bytes));
        ModpackIndex {
            format_version: 1,
            game: "minecraft".into(),
            version_id: "1.0.0".into(),
            name: "Test Pack".into(),
            summary: None,
            files: vec![
                ModpackFile {
                    path: "mods/sodium.jar".into(),
                    hashes,
                    env: Some(ModpackEnv {
                        client: "required".into(),
                        server: "unsupported".into(),
                    }),
                    downloads: vec![format!("{base}/sodium.jar")],
                    file_size: mod_bytes.len() as u64,
                },
                ModpackFile {
                    path: "mods/server-only.jar".into(),
                    hashes: HashMap::new(),
                    env: Some(ModpackEnv {
                        client: "unsupported".into(),
                        server: "required".into(),
                    }),
                    downloads: vec![format!("{base}/server-only.jar")],
                    file_size: 1,
                },
            ],
            dependencies: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn download_pack_files_installs_client_files_and_skips_server_only_ones() {
        let mod_bytes = b"fake sodium jar".to_vec();
        let (listener, addr) = bind_mock_server().await;
        let base = format!("http://{addr}");

        let mut files: HashMap<String, Vec<u8>> = HashMap::new();
        files.insert("/sodium.jar".into(), mod_bytes.clone());
        // No "/server-only.jar" entry registered - if download_pack_files
        // tried to fetch it despite env.client == "unsupported", this test
        // would fail with a 404 rather than silently passing.
        serve_mock_files(listener, files);

        let index = sample_index(&base, &mod_bytes);
        let dir = tempfile::tempdir().unwrap();
        let client = reqwest::Client::new();

        download_pack_files(&client, &index, dir.path(), 4, ProgressSink::none())
            .await
            .unwrap();

        assert_eq!(
            std::fs::read(dir.path().join("mods/sodium.jar")).unwrap(),
            mod_bytes
        );
        assert!(!dir.path().join("mods/server-only.jar").exists());
    }

    fn build_mrpack_with_overrides() -> Vec<u8> {
        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer.start_file("modrinth.index.json", options).unwrap();
            writer.write_all(b"{}").unwrap();
            writer
                .start_file("overrides/config/mod.toml", options)
                .unwrap();
            writer.write_all(b"setting = true").unwrap();
            writer
                .start_file("client-overrides/options.txt", options)
                .unwrap();
            writer.write_all(b"fov:90").unwrap();
            writer
                .start_file("server-overrides/server.properties", options)
                .unwrap();
            writer.write_all(b"should never be extracted").unwrap();
            writer.finish().unwrap();
        }
        buffer.into_inner()
    }

    #[test]
    fn extract_overrides_writes_client_and_shared_overrides_but_not_server_ones() {
        let dir = tempfile::tempdir().unwrap();
        let mrpack_path = dir.path().join("pack.mrpack");
        std::fs::write(&mrpack_path, build_mrpack_with_overrides()).unwrap();

        let instance_dir = dir.path().join("instance");
        std::fs::create_dir_all(&instance_dir).unwrap();
        extract_overrides(&mrpack_path, &instance_dir).unwrap();

        assert_eq!(
            std::fs::read_to_string(instance_dir.join("config/mod.toml")).unwrap(),
            "setting = true"
        );
        assert_eq!(
            std::fs::read_to_string(instance_dir.join("options.txt")).unwrap(),
            "fov:90"
        );
        assert!(!instance_dir.join("server.properties").exists());
    }

    #[test]
    fn safe_relative_path_rejects_parent_directory_escapes() {
        let base = Path::new("/tmp/instance");
        assert_eq!(safe_relative_path(base, "../../etc/passwd"), None);
        assert_eq!(
            safe_relative_path(base, "config/mod.toml"),
            Some(base.join("config").join("mod.toml"))
        );
    }
}
