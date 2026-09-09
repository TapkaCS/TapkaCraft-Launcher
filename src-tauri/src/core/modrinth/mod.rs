//! `ModrinthService` (Phase 7).
//!
//! Client for the public Modrinth API: search/browse mods; project version
//! metadata filtered to what an instance's Minecraft version and loader can
//! actually use; installing a chosen version's file (hash-verified, unlike
//! Fabric's meta API Modrinth publishes real sha1 hashes) alongside its
//! required dependencies, resolved recursively. Resource packs, shaders and
//! modpacks share the same API shape but aren't wired up yet - only the
//! `mod` project type, installed into an instance's `mods/` directory.

use std::fmt;

pub mod api;
pub mod install;

#[derive(Debug)]
pub enum ModrinthError {
    Network(String),
    Parse(String),
    Io(std::io::Error),
    DownloadFailed { failed: usize, total: usize },
}

impl fmt::Display for ModrinthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(msg) => write!(f, "Network error talking to Modrinth: {msg}"),
            Self::Parse(msg) => write!(f, "Couldn't understand Modrinth's response: {msg}"),
            Self::Io(err) => write!(f, "File error installing a mod: {err}"),
            Self::DownloadFailed { failed, total } => {
                write!(f, "{failed} of {total} mod files failed to download.")
            }
        }
    }
}

impl std::error::Error for ModrinthError {}

impl From<std::io::Error> for ModrinthError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl serde::Serialize for ModrinthError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Modrinth's identifier for a loader, as used in both search facets and
/// the project-version filter - `None` for `Vanilla` (and the loaders this
/// project doesn't install yet), since Modrinth has no "vanilla" mod
/// loader to filter by.
pub fn loader_identifier(kind: crate::core::instances::model::LoaderKind) -> Option<&'static str> {
    use crate::core::instances::model::LoaderKind;
    match kind {
        LoaderKind::Fabric => Some("fabric"),
        LoaderKind::Quilt => Some("quilt"),
        LoaderKind::Forge => Some("forge"),
        LoaderKind::Neoforge => Some("neoforge"),
        LoaderKind::Vanilla => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::instances::model::LoaderKind;

    #[test]
    fn loader_identifier_covers_every_installable_loader() {
        assert_eq!(loader_identifier(LoaderKind::Fabric), Some("fabric"));
        assert_eq!(loader_identifier(LoaderKind::Quilt), Some("quilt"));
        assert_eq!(loader_identifier(LoaderKind::Forge), Some("forge"));
        assert_eq!(loader_identifier(LoaderKind::Neoforge), Some("neoforge"));
        assert_eq!(loader_identifier(LoaderKind::Vanilla), None);
    }
}
