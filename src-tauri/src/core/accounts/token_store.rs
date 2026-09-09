//! `SecureTokenStore`: where the Microsoft refresh token lives between
//! runs. Never plaintext JSON on disk - `KeyringTokenStore` backs onto the
//! OS credential store (Windows Credential Manager on the real target
//! platform; the Linux kernel keyutils facility elsewhere, which needs no
//! secret-service daemon running). Entries are keyed by Minecraft UUID, one
//! per stored account.

use super::AccountError;

const SERVICE_NAME: &str = "TapkaCraft Launcher";

pub trait SecureTokenStore: Send + Sync {
    fn save_refresh_token(
        &self,
        account_uuid: &str,
        refresh_token: &str,
    ) -> Result<(), AccountError>;
    /// `Ok(None)` (not an error) when nothing has been stored for this
    /// account yet - a brand new account, or one whose token was already
    /// removed.
    fn load_refresh_token(&self, account_uuid: &str) -> Result<Option<String>, AccountError>;
    /// Idempotent: deleting an already-absent entry is not an error.
    fn delete_refresh_token(&self, account_uuid: &str) -> Result<(), AccountError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct KeyringTokenStore;

impl SecureTokenStore for KeyringTokenStore {
    fn save_refresh_token(
        &self,
        account_uuid: &str,
        refresh_token: &str,
    ) -> Result<(), AccountError> {
        let entry = keyring::Entry::new(SERVICE_NAME, account_uuid)?;
        entry.set_password(refresh_token)?;
        Ok(())
    }

    fn load_refresh_token(&self, account_uuid: &str) -> Result<Option<String>, AccountError> {
        let entry = keyring::Entry::new(SERVICE_NAME, account_uuid)?;
        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    fn delete_refresh_token(&self, account_uuid: &str) -> Result<(), AccountError> {
        let entry = keyring::Entry::new(SERVICE_NAME, account_uuid)?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(err.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real round trip against this machine's actual OS credential store -
    /// on Linux that's the kernel keyutils facility (`linux-native`
    /// feature), which needs no secret-service daemon, so this genuinely
    /// exercises the same code path Windows Credential Manager will use in
    /// production, not a fake. Uses a random-ish account id so repeated
    /// runs (and parallel test threads) don't collide with each other or
    /// leave stale state a previous run's failure might have left behind.
    fn test_account_id() -> String {
        format!("test-{}", std::process::id())
    }

    #[test]
    fn round_trips_a_refresh_token_through_the_real_os_credential_store() {
        let store = KeyringTokenStore;
        let account = test_account_id();

        // Start from a clean slate in case a previous failed run left this
        // entry behind.
        let _ = store.delete_refresh_token(&account);
        assert_eq!(store.load_refresh_token(&account).unwrap(), None);

        store
            .save_refresh_token(&account, "a-refresh-token-value")
            .unwrap();
        assert_eq!(
            store.load_refresh_token(&account).unwrap(),
            Some("a-refresh-token-value".to_string())
        );

        // Saving again overwrites rather than erroring or duplicating.
        store.save_refresh_token(&account, "a-newer-value").unwrap();
        assert_eq!(
            store.load_refresh_token(&account).unwrap(),
            Some("a-newer-value".to_string())
        );

        store.delete_refresh_token(&account).unwrap();
        assert_eq!(store.load_refresh_token(&account).unwrap(), None);
    }

    #[test]
    fn deleting_an_absent_entry_is_not_an_error() {
        let store = KeyringTokenStore;
        let account = format!("{}-never-existed", test_account_id());
        store.delete_refresh_token(&account).unwrap();
        store.delete_refresh_token(&account).unwrap(); // twice, still fine
    }
}
