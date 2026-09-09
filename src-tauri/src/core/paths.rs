//! Resolves where TapkaCraft Launcher keeps its data.
//!
//! Windows is the primary target, so this always goes through the `directories`
//! crate's platform-aware special folders rather than joining path segments
//! by hand:
//!
//! - "roaming" (`%APPDATA%\TapkaCraft\TapkaCraft Launcher` on Windows) holds
//!   lightweight, sync-friendly state: settings, account metadata, instance
//!   metadata.
//! - "local" (`%LOCALAPPDATA%\TapkaCraft\TapkaCraft Launcher` on Windows)
//!   holds large/rebuildable data: download cache, managed Java runtimes,
//!   shared libraries/assets, temp downloads.
//!
//! In portable mode (a `portable.flag` file next to the executable, or an
//! explicit `--portable` argument) both collapse to a single `data/`
//! directory next to the executable instead, and neither AppData location
//! is touched.
//!
//! None of this is wired into a Tauri command yet - Phase 1 has no caller
//! for it - but it is real, working, unit-tested path-resolution logic that
//! `core::instances`, `core::java` and `core::downloads` will build on.

// Not wired into a Tauri command yet (see the module doc comment above) - the
// only caller so far is this file's own test module, which trips `dead_code`
// under a plain `cargo build`. Remove this once `core::instances`/`core::java`
// start calling into `AppPaths`.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use directories::ProjectDirs;

const PORTABLE_FLAG_FILE: &str = "portable.flag";
const PORTABLE_ARG: &str = "--portable";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    /// Settings, account metadata, instance metadata.
    roaming: PathBuf,
    /// Download cache, managed runtimes, shared libraries/assets, temp files.
    local: PathBuf,
    portable: bool,
}

impl AppPaths {
    /// Resolves the launcher's data locations given the running
    /// executable's directory and the process's CLI arguments.
    ///
    /// Never panics: if the OS cannot report a home/AppData directory (rare,
    /// but possible in stripped-down environments), falls back to a
    /// `./tapkacraft-data` directory relative to the executable rather than
    /// aborting startup.
    pub fn resolve(exe_dir: &Path, args: &[String]) -> Self {
        if let Some(root) = portable_root(exe_dir, args) {
            return Self {
                roaming: root.clone(),
                local: root,
                portable: true,
            };
        }

        match ProjectDirs::from("cz", "TapkaCraft", "TapkaCraft Launcher") {
            Some(dirs) => Self {
                roaming: dirs.config_dir().to_path_buf(),
                local: dirs.data_local_dir().to_path_buf(),
                portable: false,
            },
            None => {
                let fallback = exe_dir.join("tapkacraft-data");
                Self {
                    roaming: fallback.clone(),
                    local: fallback,
                    portable: false,
                }
            }
        }
    }

    pub fn is_portable(&self) -> bool {
        self.portable
    }

    /// Lightweight settings, account metadata, launcher preferences.
    pub fn settings_dir(&self) -> PathBuf {
        self.roaming.clone()
    }

    /// Instance root - each instance is its own subdirectory (id-named) with
    /// its own config/mods/saves/resourcepacks/shaderpacks/screenshots.
    pub fn instances_dir(&self) -> PathBuf {
        self.roaming.join("instances")
    }

    /// Shared, deduplicated across instances.
    pub fn assets_dir(&self) -> PathBuf {
        self.local.join("assets")
    }

    /// Shared, deduplicated across instances.
    pub fn libraries_dir(&self) -> PathBuf {
        self.local.join("libraries")
    }

    /// Shared, deduplicated across instances.
    pub fn versions_dir(&self) -> PathBuf {
        self.local.join("versions")
    }

    /// TapkaCraft-managed Java runtimes (e.g. `runtimes/java21/`). Never the
    /// user's system Java installation.
    pub fn runtimes_dir(&self) -> PathBuf {
        self.local.join("runtimes")
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.local.join("cache")
    }

    pub fn temp_downloads_dir(&self) -> PathBuf {
        self.local.join("temp")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.roaming.join("logs")
    }
}

fn portable_root(exe_dir: &Path, args: &[String]) -> Option<PathBuf> {
    if portable_flag_present(exe_dir) || has_portable_arg(args) {
        Some(exe_dir.join("data"))
    } else {
        None
    }
}

fn portable_flag_present(exe_dir: &Path) -> bool {
    exe_dir.join(PORTABLE_FLAG_FILE).is_file()
}

fn has_portable_arg(args: &[String]) -> bool {
    args.iter().any(|a| a == PORTABLE_ARG)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_portable_flag_file() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!portable_flag_present(dir.path()));

        std::fs::write(dir.path().join(PORTABLE_FLAG_FILE), b"").unwrap();
        assert!(portable_flag_present(dir.path()));
    }

    #[test]
    fn detects_portable_cli_arg() {
        assert!(!has_portable_arg(&["tapkacraft.exe".to_string()]));
        assert!(has_portable_arg(&[
            "tapkacraft.exe".to_string(),
            "--portable".to_string()
        ]));
    }

    #[test]
    fn portable_mode_keeps_everything_next_to_the_executable() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(PORTABLE_FLAG_FILE), b"").unwrap();

        let paths = AppPaths::resolve(dir.path(), &[]);

        assert!(paths.is_portable());
        assert_eq!(paths.instances_dir(), dir.path().join("data/instances"));
        assert_eq!(paths.runtimes_dir(), dir.path().join("data/runtimes"));
        // Roaming and local collapse to the same portable root.
        assert_eq!(paths.settings_dir(), dir.path().join("data"));
    }

    #[test]
    fn standard_mode_does_not_use_the_portable_root() {
        let dir = tempfile::tempdir().unwrap();
        // No portable.flag, no --portable arg.
        let paths = AppPaths::resolve(dir.path(), &[]);

        assert!(!paths.is_portable());
        assert!(!paths.instances_dir().starts_with(dir.path()));
    }

    #[test]
    fn instance_and_runtime_data_are_kept_in_separate_roots_when_not_portable() {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::resolve(dir.path(), &[]);

        // Roaming (instances/settings) and local (runtimes/cache/assets) must
        // never collapse to the same directory outside portable mode - that
        // is the whole point of the split described in the Windows storage
        // requirements.
        assert_ne!(
            paths.instances_dir().parent(),
            paths.runtimes_dir().parent()
        );
    }
}
