//! Tauri IPC wrappers around `core::versions`: the Mojang release list for
//! pickers, and the Vanilla install pipeline - which streams progress back
//! as `install://progress` events rather than blocking silently until done.

use std::fmt;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::core::downloads::{DownloadEvent, ProgressSink};
use crate::core::instances::model::LoaderKind;
use crate::core::instances::service::{self, InstanceError};
use crate::core::paths::AppPaths;
use crate::core::versions::install::{ensure_installed, InstallPaths};
use crate::core::versions::manifest::{self, MANIFEST_URL};
use crate::core::versions::VersionError;

const INSTALL_PROGRESS_EVENT: &str = "install://progress";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionSummary {
    pub id: String,
    pub release_time: String,
}

#[derive(Debug)]
pub enum InstallCommandError {
    Instance(InstanceError),
    Version(VersionError),
    UnsupportedLoader { loader: &'static str },
}

impl fmt::Display for InstallCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Instance(err) => write!(f, "{err}"),
            Self::Version(err) => write!(f, "{err}"),
            Self::UnsupportedLoader { loader } => write!(
                f,
                "Installing {loader} instances isn't supported yet - only Vanilla can be installed so far."
            ),
        }
    }
}

impl std::error::Error for InstallCommandError {}

impl From<InstanceError> for InstallCommandError {
    fn from(err: InstanceError) -> Self {
        Self::Instance(err)
    }
}

impl From<VersionError> for InstallCommandError {
    fn from(err: VersionError) -> Self {
        Self::Version(err)
    }
}

impl Serialize for InstallCommandError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

fn loader_label(kind: LoaderKind) -> &'static str {
    match kind {
        LoaderKind::Vanilla => "Vanilla",
        LoaderKind::Fabric => "Fabric",
        LoaderKind::Quilt => "Quilt",
        LoaderKind::Forge => "Forge",
        LoaderKind::Neoforge => "NeoForge",
    }
}

/// Every release version Mojang has published, newest first - live if
/// reachable, the last successful fetch's cache otherwise.
#[tauri::command]
pub async fn get_version_manifest(
    app_paths: State<'_, AppPaths>,
    http_client: State<'_, reqwest::Client>,
) -> Result<Vec<VersionSummary>, VersionError> {
    let client = http_client.inner().clone();
    let cache_dir = app_paths.cache_dir();
    let manifest = manifest::fetch_or_cached(&client, MANIFEST_URL, &cache_dir).await?;
    Ok(manifest
        .releases()
        .into_iter()
        .map(|entry| VersionSummary {
            id: entry.id.clone(),
            release_time: entry.release_time.clone(),
        })
        .collect())
}

/// Downloads (or verifies an already-complete install of) an instance's
/// Minecraft version: client jar, libraries, natives, assets. Emits
/// `install://progress` (`DownloadEvent`) throughout; safe to call again on
/// the same instance later - already-correct files are skipped, not
/// re-downloaded. `concurrency` is the frontend's "Concurrent downloads"
/// setting, passed through rather than decided here.
#[tauri::command]
pub async fn install_instance(
    app: AppHandle,
    app_paths: State<'_, AppPaths>,
    http_client: State<'_, reqwest::Client>,
    id: String,
    concurrency: usize,
) -> Result<(), InstallCommandError> {
    let instance = service::get(&app_paths.instances_dir(), &id)?;
    if instance.loader.kind != LoaderKind::Vanilla {
        return Err(InstallCommandError::UnsupportedLoader {
            loader: loader_label(instance.loader.kind),
        });
    }

    let client = http_client.inner().clone();
    let manifest = manifest::fetch_or_cached(&client, MANIFEST_URL, &app_paths.cache_dir()).await?;
    let paths = InstallPaths {
        versions_dir: app_paths.versions_dir(),
        libraries_dir: app_paths.libraries_dir(),
        assets_dir: app_paths.assets_dir(),
    };

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<DownloadEvent>();
    let forward_app = app.clone();
    let forwarder = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let _ = forward_app.emit(INSTALL_PROGRESS_EVENT, event);
        }
    });

    let result = ensure_installed(
        &client,
        &paths,
        &manifest,
        &instance.minecraft_version,
        concurrency,
        ProgressSink::new(tx),
    )
    .await;

    // Dropping `ProgressSink` above (its sender went out of scope with the
    // `ensure_installed` call) lets `forwarder`'s receive loop end on its
    // own once every already-queued event is flushed - awaiting it here
    // just makes sure that flush finishes before the command returns.
    let _ = forwarder.await;
    result.map(|_| ()).map_err(InstallCommandError::from)
}
