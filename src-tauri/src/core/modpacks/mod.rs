//! `.mrpack` modpack import: parses a Modrinth-format modpack (a zip
//! containing `modrinth.index.json` plus `overrides/`), creates a fresh
//! instance for it, and installs everything the index declares - the
//! Minecraft version, the loader, every listed file, and the override
//! files. A modpack downloaded from Modrinth and one imported from a local
//! file are the same format once they're bytes on disk, so both paths
//! converge on the same parse/install code here; only how the `.mrpack`
//! bytes were obtained differs, and that lives in `commands::modpacks`.

pub mod index;
pub mod install;

use std::fmt;

/// The `.mrpack` copy kept inside an installed instance's own directory -
/// both a record of what it was installed from, and what lets
/// `commands::versions::install_instance` resume a modpack install that
/// failed partway (the same "call it again, already-correct files are
/// skipped" resumability every other install path in this app already
/// has).
pub const MODPACK_FILE: &str = "modpack.mrpack";

#[derive(Debug)]
pub enum ModpackError {
    Io(std::io::Error),
    Zip(String),
    Parse(String),
    MissingMinecraftVersion,
    UnsupportedLoader { loader: &'static str },
    DownloadFailed(String),
}

impl fmt::Display for ModpackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "Modpack file error: {err}"),
            Self::Zip(msg) => write!(f, "\"{msg}\" isn't a valid .mrpack file."),
            Self::Parse(msg) => write!(f, "Modpack manifest is invalid: {msg}"),
            Self::MissingMinecraftVersion => write!(
                f,
                "This modpack's manifest doesn't declare a Minecraft version."
            ),
            Self::UnsupportedLoader { loader } => write!(
                f,
                "This modpack needs {loader}, which isn't supported yet - only Vanilla and Fabric modpacks can be installed so far."
            ),
            Self::DownloadFailed(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ModpackError {}

impl From<std::io::Error> for ModpackError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl serde::Serialize for ModpackError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
