//! Tauri IPC wrapper around `core::lan`: starts/stops the background
//! multicast listener and forwards `LanEvent`s to the frontend exactly like
//! `install://progress`/`launch://event` already do.

use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::core::lan::{self, LanEventSink};

const LAN_EVENT: &str = "lan://event";

/// The background listener task, if the LAN tab is currently open. Kept in
/// managed state (rather than one-shot-spawned per command call) so
/// `start_lan_discovery` is idempotent and `stop_lan_discovery` has
/// something real to cancel.
#[derive(Default)]
pub struct LanDiscoveryState(Mutex<Option<JoinHandle<std::io::Result<()>>>>);

/// Binds the multicast socket and starts listening, forwarding every
/// `LanEvent` as a `lan://event`. A real bind failure (e.g. no
/// multicast-capable network interface) is returned to the frontend
/// immediately rather than failing silently inside a background task - an
/// empty LAN list should mean "no games open", never "listening secretly
/// failed and nobody was told." Calling this again while already listening
/// is a no-op.
#[tauri::command]
pub async fn start_lan_discovery(
    app: AppHandle,
    state: State<'_, LanDiscoveryState>,
) -> Result<(), String> {
    let mut guard = state.0.lock().await;
    if guard.as_ref().is_some_and(|handle| !handle.is_finished()) {
        return Ok(());
    }

    let socket = lan::bind_multicast_socket()
        .map_err(|err| format!("Couldn't listen for LAN games: {err}"))?;

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let forward_app = app.clone();
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let _ = forward_app.emit(LAN_EVENT, event);
        }
    });

    *guard = Some(tokio::spawn(lan::run(socket, LanEventSink::new(tx))));
    Ok(())
}

/// Stops the background listener, if one is running. Called when the LAN
/// tab unmounts so an idle launcher isn't left with a multicast socket
/// bound forever.
#[tauri::command]
pub async fn stop_lan_discovery(state: State<'_, LanDiscoveryState>) -> Result<(), String> {
    if let Some(handle) = state.0.lock().await.take() {
        handle.abort();
    }
    Ok(())
}
