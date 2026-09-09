//! `AccountService`: sequences `oauth`/`xbox`/`flow` into a full sign-in or
//! a silent refresh, and persists the multi-account list. Kept as free
//! functions taking their dependencies explicitly (accounts file path,
//! `&dyn SecureTokenStore`, `&reqwest::Client`) rather than a struct with
//! methods, matching `core::instances::service`'s style.

use std::path::Path;

use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use super::model::{AccountsFile, StoredAccount};
use super::token_store::SecureTokenStore;
use super::xbox::XboxTokenResponse;
use super::{flow, oauth, xbox, AccountError};
use crate::core::launch::AuthSession;

const ACCOUNTS_FILE_NAME: &str = "accounts.json";

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SignInEvent {
    OpeningBrowser,
    WaitingForBrowser,
    ExchangingCode,
    AuthenticatingWithXbox,
    AuthenticatingWithXsts,
    AuthenticatingWithMinecraft,
    CheckingOwnership,
    FetchingProfile,
}

/// Where `SignInEvent`s go - mirrors `core::downloads::ProgressSink`/
/// `core::launch::LaunchEventSink`.
#[derive(Clone)]
pub struct SignInEventSink(Option<tokio::sync::mpsc::UnboundedSender<SignInEvent>>);

impl SignInEventSink {
    pub fn new(sender: tokio::sync::mpsc::UnboundedSender<SignInEvent>) -> Self {
        Self(Some(sender))
    }

    pub fn none() -> Self {
        Self(None)
    }

    fn send(&self, event: SignInEvent) {
        if let Some(sender) = &self.0 {
            let _ = sender.send(event);
        }
    }
}

/// `pub(crate)` rather than private: `commands::launch` reuses this same
/// timestamp format for `InstanceMeta::last_played`, so there's exactly
/// one place that decides what "now" looks like on disk.
pub(crate) fn now_iso8601() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| String::from("1970-01-01T00:00:00Z"))
}

pub fn accounts_file_path(settings_dir: &Path) -> std::path::PathBuf {
    settings_dir.join(ACCOUNTS_FILE_NAME)
}

