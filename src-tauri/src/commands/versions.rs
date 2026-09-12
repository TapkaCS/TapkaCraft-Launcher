//! Tauri IPC wrappers around `core::versions`: the Mojang release list for
//! pickers, and the Vanilla install pipeline - which streams progress back
//! as `install://progress` events rather than blocking silently until done.

use std::fmt;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::core::downloads::{DownloadEvent, ProgressSink};
use crate::core::instances::model::{InstanceMeta, LoaderKind};
use crate::core::instances::service::{self, InstanceError};
use crate::core::loaders::fabric;
use crate::core::loaders::LoaderInstallError;
use crate::core::modpacks::{self, ModpackError};
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
    Loader(LoaderInstallError),
    Modpack(ModpackError),
    UnsupportedLoader { loader: &'static str },
}

impl fmt::Display for InstallCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Instance(err) => write!(f, "{err}"),
            Self::Version(err) => write!(f, "{err}"),
            Self::Loader(err) => write!(f, "{err}"),
            Self::Modpack(err) => write!(f, "{err}"),
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

impl From<LoaderInstallError> for InstallCommandError {
    fn from(err: LoaderInstallError) -> Self {
        Self::Loader(err)
    }
}

impl From<ModpackError> for InstallCommandError {
    fn from(err: ModpackError) -> Self {
        Self::Modpack(err)
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
/// Minecraft version: client jar, libraries, natives, assets, and (Fabric
/// instances) the loader itself. Emits `install://progress` (`DownloadEvent`)
/// throughout; safe to call again on the same instance later -
/// already-correct files are skipped, not re-downloaded. `concurrency` is
/// the frontend's "Concurrent downloads" setting, passed through rather
/// than decided here.
///
/// A modpack-created instance (see `commands::modpacks::import_modpack_file`)
/// keeps its source `.mrpack` inside its own directory specifically so this
/// command can find it here and also (re-)install every file the pack
/// declares plus its overrides - the same "call it again to resume" safety
/// net every other install path in this app already has, now covering
/// modpacks too instead of leaving a failed pack import to `import`'s own
/// one-shot run.
#[tauri::command]
pub async fn install_instance(
    app: AppHandle,
    app_paths: State<'_, AppPaths>,
    http_client: State<'_, reqwest::Client>,
    id: String,
    concurrency: usize,
) -> Result<(), InstallCommandError> {
    let instance = service::get(&app_paths.instances_dir(), &id)?;
    let client = http_client.inner().clone();
    install_vanilla_and_loader(&app, &app_paths, &client, &instance, concurrency).await?;

    let instance_dir = service::resolve_instance_dir(&app_paths.instances_dir(), &id)?;
    let mrpack_path = instance_dir.join(modpacks::MODPACK_FILE);
    if mrpack_path.is_file() {
        crate::commands::modpacks::install_pack_files_and_overrides(
            &app,
            &client,
            &instance_dir,
            concurrency,
        )
        .await?;
    }

    Ok(())
}

/// The Vanilla-base-plus-optional-Fabric-loader half of installing an
/// instance, shared by `install_instance` and modpack import
/// (`commands::modpacks::import_modpack_file`) so both go through the exact
/// same, already-tested pipeline instead of each growing their own.
pub(crate) async fn install_vanilla_and_loader(
    app: &AppHandle,
    app_paths: &AppPaths,
    client: &reqwest::Client,
    instance: &InstanceMeta,
    concurrency: usize,
) -> Result<(), InstallCommandError> {
    if instance.loader.kind != LoaderKind::Vanilla && instance.loader.kind != LoaderKind::Fabric {
        return Err(InstallCommandError::UnsupportedLoader {
            loader: loader_label(instance.loader.kind),
        });
    }

    let manifest = manifest::fetch_or_cached(client, MANIFEST_URL, &app_paths.cache_dir()).await?;
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
        client,
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
    result?;

    if instance.loader.kind == LoaderKind::Fabric {
        let (fabric_tx, mut fabric_rx) = tokio::sync::mpsc::unbounded_channel::<DownloadEvent>();
        let fabric_forward_app = app.clone();
        let fabric_forwarder = tokio::spawn(async move {
            while let Some(event) = fabric_rx.recv().await {
                let _ = fabric_forward_app.emit(INSTALL_PROGRESS_EVENT, event);
            }
        });
        let fabric_result = install_fabric(
            client,
            &paths.libraries_dir,
            &instance.minecraft_version,
            &instance.loader.version,
            concurrency,
            ProgressSink::new(fabric_tx),
        )
        .await;
        let _ = fabric_forwarder.await;
        fabric_result?;
    }

    Ok(())
}

/// Resolves the requested Fabric loader version (`"latest"` or an exact
/// version) for `game_version` and downloads its extra libraries -
/// `install_instance`'s own Fabric-specific step, after the Vanilla base is
/// already in place.
async fn install_fabric(
    client: &reqwest::Client,
    libraries_dir: &std::path::Path,
    game_version: &str,
    requested_loader_version: &str,
    concurrency: usize,
    events: ProgressSink,
) -> Result<(), LoaderInstallError> {
    fabric::resolve_and_install(
        client,
        fabric::FABRIC_META_URL,
        libraries_dir,
        game_version,
        requested_loader_version,
        concurrency,
        events,
    )
    .await
    .map(|_profile| ())
}
