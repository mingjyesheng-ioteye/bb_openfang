//! OpenAI Codex OAuth — PKCE authorization code flow for obtaining Codex API tokens.
//!
//! Implements OAuth 2.0 Authorization Code Grant with PKCE (RFC 7636) for the
//! OpenAI Codex API. Users authenticate via browser, and the resulting tokens
//! are stored and auto-refreshed. Compatible with Codex CLI credentials at
//! `~/.codex/auth.json`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{debug, warn};

// ── Constants ────────────────────────────────────────────────────────

/// OpenAI OAuth authorization URL.
const CODEX_AUTH_URL: &str = "https://auth.openai.com/oauth/authorize";

/// OpenAI OAuth token URL.
const CODEX_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";

/// Public OAuth client ID — same as Codex CLI.
const CODEX_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";

/// Default redirect URI — points to the OpenFang API server's callback endpoint.
const CODEX_REDIRECT_URI: &str = "http://localhost:4200/api/providers/openai-codex/oauth/callback";

/// Required scopes for Codex API access.
const CODEX_SCOPES: &str = "openid profile email offline_access";

/// Codex Responses API base URL.
pub const CODEX_API_BASE_URL: &str = "https://chatgpt.com/backend-api/codex";

/// Token refresh buffer — refresh this many seconds before expiry.
const REFRESH_BUFFER_SECS: u64 = 300; // 5 minutes

// ── Types ────────────────────────────────────────────────────────────

/// OAuth tokens stored for the Codex provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexTokens {
    /// OAuth access token (zeroized-on-use where possible).
    pub access_token: String,
    /// OAuth refresh token for obtaining new access tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// OpenID Connect ID token (contains account info).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
    /// ChatGPT account ID extracted from JWT claims.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    /// Unix timestamp when the access token expires.
    #[serde(default)]
    pub expires_at: u64,
}

impl CodexTokens {
    /// Check if the token is still valid (with refresh buffer).
    pub fn is_valid(&self) -> bool {
        if self.expires_at == 0 {
            return false;
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.expires_at > now + REFRESH_BUFFER_SECS
    }
}

/// PKCE challenge pair.
pub struct PkceChallenge {
    pub verifier: String,
    pub challenge: String,
}

/// Result of starting an OAuth PKCE flow.
pub struct CodexAuthUrl {
    /// Full authorization URL to open in the browser.
    pub url: String,
    /// PKCE verifier (must be kept for the token exchange step).
    pub verifier: String,
    /// State parameter for CSRF protection.
    pub state: String,
}

/// Status of a Codex OAuth flow.
pub enum CodexFlowStatus {
    /// Waiting for the user to complete browser auth.
    WaitingForCallback,
    /// Authorization succeeded — contains the tokens.
    Complete(CodexTokens),
    /// The callback timed out.
    Timeout,
    /// An error occurred.
    Error(String),
}

// ── PKCE ─────────────────────────────────────────────────────────────

/// Generate a PKCE S256 challenge pair.
pub fn generate_pkce() -> PkceChallenge {
    let random_bytes: [u8; 32] = rand::random();
    let verifier = base64_url_encode(&random_bytes);
    let hash = Sha256::digest(verifier.as_bytes());
    let challenge = base64_url_encode(&hash);
    PkceChallenge {
        verifier,
        challenge,
    }
}

/// Generate a random state parameter.
pub fn generate_state() -> String {
    let random_bytes: [u8; 16] = rand::random();
    base64_url_encode(&random_bytes)
}

/// Base64 URL-safe encoding without padding.
fn base64_url_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

/// Base64 URL-safe decoding (handles both padded and unpadded).
fn base64_url_decode(data: &str) -> Result<Vec<u8>, String> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(data)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(data))
        .or_else(|_| base64::engine::general_purpose::STANDARD.decode(data))
        .map_err(|e| format!("Base64 decode error: {e}"))
}

// ── Authorization URL ────────────────────────────────────────────────

