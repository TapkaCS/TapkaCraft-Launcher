//! Data types persisted/exchanged by `AccountService`. Kept separate from
//! `flow.rs`/`xbox.rs` so the shapes stored on disk and sent over Tauri IPC
//! don't get tangled up with the raw provider response shapes.

use serde::{Deserialize, Serialize};

/// One signed-in Minecraft account, as tracked in the multi-account list.
/// Never carries a refresh token or access token - those live only in
/// `SecureTokenStore` (refresh token) or nowhere at all on disk (Minecraft
/// access token, re-derived each session).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredAccount {
    pub minecraft_uuid: String,
    pub username: String,
    #[serde(default)]
    pub skin_url: Option<String>,
    /// ISO-8601 timestamp of the last successful sign-in/refresh.
    pub last_used: String,
}

/// The on-disk shape of `accounts.json` under `AppPaths::settings_dir`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountsFile {
    #[serde(default)]
    pub accounts: Vec<StoredAccount>,
    #[serde(default)]
    pub active_uuid: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accounts_file_round_trips_through_json() {
        let file = AccountsFile {
            accounts: vec![StoredAccount {
                minecraft_uuid: "abc123".into(),
                username: "Steve".into(),
                skin_url: Some("https://textures.minecraft.net/x".into()),
                last_used: "2026-01-01T00:00:00Z".into(),
            }],
            active_uuid: Some("abc123".into()),
        };
        let json = serde_json::to_string(&file).unwrap();
        let parsed: AccountsFile = serde_json::from_str(&json).unwrap();
        assert_eq!(file, parsed);
    }

    #[test]
    fn a_missing_accounts_file_field_defaults_instead_of_failing_to_parse() {
        let parsed: AccountsFile = serde_json::from_str("{}").unwrap();
        assert!(parsed.accounts.is_empty());
        assert!(parsed.active_uuid.is_none());
    }
}
