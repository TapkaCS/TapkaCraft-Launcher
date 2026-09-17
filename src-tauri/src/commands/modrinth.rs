//! Tauri IPC wrapper around `core::modrinth`: search/browse are generic
//! (loader + game version, not tied to a specific instance - useful for
//! browsing before an instance is even selected), while installing
//! content is always against a specific instance so its loader can be
//! validated (mods only - see `ContentKind::uses_loader_facet`).

use std::fmt;

use tauri::{AppHandle, Emitter, State};

use crate::core::downloads::{DownloadEvent, ProgressSink};
use crate::core::instances::model::LoaderKind;
use crate::core::instances::service::{self as instance_service, InstanceError};
use crate::core::modrinth::api::{self, ModrinthVersion, SearchResponse};
use crate::core::modrinth::install::{self, InstalledContent, InstalledContentList};
use crate::core::modrinth::{loader_identifier, ContentKind, ModrinthError};
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
pub struct InstallContentOutcome {
    pub installed: Vec<InstalledContent>,
    pub skipped_dependencies: Vec<String>,
}

/// Content of `content_kind` compatible with `game_version` (and `loader`,
/// for mods - see `ContentKind::uses_loader_facet`) matching `query`
/// (which may be empty for Modrinth's default-sorted browse listing).
#[tauri::command]
pub async fn search_content(
    http_client: State<'_, reqwest::Client>,
    query: String,
    content_kind: ContentKind,
    loader: String,
    game_version: String,
) -> Result<SearchResponse, ModrinthCommandError> {
    let loader_facet = content_kind.uses_loader_facet().then(|| loader.as_str());
    api::search(
        http_client.inner(),
        api::MODRINTH_API_URL,
        content_kind.project_type(),
        &query,
        loader_facet,
        &game_version,
        20,
    )
    .await
    .map_err(ModrinthCommandError::from)
}

/// Every version of `project_id` compatible with `game_version` (and
/// `loader`, for mods).
#[tauri::command]
pub async fn list_content_versions(
    http_client: State<'_, reqwest::Client>,
    project_id: String,
    content_kind: ContentKind,
    loader: String,
    game_version: String,
) -> Result<Vec<ModrinthVersion>, ModrinthCommandError> {
    let loader_facet = content_kind.uses_loader_facet().then(|| loader.as_str());
    api::list_project_versions(
        http_client.inner(),
        api::MODRINTH_API_URL,
        &project_id,
        loader_facet,
        &game_version,
    )
    .await
    .map_err(ModrinthCommandError::from)
}

/// Content of `content_kind` this launcher has installed into `id`'s
/// content-kind-appropriate directory (best-effort local record - see
/// `core::modrinth::install::read_installed`).
#[tauri::command]
pub fn list_installed_content(
    app_paths: State<AppPaths>,
    id: String,
    content_kind: ContentKind,
) -> Result<InstalledContentList, ModrinthCommandError> {
    let dir = instance_service::resolve_instance_dir(&app_paths.instances_dir(), &id)?;
    Ok(install::read_installed(&dir, content_kind))
}

/// Installs `version` (and its resolvable required dependencies) into
/// instance `id`'s content-kind-appropriate directory. Mods specifically
/// only support Fabric instances so far, matching what
/// `install_instance`/`launch_instance` can actually install and launch;
/// resource packs and shaders aren't tied to a loader at all, so they
/// install into any instance.
#[tauri::command]
pub async fn install_content(
    app: AppHandle,
    app_paths: State<'_, AppPaths>,
    http_client: State<'_, reqwest::Client>,
    id: String,
    content_kind: ContentKind,
    version: ModrinthVersion,
    concurrency: usize,
) -> Result<InstallContentOutcome, ModrinthCommandError> {
    let instance = instance_service::get(&app_paths.instances_dir(), &id)?;

    let loader = if content_kind.uses_loader_facet() {
        if instance.loader.kind != LoaderKind::Fabric {
            return Err(ModrinthCommandError::UnsupportedLoader {
                loader: loader_label(instance.loader.kind),
            });
        }
        loader_identifier(instance.loader.kind)
    } else {
        None
    };

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
        content_kind,
        version,
        concurrency,
        ProgressSink::new(tx),
    )
    .await;
    let _ = forwarder.await;
    let result = result?;

    Ok(InstallContentOutcome {
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
