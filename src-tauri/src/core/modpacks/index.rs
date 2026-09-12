//! `modrinth.index.json` - the manifest at the root of every `.mrpack`.
//! Field names/casing and the loader dependency keys below are Modrinth's
//! own modpack format (independent of, and camelCase unlike, the plain
//! REST API in `core::modrinth::api`): `formatVersion`, `game`,
//! `versionId`, `name`, `summary`, `files[]` (each with `path`, `hashes`,
//! an optional `env`, `downloads`, `fileSize`), and a `dependencies` map
//! keyed by `"minecraft"` plus at most one of `"fabric-loader"`,
//! `"quilt-loader"`, `"forge"`, `"neoforge"`.

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::ModpackError;
use crate::core::instances::model::LoaderKind;

const INDEX_ENTRY_NAME: &str = "modrinth.index.json";

const DEP_MINECRAFT: &str = "minecraft";
const DEP_FABRIC: &str = "fabric-loader";
const DEP_QUILT: &str = "quilt-loader";
const DEP_FORGE: &str = "forge";
const DEP_NEOFORGE: &str = "neoforge";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackIndex {
    pub format_version: u32,
    pub game: String,
    pub version_id: String,
    pub name: String,
    #[serde(default)]
    pub summary: Option<String>,
    pub files: Vec<ModpackFile>,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackFile {
    /// Relative to the instance root, forward-slash separated regardless
    /// of host OS - the format's own convention, not this codebase's.
    pub path: String,
    #[serde(default)]
    pub hashes: HashMap<String, String>,
    #[serde(default)]
    pub env: Option<ModpackEnv>,
    #[serde(default)]
    pub downloads: Vec<String>,
    #[serde(default)]
    pub file_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModpackEnv {
    /// One of `"required"`, `"optional"`, `"unsupported"`. Only `client`
    /// matters here - this launcher only ever installs the client side.
    pub client: String,
    #[serde(default)]
    pub server: String,
}

/// Reads and parses just the manifest out of a `.mrpack` (a zip file),
/// without touching `overrides/` - see `install::extract_overrides` for
/// that half.
pub fn read_index(mrpack_path: &Path) -> Result<ModpackIndex, ModpackError> {
    let file = File::open(mrpack_path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|err| ModpackError::Zip(err.to_string()))?;
    let mut entry = archive
        .by_name(INDEX_ENTRY_NAME)
        .map_err(|_| ModpackError::Parse(format!("missing {INDEX_ENTRY_NAME}")))?;
    let mut contents = String::new();
    entry.read_to_string(&mut contents)?;
    serde_json::from_str(&contents).map_err(|err| ModpackError::Parse(err.to_string()))
}

/// Picks the one loader this pack declares (a well-formed pack has at most
/// one loader dependency alongside `"minecraft"`) and its pinned version.
/// Vanilla packs simply omit every loader key. Errors for a loader this
/// launcher can't install yet rather than silently falling back to
/// Vanilla, which would produce an instance that looks installed but is
/// missing every mod the pack actually wanted.
pub fn resolve_loader(
    dependencies: &HashMap<String, String>,
) -> Result<(LoaderKind, String), ModpackError> {
    if let Some(version) = dependencies.get(DEP_FABRIC) {
        return Ok((LoaderKind::Fabric, version.clone()));
    }
    if dependencies.contains_key(DEP_QUILT) {
        return Err(ModpackError::UnsupportedLoader { loader: "Quilt" });
    }
    if dependencies.contains_key(DEP_FORGE) {
        return Err(ModpackError::UnsupportedLoader { loader: "Forge" });
    }
    if dependencies.contains_key(DEP_NEOFORGE) {
        return Err(ModpackError::UnsupportedLoader { loader: "NeoForge" });
    }

    let minecraft_version = dependencies
        .get(DEP_MINECRAFT)
        .ok_or(ModpackError::MissingMinecraftVersion)?;
    Ok((LoaderKind::Vanilla, minecraft_version.clone()))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn sample_index_json() -> serde_json::Value {
        serde_json::json!({
            "formatVersion": 1,
            "game": "minecraft",
            "versionId": "1.2.0",
            "name": "TapkaPack",
            "summary": "A cozy modpack",
            "files": [
                {
                    "path": "mods/sodium.jar",
                    "hashes": { "sha1": "abc123", "sha512": "def456" },
                    "env": { "client": "required", "server": "unsupported" },
                    "downloads": ["https://cdn.modrinth.com/sodium.jar"],
                    "fileSize": 12345
                }
            ],
            "dependencies": {
                "minecraft": "1.20.1",
                "fabric-loader": "0.15.7"
            }
        })
    }

    fn build_mrpack(index_json: &serde_json::Value) -> Vec<u8> {
        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer.start_file(INDEX_ENTRY_NAME, options).unwrap();
            writer
                .write_all(serde_json::to_string(index_json).unwrap().as_bytes())
                .unwrap();
            writer
                .start_file("overrides/config/mod.toml", options)
                .unwrap();
            writer.write_all(b"setting = true").unwrap();
            writer.finish().unwrap();
        }
        buffer.into_inner()
    }

    #[test]
    fn read_index_parses_a_real_shaped_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pack.mrpack");
        std::fs::write(&path, build_mrpack(&sample_index_json())).unwrap();

        let index = read_index(&path).unwrap();
        assert_eq!(index.format_version, 1);
        assert_eq!(index.version_id, "1.2.0");
        assert_eq!(index.name, "TapkaPack");
        assert_eq!(index.summary.as_deref(), Some("A cozy modpack"));
        assert_eq!(index.files.len(), 1);
        assert_eq!(index.files[0].path, "mods/sodium.jar");
        assert_eq!(
            index.files[0].hashes.get("sha1"),
            Some(&"abc123".to_string())
        );
        assert_eq!(index.files[0].env.as_ref().unwrap().client, "required");
        assert_eq!(
            index.dependencies.get("minecraft"),
            Some(&"1.20.1".to_string())
        );
    }

    #[test]
    fn read_index_reports_a_clear_error_for_a_non_mrpack_zip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.mrpack");
        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let writer = zip::ZipWriter::new(&mut buffer);
            writer.finish().unwrap();
        }
        std::fs::write(&path, buffer.into_inner()).unwrap();

        let err = read_index(&path).unwrap_err();
        assert!(matches!(err, ModpackError::Parse(_)));
    }

    #[test]
    fn resolve_loader_finds_fabric_and_its_pinned_version() {
        let mut deps = HashMap::new();
        deps.insert("minecraft".to_string(), "1.20.1".to_string());
        deps.insert("fabric-loader".to_string(), "0.15.7".to_string());

        let (kind, version) = resolve_loader(&deps).unwrap();
        assert_eq!(kind, LoaderKind::Fabric);
        assert_eq!(version, "0.15.7");
    }

    #[test]
    fn resolve_loader_falls_back_to_vanilla_when_no_loader_key_is_present() {
        let mut deps = HashMap::new();
        deps.insert("minecraft".to_string(), "1.20.1".to_string());

        let (kind, version) = resolve_loader(&deps).unwrap();
        assert_eq!(kind, LoaderKind::Vanilla);
        assert_eq!(version, "1.20.1");
    }

    #[test]
    fn resolve_loader_rejects_a_pack_with_no_minecraft_dependency_at_all() {
        let deps = HashMap::new();
        let err = resolve_loader(&deps).unwrap_err();
        assert!(matches!(err, ModpackError::MissingMinecraftVersion));
    }

    #[test]
    fn resolve_loader_reports_forge_as_unsupported_rather_than_silently_using_vanilla() {
        let mut deps = HashMap::new();
        deps.insert("minecraft".to_string(), "1.20.1".to_string());
        deps.insert("forge".to_string(), "47.2.0".to_string());

        let err = resolve_loader(&deps).unwrap_err();
        assert!(matches!(
            err,
            ModpackError::UnsupportedLoader { loader: "Forge" }
        ));
    }
}