/// Reads the account list. A missing or corrupted file degrades to empty
/// rather than failing outward - this file is a disposable local index of
/// real Microsoft accounts, not their source of truth, so the worst case
/// of losing it is the user signs in again.
pub fn read_accounts_file(path: &Path) -> AccountsFile {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// Temp-file-then-rename so a crash mid-write can never leave a truncated
/// `accounts.json` in place of a good one - the same pattern
/// `core::instances::service::write_meta` uses.
fn write_accounts_file(path: &Path, file: &AccountsFile) -> Result<(), AccountError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_vec_pretty(file)?;
    let tmp_path = path.with_extension("json.tmp");
    std::fs::write(&tmp_path, json)?;
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

/// Adds a new account or refreshes an existing one's cached profile info
/// and `last_used`, then makes it the active account. Pure data
/// transformation - split out from the disk I/O around it so it's
/// unit-testable without a filesystem.
fn upsert_and_activate(mut file: AccountsFile, account: StoredAccount) -> AccountsFile {
    if let Some(existing) = file
        .accounts
        .iter_mut()
        .find(|a| a.minecraft_uuid == account.minecraft_uuid)
    {
        *existing = account.clone();
    } else {
        file.accounts.push(account.clone());
    }
    file.active_uuid = Some(account.minecraft_uuid);
    file
}

pub fn list_accounts(accounts_path: &Path) -> Vec<StoredAccount> {
    read_accounts_file(accounts_path).accounts
}

pub fn active_account_uuid(accounts_path: &Path) -> Option<String> {
    read_accounts_file(accounts_path).active_uuid
}

/// Switches the active account without touching the network - just moves
/// which already-signed-in account subsequent launches/refreshes use.
pub fn set_active_account(accounts_path: &Path, uuid: &str) -> Result<(), AccountError> {
    let mut file = read_accounts_file(accounts_path);
    if !file.accounts.iter().any(|a| a.minecraft_uuid == uuid) {
        return Err(AccountError::Provider(format!(
            "No stored account with id \"{uuid}\"."
        )));
    }
    file.active_uuid = Some(uuid.to_string());
    write_accounts_file(accounts_path, &file)
}

/// Forgets an account entirely: removed from the visible list, its
/// refresh token deleted from secure storage. If it was the active
/// account, nothing is active afterward - the UI is expected to prompt a
/// sign-in or a manual re-selection rather than this guessing which
/// remaining account (if any) should take over.
pub fn remove_account(
    accounts_path: &Path,
    token_store: &dyn SecureTokenStore,
    uuid: &str,
) -> Result<(), AccountError> {
    token_store.delete_refresh_token(uuid)?;
    let mut file = read_accounts_file(accounts_path);
    file.accounts.retain(|a| a.minecraft_uuid != uuid);
    if file.active_uuid.as_deref() == Some(uuid) {
        file.active_uuid = None;
    }
    write_accounts_file(accounts_path, &file)
}

/// The full loopback-PKCE sign-in flow, start to finish: binds the
/// redirect listener, builds the authorization URL, hands it to
/// `open_browser` (the only step that needs a real Tauri `AppHandle`,
/// which this Tauri-free module never touches directly), waits for the
/// redirect, exchanges the code, then runs the same Xbox/XSTS/Minecraft
/// chain a silent refresh uses. On success the account is saved to
/// `accounts.json` and made active either way; its Microsoft refresh token
/// is saved to `token_store` only when `remember` is true - "Remember me"
/// unchecked means this sign-in lasts only for the current run of the app
/// (still cached in memory for launches this session), with nothing left
/// for a future `silent_refresh`/`try_restore_session` to find.
pub async fn complete_sign_in(
    accounts_path: &Path,
    token_store: &dyn SecureTokenStore,
    client: &reqwest::Client,
    client_id: &str,
    remember: bool,
    events: &SignInEventSink,
    open_browser: impl FnOnce(&str) -> Result<(), AccountError>,
) -> Result<AuthSession, AccountError> {
    if client_id.is_empty() {
        return Err(AccountError::NotConfigured);
    }

    let pkce = oauth::generate_pkce();
    let state = oauth::generate_state();
    let server = oauth::bind_loopback_server().await?;
    let auth_url = oauth::build_authorization_url(client_id, &server.redirect_uri, &pkce, &state);

    events.send(SignInEvent::OpeningBrowser);
    open_browser(&auth_url)?;

    events.send(SignInEvent::WaitingForBrowser);
    let redirect_uri = server.redirect_uri.clone();
    let redirect = server.wait_for_redirect(&state).await?;

    events.send(SignInEvent::ExchangingCode);
    let ms_tokens = flow::exchange_code_for_tokens(
        client,
        oauth::TOKEN_URL,
        client_id,
        &redirect.code,
        &redirect_uri,
        &pkce.verifier,
    )
    .await?;

    let (account, session) =
        authenticate_to_minecraft(client, client_id, &ms_tokens.access_token, events).await?;

    if remember {
        token_store.save_refresh_token(&account.minecraft_uuid, &ms_tokens.refresh_token)?;
    }
    let file = upsert_and_activate(read_accounts_file(accounts_path), account);
    write_accounts_file(accounts_path, &file)?;

    Ok(session)
}

/// Renews a previously signed-in account without the browser: exchanges
/// the stored Microsoft refresh token for a fresh one (Microsoft rotates
/// these, so the newly-issued token always replaces the stored one - never
/// reused), then re-derives a Minecraft session through the same
/// Xbox/XSTS/Minecraft Services chain sign-in uses, since Minecraft access
/// tokens are too short-lived to store and refresh directly.
pub async fn silent_refresh(
    accounts_path: &Path,
    token_store: &dyn SecureTokenStore,
    client: &reqwest::Client,
    client_id: &str,
    account_uuid: &str,
) -> Result<AuthSession, AccountError> {
    if client_id.is_empty() {
        return Err(AccountError::NotConfigured);
    }

    let stored_refresh_token = token_store
        .load_refresh_token(account_uuid)?
        .ok_or_else(|| {
            AccountError::Provider(
                "No stored sign-in for this account - please sign in again.".into(),
            )
        })?;

    let ms_tokens =
        flow::refresh_microsoft_tokens(client, oauth::TOKEN_URL, client_id, &stored_refresh_token)
            .await?;
    token_store.save_refresh_token(account_uuid, &ms_tokens.refresh_token)?;

    let (account, session) = authenticate_to_minecraft(
        client,
        client_id,
        &ms_tokens.access_token,
        &SignInEventSink::none(),
    )
    .await?;

    let file = upsert_and_activate(read_accounts_file(accounts_path), account);
    write_accounts_file(accounts_path, &file)?;

    Ok(session)
}

/// The shared tail end of both sign-in and refresh: a valid Microsoft
/// access token in hand, walk Xbox Live -> XSTS -> Minecraft Services ->
/// entitlement check -> profile, producing both the cached account record
/// and the session a launch needs right now.
async fn authenticate_to_minecraft(
    client: &reqwest::Client,
    client_id: &str,
    microsoft_access_token: &str,
    events: &SignInEventSink,
) -> Result<(StoredAccount, AuthSession), AccountError> {
    events.send(SignInEvent::AuthenticatingWithXbox);
    let xbl = xbox::authenticate_with_xbox_live(client, microsoft_access_token).await?;

    events.send(SignInEvent::AuthenticatingWithXsts);
    let xsts: XboxTokenResponse = xbox::authorize_with_xsts(client, &xbl.token).await?;

    events.send(SignInEvent::AuthenticatingWithMinecraft);
    let mc_access_token = flow::login_with_xbox(client, flow::MINECRAFT_LOGIN_URL, &xsts).await?;

    events.send(SignInEvent::CheckingOwnership);
    flow::check_owns_minecraft(client, flow::MINECRAFT_ENTITLEMENTS_URL, &mc_access_token).await?;

    events.send(SignInEvent::FetchingProfile);
    let profile =
        flow::fetch_profile(client, flow::MINECRAFT_PROFILE_URL, &mc_access_token).await?;

    let account = StoredAccount {
        minecraft_uuid: profile.uuid.clone(),
        username: profile.username.clone(),
        skin_url: profile.skin_url,
        last_used: now_iso8601(),
    };
    let session = AuthSession {
        player_name: profile.username,
        uuid: profile.uuid,
        access_token: mc_access_token,
        xuid: xsts.xuid.unwrap_or_default(),
        client_id: client_id.to_string(),
    };
    Ok((account, session))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_account(uuid: &str) -> StoredAccount {
        StoredAccount {
            minecraft_uuid: uuid.into(),
            username: format!("Player-{uuid}"),
            skin_url: None,
            last_used: "2026-01-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn upsert_and_activate_adds_a_new_account_and_makes_it_active() {
        let file = upsert_and_activate(AccountsFile::default(), sample_account("uuid-1"));
        assert_eq!(file.accounts.len(), 1);
        assert_eq!(file.active_uuid.as_deref(), Some("uuid-1"));
    }

    #[test]
    fn upsert_and_activate_replaces_an_existing_account_in_place_rather_than_duplicating() {
        let file = upsert_and_activate(AccountsFile::default(), sample_account("uuid-1"));
        let mut updated = sample_account("uuid-1");
        updated.username = "NewName".into();
        let file = upsert_and_activate(file, updated);

        assert_eq!(file.accounts.len(), 1);
        assert_eq!(file.accounts[0].username, "NewName");
    }

    #[test]
    fn upsert_and_activate_keeps_other_accounts_and_switches_which_is_active() {
        let file = upsert_and_activate(AccountsFile::default(), sample_account("uuid-1"));
        let file = upsert_and_activate(file, sample_account("uuid-2"));

        assert_eq!(file.accounts.len(), 2);
        assert_eq!(file.active_uuid.as_deref(), Some("uuid-2"));
    }

    #[test]
    fn read_accounts_file_defaults_on_a_missing_file_instead_of_erroring() {
        let dir = tempfile::tempdir().unwrap();
        let file = read_accounts_file(&accounts_file_path(dir.path()));
        assert!(file.accounts.is_empty());
    }

    #[test]
    fn read_accounts_file_defaults_on_corrupted_json_instead_of_crashing_the_app() {
        let dir = tempfile::tempdir().unwrap();
        let path = accounts_file_path(dir.path());
        std::fs::write(&path, b"{ not valid json").unwrap();
        let file = read_accounts_file(&path);
        assert!(file.accounts.is_empty());
    }

    #[test]
    fn write_then_read_accounts_file_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = accounts_file_path(dir.path());
        let file = upsert_and_activate(AccountsFile::default(), sample_account("uuid-1"));

        write_accounts_file(&path, &file).unwrap();
        let reloaded = read_accounts_file(&path);

        assert_eq!(reloaded, file);
    }

    #[test]
    fn set_active_account_rejects_an_unknown_uuid() {
        let dir = tempfile::tempdir().unwrap();
        let path = accounts_file_path(dir.path());
        let file = upsert_and_activate(AccountsFile::default(), sample_account("uuid-1"));
        write_accounts_file(&path, &file).unwrap();

        let err = set_active_account(&path, "does-not-exist").unwrap_err();
        assert!(matches!(err, AccountError::Provider(_)));
        // Unchanged on failure.
        assert_eq!(active_account_uuid(&path).as_deref(), Some("uuid-1"));
    }

    #[test]
    fn set_active_account_switches_between_two_stored_accounts() {
        let dir = tempfile::tempdir().unwrap();
        let path = accounts_file_path(dir.path());
        let file = upsert_and_activate(AccountsFile::default(), sample_account("uuid-1"));
        let file = upsert_and_activate(file, sample_account("uuid-2"));
        write_accounts_file(&path, &file).unwrap();
        assert_eq!(active_account_uuid(&path).as_deref(), Some("uuid-2"));

        set_active_account(&path, "uuid-1").unwrap();
        assert_eq!(active_account_uuid(&path).as_deref(), Some("uuid-1"));
    }

    #[test]
    fn remove_account_drops_it_from_the_list_and_clears_active_if_it_was_active() {
        // A unique key per run, not a literal "uuid-1" - this test touches
        // the real OS credential store (see token_store.rs), and the Linux
        // kernel keyring backend caches a deleted entry's revoked state by
        // description. A hardcoded key that a previous run of this same
        // test already deleted comes back to a *different* error than a
        // fresh key would, purely as an artifact of re-running against
        // long-lived kernel keyring state - not something a real account
        // (a naturally unique Minecraft UUID) would ever hit.
        let uuid = format!("test-remove-{}", std::process::id());
        let dir = tempfile::tempdir().unwrap();
        let path = accounts_file_path(dir.path());
        let file = upsert_and_activate(AccountsFile::default(), sample_account(&uuid));
        write_accounts_file(&path, &file).unwrap();

        let store = crate::core::accounts::token_store::KeyringTokenStore;
        // Clean slate: this container's low process count means PIDs (and
        // so this key) get reused across separate test runs quickly - the
        // same "clean slate" step token_store.rs's own real-store test
        // uses, and for the same reason.
        let _ = store.delete_refresh_token(&uuid);
        store.save_refresh_token(&uuid, "irrelevant-value").unwrap();

        remove_account(&path, &store, &uuid).unwrap();

        assert!(list_accounts(&path).is_empty());
        assert!(active_account_uuid(&path).is_none());
        // The refresh token is gone too, not just the visible list entry.
        assert_eq!(store.load_refresh_token(&uuid).unwrap(), None);
    }

    #[tokio::test]
    async fn complete_sign_in_reports_not_configured_without_a_client_id() {
        let dir = tempfile::tempdir().unwrap();
        let path = accounts_file_path(dir.path());
        let store = crate::core::accounts::token_store::KeyringTokenStore;
        let client = reqwest::Client::new();

        let result = complete_sign_in(
            &path,
            &store,
            &client,
            "",
            true,
            &SignInEventSink::none(),
            |_url| Ok(()),
        )
        .await;
        assert!(matches!(result, Err(AccountError::NotConfigured)));
    }

    #[tokio::test]
    async fn silent_refresh_reports_a_clear_error_when_nothing_was_ever_stored() {
        let dir = tempfile::tempdir().unwrap();
        let path = accounts_file_path(dir.path());
        let store = crate::core::accounts::token_store::KeyringTokenStore;
        let client = reqwest::Client::new();

        let result = silent_refresh(
            &path,
            &store,
            &client,
            "some-client-id",
            "never-signed-in-uuid",
        )
        .await;
        assert!(matches!(result, Err(AccountError::Provider(_))));
    }
}
