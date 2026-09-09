//! `InstanceService`: real filesystem persistence for instances. Each
//! instance is its own directory under the launcher's instances root
//! (`AppPaths::instances_dir`) holding `instance.json` plus its own
//! `mods/`, `saves/`, `resourcepacks/`, `shaderpacks/`, `screenshots/` -
//! isolated from every other instance. Global assets/libraries/runtimes/
//! versions are deliberately out of scope here (they live directly under
//! `AppPaths`, shared across instances - see the "INSTANCE SYSTEM" section
//! of the project plan).

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::model::{InstanceMeta, InstanceUpdate, JavaSettings, NewInstanceInput};

const INSTANCE_METADATA_FILE: &str = "instance.json";
const INSTANCE_SUBDIRS: [&str; 5] = [
    "mods",
    "saves",
    "resourcepacks",
    "shaderpacks",
    "screenshots",
];
const MAX_ID_LEN: usize = 128;

#[derive(Debug)]
pub enum InstanceError {
    NotFound(String),
    AlreadyExists(String),
    InvalidId(String),
    InvalidName(String),
    Io(std::io::Error),
    Serialization(serde_json::Error),
}

impl fmt::Display for InstanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "No instance with id \"{id}\" was found."),
            Self::AlreadyExists(id) => write!(f, "An instance with id \"{id}\" already exists."),
            Self::InvalidId(id) => write!(f, "\"{id}\" is not a valid instance id."),
            Self::InvalidName(msg) => write!(f, "{msg}"),
            Self::Io(err) => write!(f, "Instance storage error: {err}"),
            Self::Serialization(err) => write!(f, "Instance metadata is corrupted: {err}"),
        }
    }
}

impl std::error::Error for InstanceError {}

impl From<std::io::Error> for InstanceError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for InstanceError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(err)
    }
}

// Tauri commands return their error type to the frontend as JSON, so this
// needs to serialize to something the UI can show directly - the Display
// message is exactly that "concrete, not just Something went wrong" text.
impl Serialize for InstanceError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Lists every instance under `instances_dir`, sorted by name. A directory
/// that fails to parse (corrupted `instance.json`) is skipped rather than
/// failing the whole listing - one damaged instance should not hide every
/// other, still-healthy one.
pub fn list(instances_dir: &Path) -> Result<Vec<InstanceMeta>, InstanceError> {
    if !instances_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut result = Vec::new();
    for entry in fs::read_dir(instances_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let meta_path = entry.path().join(INSTANCE_METADATA_FILE);
        let Ok(bytes) = fs::read(&meta_path) else {
            continue;
        };
        if let Ok(meta) = serde_json::from_slice::<InstanceMeta>(&bytes) {
            result.push(meta);
        }
    }

    result.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(result)
}

/// Creates a new instance: a fresh directory (name slugified into a unique,
/// filesystem-safe id), its standard subdirectories, and `instance.json`.
pub fn create(
    instances_dir: &Path,
    input: NewInstanceInput,
) -> Result<InstanceMeta, InstanceError> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(InstanceError::InvalidName(
            "Profile name cannot be empty.".into(),
        ));
    }

    fs::create_dir_all(instances_dir)?;
    let id = unique_id(instances_dir, name);
    let dir = instances_dir.join(&id);
    if dir.exists() {
        // unique_id already checked, but a concurrent creation could still
        // race us here - fail loudly rather than silently overwrite.
        return Err(InstanceError::AlreadyExists(id));
    }

    fs::create_dir_all(&dir)?;
    for sub in INSTANCE_SUBDIRS {
        fs::create_dir_all(dir.join(sub))?;
    }

    let meta = InstanceMeta {
        id,
        name: name.to_string(),
        minecraft_version: input.minecraft_version,
        loader: input.loader,
        java: JavaSettings {
            memory_min_mb: 1024,
            memory_max_mb: 4096,
        },
        icon: None,
        favorite: false,
        playtime_seconds: 0,
        launch_count: 0,
        last_played: None,
    };

    if let Err(err) = write_meta(&dir, &meta) {
        // Don't leave a half-created instance directory behind on failure.
        let _ = fs::remove_dir_all(&dir);
        return Err(err);
    }
    Ok(meta)
}

/// Applies a partial edit (only `Some` fields change) and persists it.
pub fn update(
    instances_dir: &Path,
    id: &str,
    patch: InstanceUpdate,
) -> Result<InstanceMeta, InstanceError> {
    let dir = resolve_instance_dir(instances_dir, id)?;
    let mut meta = read_meta(&dir)?;

    if let Some(name) = patch.name {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(InstanceError::InvalidName(
                "Profile name cannot be empty.".into(),
            ));
        }
        meta.name = trimmed.to_string();
    }
    if let Some(favorite) = patch.favorite {
        meta.favorite = favorite;
    }
    if let Some(java) = patch.java {
        meta.java = java;
    }

    write_meta(&dir, &meta)?;
    Ok(meta)
}