/// Build the authorization URL for the Codex OAuth PKCE flow.
///
/// Returns the URL to open in the browser, plus the PKCE verifier and state
/// that must be preserved for the token exchange step.
pub fn build_auth_url() -> CodexAuthUrl {
    let pkce = generate_pkce();
    let state = generate_state();

    let params = [
        ("response_type", "code"),
        ("client_id", CODEX_CLIENT_ID),
        ("redirect_uri", CODEX_REDIRECT_URI),
        ("scope", CODEX_SCOPES),
        ("state", &state),
        ("code_challenge", &pkce.challenge),
        ("code_challenge_method", "S256"),
        // Extra params for Codex compatibility
        ("response_mode", "query"),
        ("prompt", "login"),
        ("audience", "https://api.openai.com/v1"),
        ("originator", "pi"),
    ];

    let query = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    CodexAuthUrl {
        url: format!("{}?{}", CODEX_AUTH_URL, query),
        verifier: pkce.verifier,
        state,
    }
}

// ── Token Exchange ───────────────────────────────────────────────────

/// Exchange an authorization code for tokens.
///
/// POST to the token endpoint with the code + PKCE verifier.
pub async fn exchange_code(code: &str, verifier: &str) -> Result<CodexTokens, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    debug!("Exchanging Codex authorization code for tokens");

    let resp = client
        .post(CODEX_TOKEN_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", CODEX_CLIENT_ID),
            ("code", code),
            ("redirect_uri", CODEX_REDIRECT_URI),
            ("code_verifier", verifier),
        ])
        .send()
        .await
        .map_err(|e| format!("Token exchange request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Token exchange returned {status}: {body}"));
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse token response: {e}"))?;

    parse_token_response(&body)
}

/// Refresh an expired access token using the refresh token.
pub async fn refresh_tokens(refresh_token: &str) -> Result<CodexTokens, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    debug!("Refreshing Codex OAuth tokens");

    let resp = client
        .post(CODEX_TOKEN_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", CODEX_CLIENT_ID),
            ("refresh_token", refresh_token),
        ])
        .send()
        .await
        .map_err(|e| format!("Token refresh request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Token refresh returned {status}: {body}"));
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse refresh response: {e}"))?;

    parse_token_response(&body)
}

/// Parse a token response JSON into `CodexTokens`.
fn parse_token_response(body: &serde_json::Value) -> Result<CodexTokens, String> {
    let access_token = body
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or("Missing access_token in token response")?
        .to_string();

    let refresh_token = body
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let id_token = body
        .get("id_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let expires_in = body
        .get("expires_in")
        .and_then(|v| v.as_u64())
        .unwrap_or(3600);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let expires_at = now + expires_in;

    // Extract account ID from JWT claims
    let account_id = extract_account_id(&access_token)
        .or_else(|| id_token.as_deref().and_then(extract_account_id));

    Ok(CodexTokens {
        access_token,
        refresh_token,
        id_token,
        account_id,
        expires_at,
    })
}

/// Extract ChatGPT account ID from a JWT token's claims.
///
/// Tries multiple claim paths:
/// 1. Root `chatgpt_account_id`
/// 2. Nested `https://api.openai.com/auth.chatgpt_account_id`
/// 3. `organizations[0].id`
fn extract_account_id(token: &str) -> Option<String> {
    // JWT has 3 parts separated by dots; claims are in the second part
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return None;
    }

    let claims_json = base64_url_decode(parts[1]).ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&claims_json).ok()?;

    // Try root-level chatgpt_account_id
    if let Some(id) = claims.get("chatgpt_account_id").and_then(|v| v.as_str()) {
        return Some(id.to_string());
    }

    // Try nested auth claim
    if let Some(auth) = claims.get("https://api.openai.com/auth") {
        if let Some(id) = auth.get("chatgpt_account_id").and_then(|v| v.as_str()) {
            return Some(id.to_string());
        }
    }

    // Try organizations array
    if let Some(orgs) = claims.get("organizations").and_then(|v| v.as_array()) {
        if let Some(org) = orgs.first() {
            if let Some(id) = org.get("id").and_then(|v| v.as_str()) {
                return Some(id.to_string());
            }
        }
    }

    None
}

// NOTE: The OAuth callback server is handled by the openfang-api crate
// via the `GET /api/providers/openai-codex/oauth/callback` route.
// The redirect URI in the PKCE flow points to the API server's callback
// endpoint, which receives the authorization code and exchanges it for tokens.

// ── Token Storage ────────────────────────────────────────────────────

/// Get the token storage path.
fn token_storage_path() -> std::path::PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home)
        .join(".openfang")
        .join("codex_tokens.json")
}

/// Load stored Codex tokens from disk.
pub fn load_tokens() -> Option<CodexTokens> {
    let path = token_storage_path();
    let data = std::fs::read_to_string(&path).ok()?;
    let tokens: CodexTokens = serde_json::from_str(&data).ok()?;
    Some(tokens)
}

