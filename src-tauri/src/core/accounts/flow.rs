//! The rest of the token exchange chain, past Xbox Live/XSTS (`xbox.rs`):
//! turning an authorization code (or a stored refresh token) into
//! Microsoft tokens, and turning an XSTS token into an actual Minecraft
//! session - access token, entitlement check, and profile.
//!
//! Every call here takes its endpoint URL as a parameter (production call
//! sites in `service.rs` pass the real constants below) rather than
//! hardcoding it, the same way `core::versions`' fetch functions do - it's
//! what lets these be tested against a local mock server instead of the
//! real Microsoft/Xbox/Minecraft Services hosts, which this sandbox's
//! egress policy blocks outright.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::xbox::XboxTokenResponse;
use super::AccountError;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
pub const MINECRAFT_LOGIN_URL: &str =
    "https://api.minecraftservices.com/authentication/login_with_xbox";
pub const MINECRAFT_ENTITLEMENTS_URL: &str =
    "https://api.minecraftservices.com/entitlements/mcstore";
pub const MINECRAFT_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

#[derive(Debug, Clone)]
pub struct MicrosoftTokens {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
}

#[derive(Debug, Deserialize)]
struct TokenErrorResponse {
    #[serde(default)]
    error_description: String,
}

/// First leg: the loopback redirect's authorization code, PKCE verifier
/// and the exact `redirect_uri` used to request it, exchanged for a
/// Microsoft access + refresh token pair.
pub async fn exchange_code_for_tokens(
    client: &reqwest::Client,
    token_url: &str,
    client_id: &str,
    code: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Result<MicrosoftTokens, AccountError> {
    let form = [
        ("client_id", client_id),
        ("code", code),
        ("grant_type", "authorization_code"),
        ("redirect_uri", redirect_uri),
        ("code_verifier", code_verifier),
    ];
    post_token_endpoint(client, token_url, &form).await
}

/// Silent refresh: trades a previously-stored Microsoft refresh token for
/// a fresh access + refresh token pair, without involving the browser at
/// all. Called on every launch/sign-in-restore, since the Minecraft access
/// token this eventually produces (via Xbox Live/XSTS/Minecraft Services)
/// is short-lived and has to be re-derived each session.
pub async fn refresh_microsoft_tokens(
    client: &reqwest::Client,
    token_url: &str,
    client_id: &str,
    refresh_token: &str,
) -> Result<MicrosoftTokens, AccountError> {
    let form = [
        ("client_id", client_id),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
    ];
    post_token_endpoint(client, token_url, &form).await
}

async fn post_token_endpoint(
    client: &reqwest::Client,
    url: &str,
    form: &[(&str, &str)],
) -> Result<MicrosoftTokens, AccountError> {
    let response = client
        .post(url)
        .timeout(REQUEST_TIMEOUT)
        .form(form)
        .send()
        .await
        .map_err(|err| AccountError::Network(err.to_string()))?;

    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(|err| AccountError::Network(err.to_string()))?;

    if !status.is_success() {
        let description = serde_json::from_slice::<TokenErrorResponse>(&bytes)
            .ok()
            .map(|err| err.error_description)
            .filter(|d| !d.is_empty())
            .unwrap_or_else(|| format!("Microsoft sign-in failed (HTTP {status})."));
        return Err(AccountError::Provider(description));
    }

    let parsed: TokenResponse = serde_json::from_slice(&bytes).map_err(|err| {
        AccountError::Provider(format!("Unexpected response from Microsoft: {err}"))
    })?;
    Ok(MicrosoftTokens {
        access_token: parsed.access_token,
        refresh_token: parsed.refresh_token,
    })
}

#[derive(Debug, Serialize)]
struct MinecraftLoginRequest {
    #[serde(rename = "identityToken")]
    identity_token: String,
}

#[derive(Debug, Deserialize)]
struct MinecraftLoginResponse {
    access_token: String,
}

/// The identity token format Minecraft Services expects is `XBL3.0 x=<user
/// hash>;<XSTS token>` - assembled here rather than left to call sites, so
/// there is exactly one place that has to get this format right.
pub async fn login_with_xbox(
    client: &reqwest::Client,
    url: &str,
    xsts: &XboxTokenResponse,
) -> Result<String, AccountError> {
    let identity_token = format!("XBL3.0 x={};{}", xsts.user_hash, xsts.token);
    let response = client
        .post(url)
        .timeout(REQUEST_TIMEOUT)
        .json(&MinecraftLoginRequest { identity_token })
        .send()
        .await
        .map_err(|err| AccountError::Network(err.to_string()))?;

    if !response.status().is_success() {
        return Err(AccountError::Provider(format!(
            "Minecraft sign-in failed (HTTP {}).",
            response.status()
        )));
    }
    let parsed: MinecraftLoginResponse = response.json().await.map_err(|err| {
        AccountError::Provider(format!(
            "Unexpected response from Minecraft Services: {err}"
        ))
    })?;
    Ok(parsed.access_token)
}

#[derive(Debug, Deserialize)]
struct EntitlementsResponse {
    #[serde(default)]
    items: Vec<serde_json::Value>,
}

/// Confirms the signed-in account actually owns Minecraft, so a licensing
/// problem shows up as a clear message here instead of a confusing failure
/// partway through a launch later.
pub async fn check_owns_minecraft(
    client: &reqwest::Client,
    url: &str,
    minecraft_access_token: &str,
) -> Result<(), AccountError> {
    let response = client
        .get(url)
        .timeout(REQUEST_TIMEOUT)
        .bearer_auth(minecraft_access_token)
        .send()
        .await
        .map_err(|err| AccountError::Network(err.to_string()))?;

    if !response.status().is_success() {
        return Err(AccountError::Provider(format!(
            "Couldn't verify game ownership (HTTP {}).",
            response.status()
        )));
    }
    let parsed: EntitlementsResponse = response.json().await.map_err(|err| {
        AccountError::Provider(format!(
            "Unexpected response checking game ownership: {err}"
        ))
    })?;
    if parsed.items.is_empty() {
        return Err(AccountError::NoMinecraftLicense);
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct MinecraftProfile {
    pub uuid: String,
    pub username: String,
    pub skin_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProfileResponse {
    id: String,
    name: String,
    #[serde(default)]
    skins: Vec<ProfileSkin>,
}

#[derive(Debug, Deserialize)]
struct ProfileSkin {
    state: String,
    url: String,
}

pub async fn fetch_profile(
    client: &reqwest::Client,
    url: &str,
    minecraft_access_token: &str,
) -> Result<MinecraftProfile, AccountError> {
    let response = client
        .get(url)
        .timeout(REQUEST_TIMEOUT)
        .bearer_auth(minecraft_access_token)
        .send()
        .await
        .map_err(|err| AccountError::Network(err.to_string()))?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(AccountError::Provider(
            "This Microsoft account doesn't have a Minecraft profile set up yet - create one at minecraft.net first."
                .to_string(),
        ));
    }
    if !response.status().is_success() {
        return Err(AccountError::Provider(format!(
            "Couldn't fetch the Minecraft profile (HTTP {}).",
            response.status()
        )));
    }

    let parsed: ProfileResponse = response.json().await.map_err(|err| {
        AccountError::Provider(format!("Unexpected response fetching the profile: {err}"))
    })?;
    let skin_url = parsed
        .skins
        .into_iter()
        .find(|skin| skin.state == "ACTIVE")
        .map(|skin| skin.url);

    Ok(MinecraftProfile {
        uuid: parsed.id,
        username: parsed.name,
        skin_url,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::test_support::{spawn_mock_server, spawn_mock_server_with_status, MockResponse};

    fn fake_xsts() -> XboxTokenResponse {
        XboxTokenResponse {
            token: "xsts-token".into(),
            user_hash: "user-hash".into(),
            xuid: Some("123".into()),
        }
    }

    #[tokio::test]
    async fn exchange_code_for_tokens_sends_the_right_form_and_parses_the_pair() {
        let response_json = serde_json::json!({
            "token_type": "Bearer",
            "scope": "XboxLive.signin offline_access",
            "expires_in": 3600,
            "access_token": "ms-access-token",
            "refresh_token": "ms-refresh-token"
        });
        let mut files = HashMap::new();
        files.insert(
            "/token".to_string(),
            serde_json::to_vec(&response_json).unwrap(),
        );
        let base = spawn_mock_server(files).await;

        let client = reqwest::Client::new();
        let tokens = exchange_code_for_tokens(
            &client,
            &format!("{base}/token"),
            "test-client-id",
            "auth-code",
            "http://localhost:12345",
            "the-verifier",
        )
        .await
        .unwrap();

        assert_eq!(tokens.access_token, "ms-access-token");
        assert_eq!(tokens.refresh_token, "ms-refresh-token");
    }

    #[tokio::test]
    async fn refresh_microsoft_tokens_parses_a_fresh_pair() {
        let response_json = serde_json::json!({
            "access_token": "new-access-token",
            "refresh_token": "new-refresh-token",
            "expires_in": 3600
        });
        let mut files = HashMap::new();
        files.insert(
            "/token".to_string(),
            serde_json::to_vec(&response_json).unwrap(),
        );
        let base = spawn_mock_server(files).await;

        let client = reqwest::Client::new();
        let tokens = refresh_microsoft_tokens(
            &client,
            &format!("{base}/token"),
            "client-id",
            "old-refresh-token",
        )
        .await
        .unwrap();

        assert_eq!(tokens.access_token, "new-access-token");
        assert_eq!(tokens.refresh_token, "new-refresh-token");
    }

    #[tokio::test]
    async fn post_token_endpoint_surfaces_the_error_description_on_a_real_400() {
        let error_json = serde_json::json!({
            "error": "invalid_grant",
            "error_description": "AADSTS70008: The provided authorization code has expired."
        });
        let mut files = HashMap::new();
        files.insert("/token".to_string(), MockResponse::json(400, &error_json));
        let base = spawn_mock_server_with_status(files).await;

        let client = reqwest::Client::new();
        let err = refresh_microsoft_tokens(
            &client,
            &format!("{base}/token"),
            "client-id",
            "expired-token",
        )
        .await
        .unwrap_err();

        match err {
            AccountError::Provider(message) => assert!(message.contains("expired")),
            other => panic!("expected a Provider error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn login_with_xbox_builds_the_xbl3_identity_token_and_parses_the_access_token() {
        let response_json = serde_json::json!({
            "username": "deadbeef",
            "roles": [],
            "access_token": "mc-access-token",
            "token_type": "Bearer",
            "expires_in": 86400
        });
        let mut files = HashMap::new();
        files.insert(
            "/login".to_string(),
            serde_json::to_vec(&response_json).unwrap(),
        );
        let base = spawn_mock_server(files).await;

        let client = reqwest::Client::new();
        let access_token = login_with_xbox(&client, &format!("{base}/login"), &fake_xsts())
            .await
            .unwrap();
        assert_eq!(access_token, "mc-access-token");
    }

    #[tokio::test]
    async fn check_owns_minecraft_succeeds_when_entitlements_are_present() {
        let response_json = serde_json::json!({
            "items": [ { "name": "product_minecraft", "signature": "..." } ],
            "signature": "..."
        });
        let mut files = HashMap::new();
        files.insert(
            "/entitlements".to_string(),
            serde_json::to_vec(&response_json).unwrap(),
        );
        let base = spawn_mock_server(files).await;

        let client = reqwest::Client::new();
        check_owns_minecraft(&client, &format!("{base}/entitlements"), "mc-token")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn check_owns_minecraft_reports_no_license_when_entitlements_are_empty() {
        let mut files = HashMap::new();
        files.insert(
            "/entitlements".to_string(),
            serde_json::to_vec(&serde_json::json!({ "items": [] })).unwrap(),
        );
        let base = spawn_mock_server(files).await;

        let client = reqwest::Client::new();
        let err = check_owns_minecraft(&client, &format!("{base}/entitlements"), "mc-token")
            .await
            .unwrap_err();
        assert!(matches!(err, AccountError::NoMinecraftLicense));
    }

    #[tokio::test]
    async fn fetch_profile_extracts_uuid_username_and_active_skin() {
        let response_json = serde_json::json!({
            "id": "00000000000000000000000000000001",
            "name": "Steve",
            "skins": [
                { "id": "a", "state": "INACTIVE", "url": "https://textures.minecraft.net/old" },
                { "id": "b", "state": "ACTIVE", "url": "https://textures.minecraft.net/current" }
            ],
            "capes": []
        });
        let mut files = HashMap::new();
        files.insert(
            "/profile".to_string(),
            serde_json::to_vec(&response_json).unwrap(),
        );
        let base = spawn_mock_server(files).await;

        let client = reqwest::Client::new();
        let profile = fetch_profile(&client, &format!("{base}/profile"), "mc-token")
            .await
            .unwrap();

        assert_eq!(profile.uuid, "00000000000000000000000000000001");
        assert_eq!(profile.username, "Steve");
        assert_eq!(
            profile.skin_url.as_deref(),
            Some("https://textures.minecraft.net/current")
        );
    }

    #[tokio::test]
    async fn fetch_profile_reports_a_clear_message_when_no_profile_exists_yet() {
        let mut files = HashMap::new();
        files.insert(
            "/profile".to_string(),
            MockResponse {
                status: 404,
                body: Vec::new(),
            },
        );
        let base = spawn_mock_server_with_status(files).await;

        let client = reqwest::Client::new();
        let err = fetch_profile(&client, &format!("{base}/profile"), "mc-token")
            .await
            .unwrap_err();
        match err {
            AccountError::Provider(message) => assert!(message.contains("minecraft.net")),
            other => panic!("expected a Provider error, got {other:?}"),
        }
    }
}
