//! The Tauri IPC boundary. Handlers here stay thin and delegate to
//! `crate::core` services - no business logic lives in this module. The
//! frontend must never re-implement launcher logic itself; it only calls
//! through commands declared here.

/// Real, working command: proves the frontend <-> Rust IPC bridge functions
/// end to end. Surfaced in the UI as the small version string shown on the
/// login and dashboard screens.
#[tauri::command]
pub fn get_launcher_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
