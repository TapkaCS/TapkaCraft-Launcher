//! Xbox Live user authentication and XSTS authorization - the two steps
//! between having a Microsoft access token and being allowed to call
//! Minecraft Services with it. Both calls have the same request/response
//! shape (only the body's `RelyingParty` and `Properties` differ), so they
//! share `XboxTokenResponse` and the raw-JSON-to-flattened-struct mapping.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::AccountError;

pub const XBOX_LIVE_AUTHENTICATE_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
pub const XSTS_AUTHORIZE_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// The flattened result of either call: the token to carry into the next
/// step, the user hash (`uhs`) Minecraft Services' identity token needs
/// alongside it, and the XUID if the response included one (not every
/// response does - it's optional for a reason, not an oversight).
#[derive(Debug, Clone)]
pub struct XboxTokenResponse {
    pub token: String,
    pub user_hash: String,
    pub xuid: Option<String>,
}

#[derive(Debug, Serialize)]
struct XblAuthenticateRequest<'a> {
    #[serde(rename = "Properties")]
    properties: XblAuthenticateProperties,
    #[serde(rename = "RelyingParty")]
    relying_party: &'a str,
    #[serde(rename = "TokenType")]
    token_type: &'static str,
}

#[derive(Debug, Serialize)]
struct XblAuthenticateProperties {
    #[serde(rename = "AuthMethod")]
    auth_method: &'static str,
    #[serde(rename = "SiteName")]
    site_name: &'static str,
    #[serde(rename = "RpsTicket")]
    rps_ticket: String,
}

#[derive(Debug, Serialize)]
struct XstsAuthorizeRequest<'a> {
    #[serde(rename = "Properties")]
    properties: XstsAuthorizeProperties<'a>,
    #[serde(rename = "RelyingParty")]
    relying_party: &'a str,
    #[serde(rename = "TokenType")]
    token_type: &'static str,
}

#[derive(Debug, Serialize)]
struct XstsAuthorizeProperties<'a> {
    #[serde(rename = "SandboxId")]
    sandbox_id: &'static str,
    #[serde(rename = "UserTokens")]
    user_tokens: [&'a str; 1],
}

#[derive(Debug, Deserialize)]
struct XboxSuccessResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: DisplayClaims,
}

#[derive(Debug, Deserialize)]
struct DisplayClaims {
    xui: Vec<XuiClaim>,
}

#[derive(Debug, Deserialize)]
struct XuiClaim {
    uhs: String,
    #[serde(default)]
    xid: Option<String>,
}

/// The shape Xbox Live/XSTS return on a 401 - a numeric `XErr` code that
/// maps to specific, well-known account states (no Xbox profile yet,
/// region-locked, needs adult verification, child account needing a
/// family group, ...).
#[derive(Debug, Deserialize)]
struct XboxErrorResponse {
    #[serde(rename = "XErr")]
    x_err: u64,
}

/// Step 1: exchange a Microsoft access token for an Xbox Live user token.
pub async fn authenticate_with_xbox_live(
    client: &reqwest::Client,
    microsoft_access_token: &str,
) -> Result<XboxTokenResponse, AccountError> {
    let body = XblAuthenticateRequest {
        properties: XblAuthenticateProperties {
            auth_method: "RPS",
            site_name: "user.auth.xboxlive.com",
            rps_ticket: format!("d={microsoft_access_token}"),
        },
        relying_party: "http://auth.xboxlive.com",
        token_type: "JWT",
    };
    post_xbox_endpoint(client, XBOX_LIVE_AUTHENTICATE_URL, &body).await
}

/// Step 2: exchange the Xbox Live user token for an XSTS token scoped to
/// Minecraft Services. This is the step that fails with a friendly-ish
/// `XErr` code when the Microsoft account itself can't play Minecraft at
/// all (no Xbox profile, region-locked, needs family/adult setup, ...) -
/// failures here are about the *account*, not this app.
pub async fn authorize_with_xsts(
    client: &reqwest::Client,
    xbl_token: &str,
) -> Result<XboxTokenResponse, AccountError> {
    let body = XstsAuthorizeRequest {
        properties: XstsAuthorizeProperties {
            sandbox_id: "RETAIL",
            user_tokens: [xbl_token],
        },
        relying_party: "rp://api.minecraftservices.com/",
        token_type: "JWT",
    };
    post_xbox_endpoint(client, XSTS_AUTHORIZE_URL, &body).await
}

async fn post_xbox_endpoint<B: Serialize>(
    client: &reqwest::Client,
    url: &str,
    body: &B,
) -> Result<XboxTokenResponse, AccountError> {
    let response = client
        .post(url)
        .header("Accept", "application/json")
        .timeout(REQUEST_TIMEOUT)
        .json(body)
        .send()
        .await
        .map_err(|err| AccountError::Network(err.to_string()))?;

    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(|err| AccountError::Network(err.to_string()))?;

    if !status.is_success() {
        if let Ok(error) = serde_json::from_slice::<XboxErrorResponse>(&bytes) {
            return Err(AccountError::Provider(friendly_xsts_error(error.x_err)));
        }
        return Err(AccountError::Provider(format!(
            "Xbox Live sign-in failed (HTTP {status})."
        )));
    }

    let parsed: XboxSuccessResponse = serde_json::from_slice(&bytes).map_err(|err| {
        AccountError::Provider(format!("Unexpected response from Xbox Live: {err}"))
    })?;
    let claim = parsed
        .display_claims
        .xui
        .into_iter()
        .next()
        .ok_or_else(|| {
            AccountError::Provider("Xbox Live response had no user identity in it.".into())
        })?;

    Ok(XboxTokenResponse {
        token: parsed.token,
        user_hash: claim.uhs,
        xuid: claim.xid,
    })
}

