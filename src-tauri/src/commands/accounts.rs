//! Tauri IPC wrappers around `core::accounts`. `begin_sign_in` is the only
//! one that needs a real `AppHandle` (to open the system browser) - every
//! other piece of the flow lives in Tauri-free `core::accounts` code.

use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::Mutex;

use crate::core::accounts::model::StoredAccount;
use crate::core::accounts::token_store::KeyringTokenStore;
use crate::core::accounts::{oauth, service, AccountError};
use crate::core::launch::AuthSession;
use crate::core::paths::AppPaths;

const SIGN_IN_PROGRESS_EVENT: &str = "signin://progress";

/// The currently-usable Minecraft session for this run of the app, cached
/// so a launch right after signing in doesn't repeat the whole Xbox
/// Live/XSTS/Minecraft Services round trip. Cleared on sign-out/switch;
/// refilled by `try_restore_session` or another sign-in when absent.
#[derive(Default)]
pub struct ActiveSession(pub Mutex<Option<AuthSession>>);

/// The full loopback sign-in flow. Streams `signin://progress`
/// (`SignInEvent`) throughout; resolves with the account once the whole
/// Xbox Live/XSTS/Minecraft Services/profile chain succeeds.
#[tauri::command]
pub async fn begin_sign_in(
    app: AppHandle,
    app_paths: State<'_, AppPaths>,
    http_client: State<'_, reqwest::Client>,
    active_session: State<'_, ActiveSession>,
    remember: bool,
) -> Result<StoredAccount, AccountError> {
    let client = http_client.inner().clone();
    let accounts_path = service::accounts_file_path(&app_paths.settings_dir());
    let token_store = KeyringTokenStore;

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let forward_app = app.clone();
    let forwarder = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let _ = forward_app.emit(SIGN_IN_PROGRESS_EVENT, event);
        }
    });

    let open_app = app.clone();
    let result = service::complete_sign_in(
        &accounts_path,
        &token_store,
        &client,
        oauth::MICROSOFT_CLIENT_ID,
        remember,
        &service::SignInEventSink::new(tx),
        move |url| {
            open_app
                .opener()
                .open_url(url, None::<String>)
                .map_err(|err| AccountError::Provider(format!("Couldn't open your browser: {err}")))
        },
    )
    .await;

    // Same reasoning as `install_instance`'s forwarder: the sink's sender
    // went out of scope with `complete_sign_in`, so this just makes sure
    // every already-queued event is flushed before the command returns.
    let _ = forwarder.await;

    let session = result?;
    let account_uuid = session.uuid.clone();
    *active_session.0.lock().await = Some(session);

    find_account(&accounts_path, &account_uuid).ok_or_else(|| {
        AccountError::Provider("Signed in, but couldn't reload the account record.".into())
    })
}

/// Tries to restore a previous sign-in without the browser, using whichever
/// account was last active and its stored refresh token. `Ok(None)` (not
/// an error) whenever this can't succeed for any reason - no account was
/// ever active, no client id configured, the network is unreachable, or
/// the stored refresh token was revoked - since none of those are the
/// user's problem to see an error about on every app start; the frontend's
/// answer is the same either way: show the sign-in screen.
#[tauri::command]
pub async fn try_restore_session(
    app_paths: State<'_, AppPaths>,
    http_client: State<'_, reqwest::Client>,
    active_session: State<'_, ActiveSession>,
) -> Result<Option<StoredAccount>, AccountError> {
    let accounts_path = service::accounts_file_path(&app_paths.settings_dir());
    let Some(active_uuid) = service::active_account_uuid(&accounts_path) else {
        return Ok(None);
    };

    let client = http_client.inner().clone();
    let token_store = KeyringTokenStore;
    let refreshed = service::silent_refresh(
        &accounts_path,
        &token_store,
        &client,
        oauth::MICROSOFT_CLIENT_ID,
        &active_uuid,
    )
    .await;

    match refreshed {
        Ok(session) => {
            let uuid = session.uuid.clone();
            *active_session.0.lock().await = Some(session);
            Ok(find_account(&accounts_path, &uuid))
        }
        Err(_) => Ok(None),
    }
}

#[tauri::command]
pub fn list_accounts(app_paths: State<AppPaths>) -> Vec<StoredAccount> {
    service::list_accounts(&service::accounts_file_path(&app_paths.settings_dir()))
}

#[tauri::command]
pub fn active_account(app_paths: State<AppPaths>) -> Option<StoredAccount> {
    let path = service::accounts_file_path(&app_paths.settings_dir());
    let active_uuid = service::active_account_uuid(&path)?;
    find_account(&path, &active_uuid)
}

fn find_account(accounts_path: &std::path::Path, uuid: &str) -> Option<StoredAccount> {
    service::list_accounts(accounts_path)
        .into_iter()
        .find(|account| account.minecraft_uuid == uuid)
}

/// Switches which stored account is active - no network involved. The next
/// launch (or `try_restore_session` on the next app start) silently
/// refreshes under the newly-active account instead of reusing whatever
/// session was cached for the previous one.
#[tauri::command]
pub async fn switch_account(
    app_paths: State<'_, AppPaths>,
    active_session: State<'_, ActiveSession>,
    uuid: String,
) -> Result<(), AccountError> {
    let path = service::accounts_file_path(&app_paths.settings_dir());
    service::set_active_account(&path, &uuid)?;
    *active_session.0.lock().await = None;
    Ok(())
}

/// Clears the in-memory session (Play locks again, the login screen shows)
/// without forgetting the account - its refresh token stays in secure
/// storage so switching back to it later doesn't need the browser again.
/// Use `remove_account` to actually forget an account.
#[tauri::command]
pub async fn sign_out(active_session: State<'_, ActiveSession>) -> Result<(), AccountError> {
    *active_session.0.lock().await = None;
    Ok(())
}

/// Forgets an account entirely: dropped from the stored list, its refresh
/// token deleted from secure storage. Clears the in-memory session too, if
/// this was the account it belonged to.
#[tauri::command]
pub async fn remove_account(
    app_paths: State<'_, AppPaths>,
    active_session: State<'_, ActiveSession>,
    uuid: String,
) -> Result<(), AccountError> {
    let path = service::accounts_file_path(&app_paths.settings_dir());
    let token_store = KeyringTokenStore;
    service::remove_account(&path, &token_store, &uuid)?;

    let mut guard = active_session.0.lock().await;
    if guard.as_ref().is_some_and(|session| session.uuid == uuid) {
        *guard = None;
    }
    Ok(())
}
