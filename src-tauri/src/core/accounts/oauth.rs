//! The loopback authorization-code + PKCE flow against Microsoft's identity
//! platform (RFC 8252, "OAuth 2.0 for Native Apps") - the same pattern
//! every third-party Minecraft launcher uses. The user signs in in their
//! own system browser; this app never sees their Microsoft password, only
//! the short-lived authorization code Microsoft redirects back with.
//!
//! The `/consumers/` tenant (rather than `/common/`) is deliberate: Xbox
//! Live sign-in - the very next step after this one - only ever works with
//! personal Microsoft accounts, never work/school accounts, so there is no
//! reason to accept them here.

use std::net::SocketAddr;
use std::time::Duration;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use reqwest::Url;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use super::AccountError;

/// TapkaCraft Launcher's own Azure AD (Entra ID) app registration - a
/// public/native client, so this id is not a secret (it's inherently
/// visible in the authorization request itself, same as every other
/// third-party Minecraft launcher's).
pub const MICROSOFT_CLIENT_ID: &str = "7f7e717b-4054-4b78-a1d6-958f7935e7d1";

const AUTHORIZE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize";
pub const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const SCOPE: &str = "XboxLive.signin offline_access";
/// How long to wait for the user to finish in their browser before giving
/// up - generous, since this includes actually typing a password/2FA code,
/// not just clicking a button.
const REDIRECT_TIMEOUT: Duration = Duration::from_secs(300);

pub struct PkceChallenge {
    pub verifier: String,
    pub challenge: String,
}

/// Generates a fresh PKCE pair: a high-entropy `verifier` kept secret by
/// this app, and its SHA256-derived `challenge` sent to Microsoft up
/// front. Microsoft only issues tokens to whoever can later present the
/// matching `verifier` - proof that the token exchange is coming from the
/// same app instance that started the sign-in, not an attacker who
/// intercepted the redirect.
pub fn generate_pkce() -> PkceChallenge {
    let verifier = random_url_safe_string(32);
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());
    PkceChallenge {
        verifier,
        challenge,
    }
}

/// A per-attempt random value echoed back in the redirect - rejected if it
/// doesn't match, so a redirect from a stale or forged sign-in attempt
/// can't be replayed into this one.
pub fn generate_state() -> String {
    random_url_safe_string(16)
}

