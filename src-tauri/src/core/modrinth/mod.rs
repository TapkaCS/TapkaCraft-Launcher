//! `ModrinthService` (Phase 7, generalized in Phase 9d).
//!
//! Client for the public Modrinth API: search/browse content; project
//! version metadata filtered to what an instance's Minecraft version (and,
//! for mods specifically, loader) can actually use; installing a chosen
//! version's file (hash-verified, unlike Fabric's meta API Modrinth
//! publishes real sha1 hashes) alongside its required dependencies,
//! resolved recursively. Modpacks share the same search API shape but are
//! a separate installer (`core::modpacks`) - they create a whole new
//! instance rather than installing into an existing one's subdirectory.

use std::fmt;

pub mod api;
pub mod install;

/// The three single-file content types this launcher can browse on
/// Modrinth and drop straight into an existing instance - modpacks are
/// deliberately not here; installing one creates a whole new instance
/// through `core::modpacks` instead of adding a file to one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContentKind {
    Mod,
    Resourcepack,
    Shader,
}

impl ContentKind {
    pub fn project_type(self) -> &'static str {
        match self {
            Self::Mod => "mod",
            Self::Resourcepack => "resourcepack",
            Self::Shader => "shader",
        }
    }

    /// Where a version's file lands inside an instance directory - the
    /// same folder names vanilla Minecraft itself already reads from, so
    /// nothing extra is needed for the game to pick these up.
    pub fn install_subdir(self) -> &'static str {
        match self {
            Self::Mod => "mods",
            Self::Resourcepack => "resourcepacks",
            Self::Shader => "shaderpacks",
        }
    }

    /// Only mods are tied to a specific mod loader (Fabric/Forge/...).
    /// Resource packs and shaders are plain assets vanilla Minecraft loads
    /// by itself - filtering or resolving them by a mod loader facet would
    /// just incorrectly exclude real ones, which tag themselves by
    /// whatever renderer they target (e.g. Iris/OptiFine for shaders), not
    /// by mod loader.
    pub fn uses_loader_facet(self) -> bool {
        matches!(self, Self::Mod)
    }
}

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
