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
//! - The `SecureTokenStore` trait, so refresh tokens are never written as
//!   plaintext JSON. The Windows implementation backs onto Windows
//!   Credential Manager; non-Windows implementations are secondary.
//!
//! Requires an Azure AD (Entra ID) app registration (client ID) supplied by
//! the project owner - see the Phase 4 notes in the project plan.
