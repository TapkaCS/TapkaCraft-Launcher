//! Tauri IPC wrapper around `core::modpacks`: browsing modpacks hosted on
//! Modrinth reuses `core::modrinth::api` almost unchanged (a modpack is
//! just another Modrinth project type). Installing one converges on the
//! same `import_and_install` pipeline regardless of whether its `.mrpack`
//! came from a Modrinth download or a local file the user picked, since
//! it's the same format either way once the bytes are on disk.

use std::fmt;
use std::fs;
use std::path::Path;

use tauri::{AppHandle, Emitter, State};

use crate::core::downloads::{download_all, DownloadEvent, DownloadTask, ProgressSink};
use crate::core::instances::model::{InstanceMeta, LoaderConfig, NewInstanceInput};
use crate::core::instances::service::{self as instance_service, InstanceError};
use crate::core::modpacks::{
    self, index as modpack_index, install as modpack_install, ModpackError,
};
use crate::core::modrinth::api::{self, ModrinthVersion, SearchResponse};
use crate::core::modrinth::ModrinthError;
use crate::core::paths::AppPaths;

use super::versions::{install_vanilla_and_loader, InstallCommandError};

const INSTALL_PROGRESS_EVENT: &str = "install://progress";

#[derive(Debug)]
pub enum ModpackCommandError {
    Instance(InstanceError),
    Modpack(ModpackError),
    Install(InstallCommandError),
    Modrinth(ModrinthError),
}

impl fmt::Display for ModpackCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Instance(err) => write!(f, "{err}"),
            Self::Modpack(err) => write!(f, "{err}"),
            Self::Install(err) => write!(f, "{err}"),
            Self::Modrinth(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for ModpackCommandError {}

impl From<InstanceError> for ModpackCommandError {
    fn from(err: InstanceError) -> Self {
        Self::Instance(err)
    }
}

impl From<ModpackError> for ModpackCommandError {
    fn from(err: ModpackError) -> Self {
        Self::Modpack(err)
    }
}

impl From<InstallCommandError> for ModpackCommandError {
    fn from(err: InstallCommandError) -> Self {
        Self::Install(err)
    }
}

impl From<ModrinthError> for ModpackCommandError {
    fn from(err: ModrinthError) -> Self {
        Self::Modrinth(err)
    }
}

impl serde::Serialize for ModpackCommandError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Modpacks compatible with `loader`/`game_version` matching `query` -
/// identical to `commands::modrinth::search_content` except for the
/// `project_type`, since Modrinth hosts modpacks in the same catalog as
/// mods. Unlike resource packs/shaders, modpacks are always loader-gated
/// (a modpack targets one specific loader), so `loader` is always sent.
#[tauri::command]
pub async fn search_modpacks(
    http_client: State<'_, reqwest::Client>,
    query: String,
    loader: String,
    game_version: String,
) -> Result<SearchResponse, ModpackCommandError> {
    api::search(
        http_client.inner(),
        api::MODRINTH_API_URL,
        "modpack",
        &query,
        Some(&loader),
        &game_version,
        20,
    )
    .await
    .map_err(ModpackCommandError::from)
}

/// Every version of a modpack project compatible with `loader`/`game_version`.
#[tauri::command]
pub async fn list_modpack_versions(
    http_client: State<'_, reqwest::Client>,
    project_id: String,
    loader: String,
    game_version: String,
) -> Result<Vec<ModrinthVersion>, ModpackCommandError> {
    api::list_project_versions(
        http_client.inner(),
        api::MODRINTH_API_URL,
        &project_id,
        Some(&loader),
        &game_version,
    )
    .await
    .map_err(ModpackCommandError::from)
}

/// Imports a `.mrpack` the user picked from disk: creates a fresh instance
/// for it and fully installs it (base game, loader, every declared file,
/// overrides). `path` is a real filesystem path from the native file picker
/// (`@tauri-apps/plugin-dialog`), not a browser `File` object.
#[tauri::command]
pub async fn import_modpack_file(
    app: AppHandle,
    app_paths: State<'_, AppPaths>,
    http_client: State<'_, reqwest::Client>,
    path: String,
    concurrency: usize,
) -> Result<InstanceMeta, ModpackCommandError> {
    let client = http_client.inner().clone();
    import_and_install(&app, &app_paths, &client, Path::new(&path), concurrency).await
}

/// Downloads a modpack version's `.mrpack` from Modrinth to a temp file,
/// then imports it exactly like a local file - the only difference between
/// the two sources is how the bytes reach disk.
#[tauri::command]
pub async fn install_modpack_from_modrinth(
    app: AppHandle,
    app_paths: State<'_, AppPaths>,
    http_client: State<'_, reqwest::Client>,
    version: ModrinthVersion,
    concurrency: usize,
) -> Result<InstanceMeta, ModpackCommandError> {
    let client = http_client.inner().clone();
    let file = version
        .files
        .iter()
        .find(|f| f.primary)
        .or_else(|| version.files.first())
        .ok_or_else(|| ModpackError::Parse("this modpack version has no files".to_string()))?;

    let temp_dir = app_paths.temp_downloads_dir();
    fs::create_dir_all(&temp_dir).map_err(ModpackError::from)?;
    let temp_path = temp_dir.join(format!("{}.mrpack", version.id));

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<DownloadEvent>();
    let forward_app = app.clone();
    let forwarder = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let _ = forward_app.emit(INSTALL_PROGRESS_EVENT, event);
        }
    });
    let task = DownloadTask {
        url: file.url.clone(),
        dest: temp_path.clone(),
        expected_sha1: Some(file.hashes.sha1.clone()),
        label: file.filename.clone(),
    };
    let summary = download_all(&client, vec![task], 1, ProgressSink::new(tx)).await;
    let _ = forwarder.await;
    if summary.failed > 0 {
        return Err(ModpackCommandError::from(ModpackError::DownloadFailed(
            format!("Couldn't download {}", file.filename),
        )));
    }

    let result = import_and_install(&app, &app_paths, &client, &temp_path, concurrency).await;
    let _ = fs::remove_file(&temp_path);
    result
}

