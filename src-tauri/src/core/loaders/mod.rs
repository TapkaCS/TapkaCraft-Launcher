//! Mod loader support - install and launch data on top of a Vanilla base
//! version (`core::versions`). Only Fabric is implemented so far; Quilt,
//! Forge and NeoForge instances can be created in the UI but not yet
//! installed or launched (see `LoaderKind` in `core::instances::model`).

use std::fmt;

pub mod fabric;

#[derive(Debug)]
pub enum LoaderInstallError {
    Network(String),
    Parse(String),
    Io(std::io::Error),
    /// The loader's meta service doesn't publish a build for this
    /// Minecraft version at all.
    NoLoaderForGameVersion {
        game_version: String,
    },
    /// A specific loader version was requested (not "latest") and isn't
    /// one the meta service knows about.
    UnknownLoaderVersion {
        loader_version: String,
    },
    DownloadFailed {
        failed: usize,
        total: usize,
    },
}

impl fmt::Display for LoaderInstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(msg) => write!(f, "Network error installing the mod loader: {msg}"),
            Self::Parse(msg) => write!(f, "Couldn't understand the mod loader's data: {msg}"),
            Self::Io(err) => write!(f, "File error installing the mod loader: {err}"),
            Self::NoLoaderForGameVersion { game_version } => write!(
                f,
                "No loader build is published for Minecraft {game_version}."
            ),
            Self::UnknownLoaderVersion { loader_version } => {
                write!(f, "Loader version \"{loader_version}\" was not found.")
            }
            Self::DownloadFailed { failed, total } => {
                write!(
                    f,
                    "{failed} of {total} mod loader files failed to download."
                )
            }
        }
    }
}

impl std::error::Error for LoaderInstallError {}

impl From<std::io::Error> for LoaderInstallError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl serde::Serialize for LoaderInstallError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