fn random_url_safe_string(byte_len: usize) -> String {
    let mut bytes = vec![0u8; byte_len];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// The URL to open in the system browser.
pub fn build_authorization_url(
    client_id: &str,
    redirect_uri: &str,
    pkce: &PkceChallenge,
    state: &str,
) -> String {
    let mut url = Url::parse(AUTHORIZE_URL).expect("AUTHORIZE_URL is a valid, fixed URL");
    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("response_mode", "query")
        .append_pair("scope", SCOPE)
        .append_pair("code_challenge", &pkce.challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", state);
    url.to_string()
}

/// A bound loopback listener plus the exact `redirect_uri` string to send
/// Microsoft, built from whatever port the OS actually handed out.
/// Deliberately two steps (bind, then wait) - the caller needs the
/// `redirect_uri` to build the authorization URL *before* opening the
/// browser, which happens before anyone can be waiting on the redirect.
pub struct LoopbackServer {
    listener: TcpListener,
    pub redirect_uri: String,
}

/// Binds `127.0.0.1:0` (the OS picks a free port) and reports the
/// `redirect_uri` to register the sign-in attempt under. Uses the literal
/// hostname `localhost` (which resolves to this same loopback address) so
/// it matches Microsoft's special "any port on localhost is allowed"
/// native-app redirect URI registration - the Azure app only has to
/// register `http://localhost` once, not a specific port.
pub async fn bind_loopback_server() -> std::io::Result<LoopbackServer> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr: SocketAddr = listener.local_addr()?;
    Ok(LoopbackServer {
        listener,
        redirect_uri: format!("http://localhost:{}", addr.port()),
    })
}

/// What Microsoft's redirect handed back.
#[derive(Debug)]
pub struct RedirectResult {
    pub code: String,
}

impl LoopbackServer {
    /// Waits for exactly one browser redirect carrying `?code=...&state=...`
    /// (or `?error=...`), replies with a small "you can close this tab"
    /// page, and returns the authorization code. Rejects a `state` that
    /// doesn't match what this attempt generated - the redirect could
    /// otherwise be a leftover from a previous attempt, or forged.
    pub async fn wait_for_redirect(
        self,
        expected_state: &str,
    ) -> Result<RedirectResult, AccountError> {
        let accept = self.listener.accept();
        let (mut socket, _) = tokio::time::timeout(REDIRECT_TIMEOUT, accept)
            .await
            .map_err(|_| AccountError::TimedOut)??;

        let mut buf = [0u8; 8192];
        let n = socket.read(&mut buf).await?;
        let request = String::from_utf8_lossy(&buf[..n]);
        let path_and_query = request.split_whitespace().nth(1).ok_or_else(|| {
            AccountError::Provider("The browser sent a request TapkaCraft couldn't read.".into())
        })?;

        let full_url = Url::parse(&format!("http://localhost{path_and_query}")).map_err(|err| {
            AccountError::Provider(format!("Couldn't parse the sign-in redirect: {err}"))
        })?;
        let params: std::collections::HashMap<String, String> = full_url
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();

        let (body, result) = if let Some(error_code) = params.get("error") {
            if error_code == "access_denied" {
                (
                    redirect_page(
                        "Sign-in cancelled",
                        "You can close this tab and return to TapkaCraft Launcher.",
                    ),
                    Err(AccountError::Cancelled),
                )
            } else {
                let description = params
                    .get("error_description")
                    .map(String::as_str)
                    .unwrap_or("Sign-in was cancelled or denied.");
                let message = format!("{description} ({error_code})");
                (
                    redirect_page("Sign-in didn't complete", description),
                    Err(AccountError::Provider(message)),
                )
            }
        } else if params.get("state").map(String::as_str) != Some(expected_state) {
            (
                redirect_page(
                    "Sign-in didn't complete",
                    "This sign-in link is no longer valid. Please try again from the launcher.",
                ),
                Err(AccountError::Provider(
                    "Redirect state did not match - a stale or forged sign-in link.".into(),
                )),
            )
        } else if let Some(code) = params.get("code") {
            (
                redirect_page(
                    "Signed in",
                    "You can close this tab and return to TapkaCraft Launcher.",
                ),
                Ok(RedirectResult { code: code.clone() }),
            )
        } else {
            (
                redirect_page(
                    "Sign-in didn't complete",
                    "No authorization code was returned. Please try again.",
                ),
                Err(AccountError::Provider(
                    "Redirect had neither `code` nor `error`.".into(),
                )),
            )
        };

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = socket.write_all(response.as_bytes()).await;

        result
    }
}

fn redirect_page(title: &str, message: &str) -> String {
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{title} - TapkaCraft Launcher</title></head>\
         <body style=\"font-family: sans-serif; text-align: center; padding: 4em;\">\
         <h1>{title}</h1><p>{message}</p></body></html>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_pkce_challenge_is_the_base64url_sha256_of_the_verifier() {
        let pkce = generate_pkce();
        assert_eq!(pkce.verifier.len(), 43); // 32 bytes, base64url no-pad
        let mut hasher = Sha256::new();
        hasher.update(pkce.verifier.as_bytes());
        let expected = URL_SAFE_NO_PAD.encode(hasher.finalize());
        assert_eq!(pkce.challenge, expected);
    }

    #[test]
    fn generate_pkce_never_repeats() {
        let a = generate_pkce();
        let b = generate_pkce();
        assert_ne!(a.verifier, b.verifier);
    }

    #[test]
    fn generate_state_never_repeats() {
        assert_ne!(generate_state(), generate_state());
    }

    #[test]
    fn authorization_url_carries_every_required_parameter_correctly_encoded() {
        let pkce = generate_pkce();
        let url = build_authorization_url(
            "test-client-id",
            "http://localhost:53219",
            &pkce,
            "the-state-value",
        );
        let parsed = Url::parse(&url).unwrap();
        let params: std::collections::HashMap<_, _> = parsed.query_pairs().collect();

        assert_eq!(params["client_id"], "test-client-id");
        assert_eq!(params["response_type"], "code");
        assert_eq!(params["redirect_uri"], "http://localhost:53219");
        assert_eq!(params["scope"], "XboxLive.signin offline_access");
        assert_eq!(params["code_challenge"], pkce.challenge);
        assert_eq!(params["code_challenge_method"], "S256");
        assert_eq!(params["state"], "the-state-value");
        assert!(
            url.starts_with("https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize")
        );
    }

    #[tokio::test]
    async fn bind_loopback_server_reports_the_port_it_actually_bound() {
        let server = bind_loopback_server().await.unwrap();
        let actual_port = server.listener.local_addr().unwrap().port();
        assert_eq!(
            server.redirect_uri,
            format!("http://localhost:{actual_port}")
        );
    }

    /// Simulates the real browser redirect end to end against a real
    /// loopback socket: connects like a browser would, sends the redirect
    /// request, and checks both the extracted code and the HTTP response
    /// the "browser" receives back.
    #[tokio::test]
    async fn wait_for_redirect_extracts_the_code_from_a_real_http_request() {
        let server = bind_loopback_server().await.unwrap();
        let redirect_uri = server.redirect_uri.clone();
        let port: u16 = redirect_uri.rsplit(':').next().unwrap().parse().unwrap();

        let browser = tokio::spawn(async move {
            let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
                .await
                .unwrap();
            stream
                .write_all(b"GET /?code=abc123&state=xyz HTTP/1.1\r\nHost: localhost\r\n\r\n")
                .await
                .unwrap();
            let mut response = Vec::new();
            stream.read_to_end(&mut response).await.unwrap();
            String::from_utf8_lossy(&response).into_owned()
        });

        let result = server.wait_for_redirect("xyz").await.unwrap();
        assert_eq!(result.code, "abc123");

        let response = browser.await.unwrap();
        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("close this tab"));
    }

    #[tokio::test]
    async fn wait_for_redirect_rejects_a_mismatched_state() {
        let server = bind_loopback_server().await.unwrap();
        let port: u16 = server
            .redirect_uri
            .rsplit(':')
            .next()
            .unwrap()
            .parse()
            .unwrap();

        tokio::spawn(async move {
            let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
                .await
                .unwrap();
            let _ = stream
                .write_all(b"GET /?code=abc123&state=wrong-state HTTP/1.1\r\n\r\n")
                .await;
        });

        let result = server.wait_for_redirect("expected-state").await;
        assert!(matches!(result, Err(AccountError::Provider(_))));
    }

    #[tokio::test]
    async fn wait_for_redirect_reports_cancellation_when_the_user_denies_consent() {
        let server = bind_loopback_server().await.unwrap();
        let port: u16 = server
            .redirect_uri
            .rsplit(':')
            .next()
            .unwrap()
            .parse()
            .unwrap();

        tokio::spawn(async move {
            let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
                .await
                .unwrap();
            let _ = stream
                .write_all(b"GET /?error=access_denied&error_description=The+user+cancelled&state=xyz HTTP/1.1\r\n\r\n")
                .await;
        });

        let result = server.wait_for_redirect("xyz").await;
        assert!(
            matches!(result, Err(AccountError::Cancelled)),
            "expected Cancelled, got {result:?}"
        );
    }

    #[tokio::test]
    async fn wait_for_redirect_surfaces_a_genuine_error_response_from_microsoft() {
        let server = bind_loopback_server().await.unwrap();
        let port: u16 = server
            .redirect_uri
            .rsplit(':')
            .next()
            .unwrap()
            .parse()
            .unwrap();

        tokio::spawn(async move {
            let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
                .await
                .unwrap();
            let _ = stream
                .write_all(b"GET /?error=server_error&error_description=Something+broke&state=xyz HTTP/1.1\r\n\r\n")
                .await;
        });

        let result = server.wait_for_redirect("xyz").await;
        match result {
            Err(AccountError::Provider(msg)) => assert!(msg.contains("Something broke")),
            other => panic!("expected a Provider error, got {other:?}"),
        }
    }
}