/// Save Codex tokens to disk.
pub fn save_tokens(tokens: &CodexTokens) -> Result<(), String> {
    let path = token_storage_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create token directory: {e}"))?;
    }

    let json = serde_json::to_string_pretty(tokens)
        .map_err(|e| format!("Failed to serialize tokens: {e}"))?;

    std::fs::write(&path, &json).map_err(|e| format!("Failed to write tokens: {e}"))?;

    // Set restrictive permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        let _ = std::fs::set_permissions(&path, perms);
    }

    debug!("Saved Codex OAuth tokens to {}", path.display());
    Ok(())
}

/// Delete stored Codex tokens.
pub fn delete_tokens() -> Result<(), String> {
    let path = token_storage_path();
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("Failed to delete tokens: {e}"))?;
    }
    Ok(())
}

// ── Codex CLI Compatibility ──────────────────────────────────────────

/// Load tokens from the Codex CLI auth file (`~/.codex/auth.json`).
///
/// The Codex CLI stores credentials in a different format. This function
/// reads and converts them for use with the OpenFang Codex driver.
pub fn load_codex_cli_tokens() -> Option<CodexTokens> {
    let codex_home = std::env::var("CODEX_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .unwrap_or_else(|_| ".".to_string());
            std::path::PathBuf::from(home).join(".codex")
        });

    let auth_path = codex_home.join("auth.json");
    let data = std::fs::read_to_string(&auth_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&data).ok()?;

    // Codex CLI format: {"access_token": "...", "refresh_token": "...", "expires_at": ...}
    // or: {"token": "...", ...}
    let access_token = json
        .get("access_token")
        .or_else(|| json.get("token"))
        .and_then(|v| v.as_str())?
        .to_string();

    let refresh_token = json
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let id_token = json
        .get("id_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let expires_at = json
        .get("expires_at")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let account_id = json
        .get("account_id")
        .or_else(|| json.get("chatgpt_account_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| extract_account_id(&access_token));

    Some(CodexTokens {
        access_token,
        refresh_token,
        id_token,
        account_id,
        expires_at,
    })
}

// ── Get Valid Tokens (with auto-refresh) ─────────────────────────────

/// Get valid Codex tokens, auto-refreshing if expired.
///
/// Resolution order:
/// 1. Stored OpenFang tokens (with auto-refresh)
/// 2. Codex CLI tokens (`~/.codex/auth.json`)
/// 3. None
pub async fn get_valid_tokens() -> Option<CodexTokens> {
    // Try OpenFang stored tokens first
    if let Some(tokens) = load_tokens() {
        if tokens.is_valid() {
            return Some(tokens);
        }
        // Try refreshing
        if let Some(ref refresh) = tokens.refresh_token {
            match refresh_tokens(refresh).await {
                Ok(new_tokens) => {
                    if let Err(e) = save_tokens(&new_tokens) {
                        warn!("Failed to save refreshed Codex tokens: {e}");
                    }
                    return Some(new_tokens);
                }
                Err(e) => {
                    warn!("Codex token refresh failed: {e}");
                }
            }
        }
    }

    // Fall back to Codex CLI tokens
    if let Some(cli_tokens) = load_codex_cli_tokens() {
        if cli_tokens.is_valid() {
            return Some(cli_tokens);
        }
        // Try refreshing CLI tokens too
        if let Some(ref refresh) = cli_tokens.refresh_token {
            match refresh_tokens(refresh).await {
                Ok(new_tokens) => {
                    if let Err(e) = save_tokens(&new_tokens) {
                        warn!("Failed to save refreshed Codex CLI tokens: {e}");
                    }
                    return Some(new_tokens);
                }
                Err(e) => {
                    warn!("Codex CLI token refresh failed: {e}");
                }
            }
        }
    }

    None
}

/// Check if Codex OAuth is available (tokens exist or Codex CLI is configured).
pub fn codex_oauth_available() -> bool {
    load_tokens().is_some() || load_codex_cli_tokens().is_some()
}

// ── URL Encoding Helper ──────────────────────────────────────────────

mod urlencoding {
    /// Percent-encode a string for use in URL query parameters.
    pub fn encode(input: &str) -> String {
        let mut result = String::with_capacity(input.len() * 3);
        for byte in input.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    result.push(byte as char);
                }
                _ => {
                    result.push('%');
                    result.push_str(&format!("{:02X}", byte));
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_generation() {
        let pkce = generate_pkce();
        assert!(!pkce.verifier.is_empty());
        assert!(!pkce.challenge.is_empty());
        assert_ne!(pkce.verifier, pkce.challenge);
    }

    #[test]
    fn test_state_generation() {
        let state = generate_state();
        assert!(!state.is_empty());
        // Two states should be different
        let state2 = generate_state();
        assert_ne!(state, state2);
    }

    #[test]
    fn test_build_auth_url() {
        let auth = build_auth_url();
        assert!(auth.url.starts_with("https://auth.openai.com/oauth/authorize"));
        assert!(auth.url.contains("code_challenge_method=S256"));
        assert!(auth.url.contains("response_type=code"));
        assert!(auth.url.contains(CODEX_CLIENT_ID));
        assert!(!auth.verifier.is_empty());
        assert!(!auth.state.is_empty());
    }

    #[test]
    fn test_extract_account_id_root() {
        // Build a minimal JWT with chatgpt_account_id at root
        let claims = serde_json::json!({"chatgpt_account_id": "acc_123"});
        let claims_b64 = base64_url_encode(claims.to_string().as_bytes());
        let token = format!("header.{}.signature", claims_b64);
        assert_eq!(extract_account_id(&token), Some("acc_123".to_string()));
    }

    #[test]
    fn test_extract_account_id_nested() {
        let claims = serde_json::json!({
            "https://api.openai.com/auth": {"chatgpt_account_id": "acc_456"}
        });
        let claims_b64 = base64_url_encode(claims.to_string().as_bytes());
        let token = format!("header.{}.signature", claims_b64);
        assert_eq!(extract_account_id(&token), Some("acc_456".to_string()));
    }

    #[test]
    fn test_extract_account_id_org() {
        let claims = serde_json::json!({
            "organizations": [{"id": "org_789"}]
        });
        let claims_b64 = base64_url_encode(claims.to_string().as_bytes());
        let token = format!("header.{}.signature", claims_b64);
        assert_eq!(extract_account_id(&token), Some("org_789".to_string()));
    }

    #[test]
    fn test_extract_account_id_none() {
        assert_eq!(extract_account_id("not-a-jwt"), None);
        assert_eq!(extract_account_id("a.b.c"), None); // invalid base64
    }

    #[test]
    fn test_token_validity() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Valid token (expires in 1 hour)
        let valid = CodexTokens {
            access_token: "t".into(),
            refresh_token: None,
            id_token: None,
            account_id: None,
            expires_at: now + 3600,
        };
        assert!(valid.is_valid());

        // Token that expires in < 5 min should be considered expired
        let almost_expired = CodexTokens {
            access_token: "t".into(),
            refresh_token: None,
            id_token: None,
            account_id: None,
            expires_at: now + 60,
        };
        assert!(!almost_expired.is_valid());

        // Already expired token
        let expired = CodexTokens {
            access_token: "t".into(),
            refresh_token: None,
            id_token: None,
            account_id: None,
            expires_at: now - 100,
        };
        assert!(!expired.is_valid());
    }

    #[test]
    fn test_parse_token_response() {
        let body = serde_json::json!({
            "access_token": "test_access",
            "refresh_token": "test_refresh",
            "id_token": "test_id",
            "expires_in": 3600,
        });
        let tokens = parse_token_response(&body).unwrap();
        assert_eq!(tokens.access_token, "test_access");
        assert_eq!(tokens.refresh_token.as_deref(), Some("test_refresh"));
        assert_eq!(tokens.id_token.as_deref(), Some("test_id"));
        assert!(tokens.expires_at > 0);
    }

    #[test]
    fn test_url_encoding() {
        assert_eq!(urlencoding::encode("hello world"), "hello%20world");
        assert_eq!(urlencoding::encode("foo+bar"), "foo%2Bbar");
        assert_eq!(urlencoding::encode("safe-chars_ok.here~"), "safe-chars_ok.here~");
    }

    #[test]
    fn test_constants() {
        assert!(CODEX_AUTH_URL.starts_with("https://"));
        assert!(CODEX_TOKEN_URL.starts_with("https://"));
        assert!(!CODEX_CLIENT_ID.is_empty());
        assert!(CODEX_REDIRECT_URI.contains("callback"));
    }
}