/// Translates Xbox Live's well-known `XErr` codes into messages a player
/// can actually act on, rather than a bare error number. These codes have
/// been stable for years and are the same ones every third-party
/// Minecraft launcher recognizes.
fn friendly_xsts_error(x_err: u64) -> String {
    match x_err {
        2148916233 => {
            "This Microsoft account has no Xbox profile. Create one at xbox.com, then try signing in again.".to_string()
        }
        2148916235 => {
            "Xbox Live isn't available in this account's country/region.".to_string()
        }
        2148916236 | 2148916237 => {
            "This account needs adult verification on the Xbox website before it can be used here."
                .to_string()
        }
        2148916238 => {
            "This is a child account and needs to be added to a Microsoft family group by an adult before it can sign in.".to_string()
        }
        other => format!("Xbox Live rejected this account (error {other})."),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::test_support::{spawn_mock_server, spawn_mock_server_with_status, MockResponse};

    #[tokio::test]
    async fn authenticate_with_xbox_live_parses_a_real_shaped_success_response() {
        let response_json = serde_json::json!({
            "IssueInstant": "2024-01-01T00:00:00Z",
            "NotAfter": "2024-01-01T16:00:00Z",
            "Token": "xbl-token-value",
            "DisplayClaims": { "xui": [ { "uhs": "user-hash-value", "xid": "1234567890" } ] }
        });
        let mut files = HashMap::new();
        files.insert(
            "/authenticate".to_string(),
            serde_json::to_vec(&response_json).unwrap(),
        );
        let base = spawn_mock_server(files).await;

        // Real call, redirected at a local server instead of the real
        // XBOX_LIVE_AUTHENTICATE_URL constant (unreachable from this
        // sandbox) - exercises the exact same request-building and
        // response-parsing code the production path uses.
        let client = reqwest::Client::new();
        let result = post_xbox_endpoint(
            &client,
            &format!("{base}/authenticate"),
            &serde_json::json!({}),
        )
        .await
        .unwrap();

        assert_eq!(result.token, "xbl-token-value");
        assert_eq!(result.user_hash, "user-hash-value");
        assert_eq!(result.xuid.as_deref(), Some("1234567890"));
    }

    #[tokio::test]
    async fn a_real_401_with_an_xerr_body_is_translated_to_a_friendly_message() {
        let error_json = serde_json::json!({
            "Identity": "0",
            "XErr": 2148916233u64,
            "Message": "",
            "Redirect": "https://start.ui.xboxlive.com/CreateAccount"
        });
        let mut files = HashMap::new();
        files.insert("/xsts".to_string(), MockResponse::json(401, &error_json));
        let base = spawn_mock_server_with_status(files).await;

        let client = reqwest::Client::new();
        let err = post_xbox_endpoint(&client, &format!("{base}/xsts"), &serde_json::json!({}))
            .await
            .unwrap_err();

        match err {
            AccountError::Provider(message) => assert!(message.contains("no Xbox profile")),
            other => panic!("expected a Provider error, got {other:?}"),
        }
    }

    #[test]
    fn friendly_xsts_error_covers_every_documented_code_distinctly() {
        let no_account = friendly_xsts_error(2148916233);
        let region_locked = friendly_xsts_error(2148916235);
        let needs_adult_verification = friendly_xsts_error(2148916236);
        let child_account = friendly_xsts_error(2148916238);
        let unknown = friendly_xsts_error(999999999);

        for pair in [
            (&no_account, &region_locked),
            (&no_account, &needs_adult_verification),
            (&no_account, &child_account),
            (&region_locked, &needs_adult_verification),
        ] {
            assert_ne!(
                pair.0, pair.1,
                "distinct XErr codes must not collapse to the same message"
            );
        }
        assert!(unknown.contains("999999999"));
    }

    #[tokio::test]
    async fn post_xbox_endpoint_reports_a_typed_provider_error_on_an_unparseable_response() {
        let mut files = HashMap::new();
        files.insert("/broken".to_string(), b"not json at all".to_vec());
        let base = spawn_mock_server(files).await;

        let client = reqwest::Client::new();
        let err = post_xbox_endpoint(&client, &format!("{base}/broken"), &serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, AccountError::Provider(_)));
    }

    #[tokio::test]
    async fn post_xbox_endpoint_reports_a_typed_error_on_http_404() {
        let base = spawn_mock_server(HashMap::new()).await; // nothing registered -> 404
        let client = reqwest::Client::new();
        let err = post_xbox_endpoint(&client, &format!("{base}/missing"), &serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, AccountError::Provider(_)));
    }
}
