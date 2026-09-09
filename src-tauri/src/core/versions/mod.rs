//! `VersionService`: Mojang version manifest + per-version metadata
//! resolution (OS/arch rules, libraries, natives, asset index), and the
//! Vanilla install pipeline built on top of it. Loader installers
//! (Fabric/Forge/Quilt/NeoForge) are a later phase; this module only
//! knows about Vanilla.

use std::fmt;

pub mod assets;
pub mod install;
pub mod manifest;
pub mod rules;
pub mod version_json;

#[derive(Debug)]
pub enum VersionError {
    Network(String),
    Parse(String),
    NotFound(String),
    Io(std::io::Error),
    HashMismatch {
        expected: String,
        actual: String,
        file: String,
    },
}

impl fmt::Display for VersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(msg) => write!(f, "Network error while fetching version data: {msg}"),
            Self::Parse(msg) => write!(f, "Couldn't parse Mojang version data: {msg}"),
            Self::NotFound(id) => write!(f, "Minecraft version \"{id}\" was not found."),
            Self::Io(err) => write!(f, "File error while installing: {err}"),
            Self::HashMismatch {
                expected,
                actual,
                file,
            } => write!(
                f,
                "Downloaded file \"{file}\" is corrupted (expected sha1 {expected}, got {actual})."
            ),
        }
    }
}

impl std::error::Error for VersionError {}

impl From<std::io::Error> for VersionError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl serde::Serialize for VersionError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
