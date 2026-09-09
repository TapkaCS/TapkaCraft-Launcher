//! Tauri IPC wrapper around `core::modrinth`: search/browse are generic
//! (loader + game version, not tied to a specific instance - useful for
//! browsing before an instance is even selected), while installing a mod
//! is always against a specific instance so its loader can be validated.

use std::fmt;

use tauri::{AppHandle, Emitter, State};

use crate::core::downloads::{DownloadEvent, ProgressSink};
use crate::core::instances::model::LoaderKind;
use crate::core::instances::service::{self as instance_service, InstanceError};
use crate::core::modrinth::api::{self, ModrinthVersion, SearchResponse};
use crate::core::modrinth::install::{self, InstalledMod, InstalledMods};
use crate::core::modrinth::{loader_identifier, ModrinthError};
use crate::core::paths::AppPaths;

const INSTALL_PROGRESS_EVENT: &str = "install://progress";

#[derive(Debug)]
pub enum ModrinthCommandError {
    Instance(InstanceError),
    Modrinth(ModrinthError),
    UnsupportedLoader { loader: &'static str },
}

impl fmt::Display for ModrinthCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Instance(err) => write!(f, "{err}"),
            Self::Modrinth(err) => write!(f, "{err}"),
            Self::UnsupportedLoader { loader } => write!(
                f,
                "Installing mods requires a Fabric instance - {loader} isn't supported yet."
            ),
        }
    }
}

impl std::error::Error for ModrinthCommandError {}

impl From<InstanceError> for ModrinthCommandError {
    fn from(err: InstanceError) -> Self {
        Self::Instance(err)
    }
}

impl From<ModrinthError> for ModrinthCommandError {
    fn from(err: ModrinthError) -> Self {
        Self::Modrinth(err)
    }
}

impl serde::Serialize for ModrinthCommandError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallModOutcome {
    pub installed: Vec<InstalledMod>,
    pub skipped_dependencies: Vec<String>,
}

/// Mods compatible with `loader`/`game_version` matching `query` (which may
/// be empty for Modrinth's default-sorted browse listing).
#[tauri::command]
pub async fn search_mods(
    http_client: State<'_, reqwest::Client>,
    query: String,
    loader: String,
    game_version: String,
) -> Result<SearchResponse, ModrinthCommandError> {
    api::search(
        http_client.inner(),
        api::MODRINTH_API_URL,
        &query,
        &loader,
        &game_version,
        20,
    )
    .await
    .map_err(ModrinthCommandError::from)
}

/// Every version of `project_id` compatible with `loader`/`game_version`.
#[tauri::command]
pub async fn list_mod_versions(
    http_client: State<'_, reqwest::Client>,
    project_id: String,
    loader: String,
    game_version: String,
) -> Result<Vec<ModrinthVersion>, ModrinthCommandError> {
    api::list_project_versions(
        http_client.inner(),
        api::MODRINTH_API_URL,
        &project_id,
        &loader,
        &game_version,
    )
    .await
    .map_err(ModrinthCommandError::from)
}

/// Mods this launcher has installed into `id`'s `mods/` directory
/// (best-effort local record - see `core::modrinth::install::read_installed`).
#[tauri::command]
pub fn list_installed_mods(
    app_paths: State<AppPaths>,
    id: String,
) -> Result<InstalledMods, ModrinthCommandError> {
    let dir = instance_service::resolve_instance_dir(&app_paths.instances_dir(), &id)?;
    Ok(install::read_installed(&dir))
}

/// Installs `version` (and its resolvable required dependencies) into
/// instance `id`'s `mods/` directory. Only Fabric instances are supported
/// so far, matching what `install_instance`/`launch_instance` can actually
/// install and launch.
#[tauri::command]
pub async fn install_mod(
    app: AppHandle,
    app_paths: State<'_, AppPaths>,
    http_client: State<'_, reqwest::Client>,
    id: String,
    version: ModrinthVersion,
    concurrency: usize,
) -> Result<InstallModOutcome, ModrinthCommandError> {
    let instance = instance_service::get(&app_paths.instances_dir(), &id)?;
    if instance.loader.kind != LoaderKind::Fabric {
        return Err(ModrinthCommandError::UnsupportedLoader {
            loader: loader_label(instance.loader.kind),
        });
    }
    let loader = loader_identifier(instance.loader.kind).expect(
        "just checked instance.loader.kind == LoaderKind::Fabric, which always maps to Some",
    );

    let instance_dir = instance_service::resolve_instance_dir(&app_paths.instances_dir(), &id)?;
    let client = http_client.inner().clone();

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<DownloadEvent>();
    let forward_app = app.clone();
    let forwarder = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let _ = forward_app.emit(INSTALL_PROGRESS_EVENT, event);
        }
    });

    let target = install::CompatibilityTarget {
        base_url: api::MODRINTH_API_URL,
        loader,
        game_version: &instance.minecraft_version,
    };
    let result = install::install_version(
        &client,
        &target,
        &instance_dir,
        version,
        concurrency,
        ProgressSink::new(tx),
    )
    .await;
    let _ = forwarder.await;
    let result = result?;

    Ok(InstallModOutcome {
        installed: result.installed,
        skipped_dependencies: result.skipped_dependencies,
    })
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