/// Permanently deletes an instance's directory and everything in it
/// (mods, saves, config...). Confirmation is a UI-layer responsibility.
pub fn delete(instances_dir: &Path, id: &str) -> Result<(), InstanceError> {
    let dir = resolve_instance_dir(instances_dir, id)?;
    fs::remove_dir_all(&dir)?;
    Ok(())
}

/// Resolves `id` to its directory, first validating it as a safe path
/// segment. Because `is_valid_id` only allows `[a-z0-9_-]`, the resulting
/// path can never escape `instances_dir` (no `..`, no separators) -
/// deliberately the same "never trust an id/filename into a path" posture
/// the project plan calls for around archive extraction (zip-slip).
///
/// `pub(crate)` rather than private: `commands::instances::open_instance_folder`
/// needs the same validated path to hand to the opener plugin.
pub(crate) fn resolve_instance_dir(
    instances_dir: &Path,
    id: &str,
) -> Result<PathBuf, InstanceError> {
    if !is_valid_id(id) {
        return Err(InstanceError::InvalidId(id.to_string()));
    }
    let dir = instances_dir.join(id);
    if !dir.is_dir() {
        return Err(InstanceError::NotFound(id.to_string()));
    }
    Ok(dir)
}

fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ID_LEN
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// ASCII-folds a display name into a stable directory-safe id (`Výkonnost`
/// -> `profile`, `Fabric Performance!` -> `fabric-performance`). The
/// human-facing `name` field keeps full Unicode - only the internal id is
/// restricted, the same way a URL slug would be.
fn slugify(name: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = true; // avoid a leading dash
    for ch in name.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }
    let trimmed = slug.trim_end_matches('-');
    if trimmed.is_empty() {
        "profile".to_string()
    } else {
        trimmed.to_string()
    }
}

fn unique_id(instances_dir: &Path, name: &str) -> String {
    let base = slugify(name);
    if !instances_dir.join(&base).exists() {
        return base;
    }
    let mut n = 2;
    loop {
        let candidate = format!("{base}-{n}");
        if !instances_dir.join(&candidate).exists() {
            return candidate;
        }
        n += 1;
    }
}

