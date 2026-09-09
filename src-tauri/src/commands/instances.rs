//! Thin Tauri IPC wrappers around `core::instances::service`. No business
//! logic lives here - each handler just resolves the managed `AppPaths`
//! state to an `instances_dir` and delegates.

use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::core::instances::model::{InstanceMeta, InstanceUpdate, NewInstanceInput};
use crate::core::instances::service::{self, InstanceError};
use crate::core::paths::AppPaths;

#[tauri::command]
pub fn list_instances(app_paths: State<AppPaths>) -> Result<Vec<InstanceMeta>, InstanceError> {
    service::list(&app_paths.instances_dir())
}

#[tauri::command]
pub fn create_instance(
    app_paths: State<AppPaths>,
    input: NewInstanceInput,
) -> Result<InstanceMeta, InstanceError> {
    service::create(&app_paths.instances_dir(), input)
}

#[tauri::command]
pub fn update_instance(
    app_paths: State<AppPaths>,
    id: String,
    patch: InstanceUpdate,
) -> Result<InstanceMeta, InstanceError> {
    service::update(&app_paths.instances_dir(), &id, patch)
}

#[tauri::command]
pub fn delete_instance(app_paths: State<AppPaths>, id: String) -> Result<(), InstanceError> {
    service::delete(&app_paths.instances_dir(), &id)
}

/// Reveals the instance's directory in the OS file explorer (Windows
/// Explorer). Reuses `resolve_instance_dir`'s validation, so this can only
/// ever open a path this launcher actually manages.
#[tauri::command]
pub fn open_instance_folder(
    app: AppHandle,
    app_paths: State<AppPaths>,
    id: String,
) -> Result<(), InstanceError> {
    let dir = service::resolve_instance_dir(&app_paths.instances_dir(), &id)?;
    app.opener()
        .reveal_item_in_dir(dir)
        .map_err(|err| InstanceError::Io(std::io::Error::other(err.to_string())))
}
