//! `AccountService` (Phase 4).
//!
//! Owns Microsoft authentication end to end: a loopback authorization-code
//! flow with PKCE in the system browser (never a password form drawn by
//! this app), then Xbox Live user auth, then XSTS, then Minecraft Services
//! login, then the Minecraft profile. Also owns:
//!
//! - The account state machine: `LoggedOut -> Authenticating -> Authenticated`,
//!   with `Refreshing` and `AuthError` reachable from `Authenticated` on
//!   silent token refresh.
//! - Multi-account storage (add/switch/remove), keyed by Minecraft UUID.
//! - `SecureTokenStore`, so refresh tokens are never written as plaintext
//!   JSON. The Windows implementation backs onto Windows Credential
//!   Manager; the Linux implementation backs onto the kernel keyutils
//!   facility (no secret-service daemon required).
//!
//! Uses TapkaCraft Launcher's own Azure AD (Entra ID) app registration -
//! see `MICROSOFT_CLIENT_ID` in `oauth.rs`. Nothing in this module
//! fabricates a fake/offline session as a substitute for a real sign-in.

use std::fmt;

pub mod flow;
pub mod model;
pub mod oauth;
pub mod service;
pub mod token_store;
pub mod xbox;

#[derive(Debug)]
pub enum AccountError {
    /// The launcher isn't configured with a real Azure AD client ID yet.
    NotConfigured,
    /// The user closed the browser tab/window without finishing, or the
    /// flow was cancelled from the UI.
    Cancelled,
    /// Waited too long for the browser to complete the sign-in.
    TimedOut,
    /// Microsoft/Xbox Live/Minecraft Services returned an error, or the
    /// response didn't parse as expected. Carries a message that's already
    /// safe to show the user (never a raw token).
    Provider(String),
    /// This Microsoft account doesn't own Minecraft.
    NoMinecraftLicense,
    Network(String),
    Io(std::io::Error),
    Storage(String),
    Serialization(serde_json::Error),
}

impl fmt::Display for AccountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConfigured => write!(
                f,
                "Microsoft sign-in isn't configured yet (missing Azure AD client id)."
            ),
            Self::Cancelled => write!(f, "Sign-in was cancelled."),
            Self::TimedOut => write!(f, "Sign-in timed out waiting for the browser."),
            Self::Provider(msg) => write!(f, "{msg}"),
            Self::NoMinecraftLicense => {
                write!(f, "This Microsoft account doesn't own Minecraft.")
            }
            Self::Network(msg) => write!(f, "Network error during sign-in: {msg}"),
            Self::Io(err) => write!(f, "Sign-in error: {err}"),
            Self::Storage(msg) => write!(f, "Couldn't access secure credential storage: {msg}"),
            Self::Serialization(err) => write!(f, "Account data is corrupted: {err}"),
        }
    }
}

impl std::error::Error for AccountError {}

impl From<std::io::Error> for AccountError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for AccountError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(err)
    }
}

impl From<keyring::Error> for AccountError {
    fn from(err: keyring::Error) -> Self {
        Self::Storage(err.to_string())
    }
}

impl serde::Serialize for AccountError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