fn read_meta(dir: &Path) -> Result<InstanceMeta, InstanceError> {
    let bytes = fs::read(dir.join(INSTANCE_METADATA_FILE))?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Temp-file-then-rename so a crash mid-write can never leave a truncated
/// `instance.json` in place of a good one.
fn write_meta(dir: &Path, meta: &InstanceMeta) -> Result<(), InstanceError> {
    let json = serde_json::to_vec_pretty(meta)?;
    let tmp_path = dir.join(format!("{INSTANCE_METADATA_FILE}.tmp"));
    fs::write(&tmp_path, json)?;
    fs::rename(&tmp_path, dir.join(INSTANCE_METADATA_FILE))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::instances::model::LoaderConfig;
    use crate::core::instances::model::LoaderKind;

    fn sample_input(name: &str) -> NewInstanceInput {
        NewInstanceInput {
            name: name.to_string(),
            minecraft_version: "1.21.1".to_string(),
            loader: LoaderConfig {
                kind: LoaderKind::Fabric,
                version: "latest".to_string(),
            },
        }
    }

    #[test]
    fn create_makes_the_standard_directory_structure_and_metadata() {
        let root = tempfile::tempdir().unwrap();
        let meta = create(root.path(), sample_input("Performance")).unwrap();

        assert_eq!(meta.id, "performance");
        assert_eq!(meta.name, "Performance");
        assert!(!meta.favorite);

        let dir = root.path().join("performance");
        assert!(dir.join("instance.json").is_file());
        for sub in INSTANCE_SUBDIRS {
            assert!(dir.join(sub).is_dir(), "missing {sub} subdirectory");
        }
    }

    #[test]
    fn create_rejects_an_empty_name() {
        let root = tempfile::tempdir().unwrap();
        let err = create(root.path(), sample_input("   ")).unwrap_err();
        assert!(matches!(err, InstanceError::InvalidName(_)));
    }

    #[test]
    fn create_avoids_id_collisions_for_repeated_names() {
        let root = tempfile::tempdir().unwrap();
        let first = create(root.path(), sample_input("Modded")).unwrap();
        let second = create(root.path(), sample_input("Modded")).unwrap();
        assert_eq!(first.id, "modded");
        assert_eq!(second.id, "modded-2");
    }

    #[test]
    fn create_slugifies_diacritics_by_dropping_them_but_keeps_the_ascii_letters() {
        let root = tempfile::tempdir().unwrap();
        // "Čeština" is mostly plain ASCII letters (e, t, i, n, a) with two
        // diacritic marks (Č, š) - those two get treated as separators,
        // same as any other non-ASCII character, but the ASCII letters
        // around them still survive into the slug.
        let meta = create(root.path(), sample_input("Čeština")).unwrap();
        assert_eq!(meta.id, "e-tina");
        // The display name itself must keep its real Unicode content.
        assert_eq!(meta.name, "Čeština");
    }

    #[test]
    fn create_falls_back_to_a_generic_id_for_fully_non_ascii_names() {
        let root = tempfile::tempdir().unwrap();
        let meta = create(root.path(), sample_input("測試")).unwrap();
        assert_eq!(meta.id, "profile");
        assert_eq!(meta.name, "測試");
    }

    #[test]
    fn list_returns_created_instances_sorted_by_name() {
        let root = tempfile::tempdir().unwrap();
        create(root.path(), sample_input("Zephyr")).unwrap();
        create(root.path(), sample_input("Alpha")).unwrap();

        let names: Vec<String> = list(root.path())
            .unwrap()
            .into_iter()
            .map(|m| m.name)
            .collect();
        assert_eq!(names, vec!["Alpha".to_string(), "Zephyr".to_string()]);
    }

    #[test]
    fn list_returns_empty_when_the_instances_directory_does_not_exist_yet() {
        let root = tempfile::tempdir().unwrap();
        let missing = root.path().join("does-not-exist");
        assert_eq!(list(&missing).unwrap(), Vec::new());
    }

    #[test]
    fn list_skips_a_corrupted_instance_but_still_returns_the_rest() {
        let root = tempfile::tempdir().unwrap();
        create(root.path(), sample_input("Healthy")).unwrap();

        let broken_dir = root.path().join("broken");
        fs::create_dir_all(&broken_dir).unwrap();
        fs::write(broken_dir.join("instance.json"), b"{ not valid json").unwrap();

        let names: Vec<String> = list(root.path())
            .unwrap()
            .into_iter()
            .map(|m| m.name)
            .collect();
        assert_eq!(names, vec!["Healthy".to_string()]);
    }

    #[test]
    fn update_changes_only_the_provided_fields() {
        let root = tempfile::tempdir().unwrap();
        let created = create(root.path(), sample_input("Original")).unwrap();

        let updated = update(
            root.path(),
            &created.id,
            InstanceUpdate {
                name: Some("Renamed".into()),
                favorite: Some(true),
                java: None,
            },
        )
        .unwrap();

        assert_eq!(updated.name, "Renamed");
        assert!(updated.favorite);
        assert_eq!(updated.java, created.java); // untouched field preserved

        let reloaded = read_meta(&root.path().join(&created.id)).unwrap();
        assert_eq!(reloaded, updated);
    }

    #[test]
    fn update_rejects_blanking_out_the_name() {
        let root = tempfile::tempdir().unwrap();
        let created = create(root.path(), sample_input("Original")).unwrap();
        let err = update(
            root.path(),
            &created.id,
            InstanceUpdate {
                name: Some("   ".into()),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(matches!(err, InstanceError::InvalidName(_)));
    }

    #[test]
    fn update_on_an_unknown_id_reports_not_found() {
        let root = tempfile::tempdir().unwrap();
        let err = update(root.path(), "missing", InstanceUpdate::default()).unwrap_err();
        assert!(matches!(err, InstanceError::NotFound(_)));
    }

    #[test]
    fn delete_removes_the_instance_directory() {
        let root = tempfile::tempdir().unwrap();
        let created = create(root.path(), sample_input("Temp")).unwrap();
        assert!(root.path().join(&created.id).exists());

        delete(root.path(), &created.id).unwrap();
        assert!(!root.path().join(&created.id).exists());
    }

    #[test]
    fn delete_on_an_unknown_id_reports_not_found_instead_of_touching_disk() {
        let root = tempfile::tempdir().unwrap();
        let err = delete(root.path(), "does-not-exist").unwrap_err();
        assert!(matches!(err, InstanceError::NotFound(_)));
    }

    #[test]
    fn path_traversal_attempts_are_rejected_before_touching_the_filesystem() {
        let root = tempfile::tempdir().unwrap();
        // Something outside the instances root that a naive join() would
        // happily reach if the id were trusted as-is.
        let escape_target = root.path().parent().unwrap().join("evil-marker");
        fs::write(&escape_target, b"should never be deleted").unwrap();

        for malicious_id in [
            "../evil-marker",
            "..\\evil-marker",
            "a/b",
            "a\\b",
            "..",
            ".",
        ] {
            let err = delete(root.path(), malicious_id).unwrap_err();
            assert!(
                matches!(err, InstanceError::InvalidId(_)),
                "id: {malicious_id}"
            );
        }

        assert!(
            escape_target.exists(),
            "path traversal must not delete files outside instances_dir"
        );
    }

    #[test]
    fn is_valid_id_accepts_slugs_and_rejects_everything_path_like() {
        assert!(is_valid_id("performance"));
        assert!(is_valid_id("fabric-performance_2"));
        assert!(!is_valid_id(""));
        assert!(!is_valid_id(".."));
        assert!(!is_valid_id("a/b"));
        assert!(!is_valid_id("a\\b"));
        assert!(!is_valid_id(&"a".repeat(MAX_ID_LEN + 1)));
    }
}