async fn import_and_install(
    app: &AppHandle,
    app_paths: &AppPaths,
    client: &reqwest::Client,
    mrpack_path: &Path,
    concurrency: usize,
) -> Result<InstanceMeta, ModpackCommandError> {
    let index = modpack_index::read_index(mrpack_path)?;
    let minecraft_version = index
        .dependencies
        .get("minecraft")
        .cloned()
        .ok_or(ModpackError::MissingMinecraftVersion)?;
    let (loader_kind, loader_version) = modpack_index::resolve_loader(&index.dependencies)?;

    let instance = instance_service::create(
        &app_paths.instances_dir(),
        NewInstanceInput {
            name: index.name.clone(),
            minecraft_version,
            loader: LoaderConfig {
                kind: loader_kind,
                version: loader_version,
            },
        },
    )?;
    let instance_dir =
        instance_service::resolve_instance_dir(&app_paths.instances_dir(), &instance.id)?;

    if let Err(err) = fs::copy(mrpack_path, instance_dir.join(modpacks::MODPACK_FILE)) {
        // Nothing real has been installed yet at this point - unlike a
        // failure further down, there's no partial install worth keeping
        // around for the user to resume, so clean up instead of leaving an
        // empty, indistinguishable-from-manually-created profile behind.
        let _ = instance_service::delete(&app_paths.instances_dir(), &instance.id);
        return Err(ModpackError::from(err).into());
    }

    install_vanilla_and_loader(app, app_paths, client, &instance, concurrency).await?;
    install_pack_files_and_overrides(app, client, &instance_dir, concurrency).await?;

    Ok(instance)
}

/// The modpack-specific half of installing an instance (every file
/// `modrinth.index.json` declares, plus overrides) - called both right
/// after import above, and from `commands::versions::install_instance` so
/// re-clicking "Install" on a modpack profile can resume a failed import
/// the same way every other install path in this app is resumable.
pub(crate) async fn install_pack_files_and_overrides(
    app: &AppHandle,
    client: &reqwest::Client,
    instance_dir: &Path,
    concurrency: usize,
) -> Result<(), ModpackError> {
    let mrpack_path = instance_dir.join(modpacks::MODPACK_FILE);
    let index = modpack_index::read_index(&mrpack_path)?;

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<DownloadEvent>();
    let forward_app = app.clone();
    let forwarder = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let _ = forward_app.emit(INSTALL_PROGRESS_EVENT, event);
        }
    });
    let result = modpack_install::download_pack_files(
        client,
        &index,
        instance_dir,
        concurrency,
        ProgressSink::new(tx),
    )
    .await;
    let _ = forwarder.await;
    result?;

    modpack_install::extract_overrides(&mrpack_path, instance_dir)?;
    Ok(())
}
