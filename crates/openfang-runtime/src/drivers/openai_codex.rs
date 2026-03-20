//! OpenAI Codex OAuth driver — uses PKCE-authenticated tokens for the Codex Responses API.
//!
//! Wraps the OpenAI-compatible driver with automatic OAuth token management.
//! Authenticates via the Codex PKCE flow (browser-based), stores and auto-refreshes
//! tokens, then delegates LLM calls to the OpenAI driver pointed at the Codex
//! Responses API (`chatgpt.com/backend-api/codex`).

use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::debug;
use zeroize::Zeroizing;

use crate::codex_oauth::{self, CodexTokens, CODEX_API_BASE_URL};
use crate::llm_driver::{CompletionRequest, CompletionResponse, LlmDriver, LlmError, StreamEvent};

/// Cached Codex API token with pre-computed expiry.
#[derive(Clone)]
struct CachedCodexAuth {
    /// Access token for the Codex API (zeroized on drop).
    access_token: Zeroizing<String>,
    /// ChatGPT account ID for request headers.
    account_id: Option<String>,
    /// When we should refresh (before actual expiry).
    refresh_at: Instant,
    /// Refresh token for obtaining new access tokens.
    #[allow(dead_code)]
    refresh_token: Option<Zeroizing<String>>,
}

impl CachedCodexAuth {
    fn is_valid(&self) -> bool {
        Instant::now() < self.refresh_at
    }
}

/// Thread-safe auth cache.
struct CodexAuthCache {
    cached: Mutex<Option<CachedCodexAuth>>,
}

impl CodexAuthCache {
    fn new() -> Self {
        Self {
            cached: Mutex::new(None),
        }
    }

    fn get(&self) -> Option<CachedCodexAuth> {
        let lock = self.cached.lock().unwrap_or_else(|e| e.into_inner());
        lock.as_ref().filter(|t| t.is_valid()).cloned()
    }

    fn set(&self, auth: CachedCodexAuth) {
        let mut lock = self.cached.lock().unwrap_or_else(|e| e.into_inner());
        *lock = Some(auth);
    }
}

impl Default for CodexAuthCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert stored `CodexTokens` into a cached auth entry.
fn tokens_to_cached(tokens: &CodexTokens) -> CachedCodexAuth {
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Calculate remaining TTL, minimum 60 seconds
    let ttl_secs = if tokens.expires_at > now_unix {
        (tokens.expires_at - now_unix).max(60)
    } else {
        60
    };

    // Refresh 5 minutes before expiry
    let refresh_buffer = 300u64.min(ttl_secs / 2);
    let refresh_at = Instant::now() + Duration::from_secs(ttl_secs.saturating_sub(refresh_buffer));

    CachedCodexAuth {
        access_token: Zeroizing::new(tokens.access_token.clone()),
        account_id: tokens.account_id.clone(),
        refresh_at,
        refresh_token: tokens
            .refresh_token
            .as_ref()
            .map(|r| Zeroizing::new(r.clone())),
    }
}

/// LLM driver for OpenAI Codex via OAuth PKCE authentication.
///
/// On each API call, ensures valid OAuth tokens are available (loading from
/// storage or refreshing as needed), then delegates to an OpenAI-compatible
/// driver pointed at the Codex Responses API.
pub struct CodexOAuthDriver {
    auth_cache: CodexAuthCache,
}

impl CodexOAuthDriver {
    /// Create a new Codex OAuth driver.
    ///
    /// Attempts to load existing tokens from storage immediately.
    pub fn new() -> Self {
        let driver = Self {
            auth_cache: CodexAuthCache::new(),
        };

        // Pre-populate cache from stored tokens
        if let Some(tokens) = codex_oauth::load_tokens()
            .or_else(codex_oauth::load_codex_cli_tokens)
        {
            if tokens.is_valid() {
                driver.auth_cache.set(tokens_to_cached(&tokens));
                debug!("Loaded existing Codex OAuth tokens");
            }
        }

        driver
    }

    /// Get valid authentication, refreshing if needed.
    async fn ensure_auth(&self) -> Result<CachedCodexAuth, LlmError> {
        // Check cache first
        if let Some(cached) = self.auth_cache.get() {
            return Ok(cached);
        }

        debug!("Codex OAuth token expired or missing, attempting refresh...");

        // Try to get valid tokens (with auto-refresh)
        let tokens = codex_oauth::get_valid_tokens().await.ok_or_else(|| {
            LlmError::AuthenticationFailed(
                "Codex OAuth tokens not available. Please authenticate via: \
                 POST /api/providers/openai-codex/oauth/start"
                    .to_string(),
            )
        })?;

        let cached = tokens_to_cached(&tokens);
        self.auth_cache.set(cached.clone());

        Ok(cached)
    }

    /// Create a fresh OpenAI driver configured for the Codex Responses API.
    fn make_inner_driver(&self, auth: &CachedCodexAuth) -> super::openai::OpenAIDriver {
        let mut headers = vec![
            (
                "OpenAI-Beta".to_string(),
                "responses=experimental".to_string(),
            ),
            ("originator".to_string(), "pi".to_string()),
        ];

        // Add account ID header if available
        if let Some(ref account_id) = auth.account_id {
            headers.push(("chatgpt-account-id".to_string(), account_id.clone()));
        }

        super::openai::OpenAIDriver::new(
            auth.access_token.to_string(),
            CODEX_API_BASE_URL.to_string(),
        )
        .with_extra_headers(headers)
    }
}

impl Default for CodexOAuthDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl LlmDriver for CodexOAuthDriver {
    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, LlmError> {
        let auth = self.ensure_auth().await?;
        let driver = self.make_inner_driver(&auth);
        driver.complete(request).await
    }

    async fn stream(
        &self,
        request: CompletionRequest,
        tx: tokio::sync::mpsc::Sender<StreamEvent>,
    ) -> Result<CompletionResponse, LlmError> {
        let auth = self.ensure_auth().await?;
        let driver = self.make_inner_driver(&auth);
        driver.stream(request, tx).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokens_to_cached() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let tokens = CodexTokens {
            access_token: "test-access".to_string(),
            refresh_token: Some("test-refresh".to_string()),
            id_token: None,
            account_id: Some("acc_123".to_string()),
            expires_at: now + 3600,
        };

        let cached = tokens_to_cached(&tokens);
        assert_eq!(*cached.access_token, "test-access");
        assert_eq!(cached.account_id, Some("acc_123".to_string()));
        assert!(cached.is_valid());
        assert!(cached.refresh_token.is_some());
    }

    #[test]
    fn test_auth_cache_empty() {
        let cache = CodexAuthCache::new();
        assert!(cache.get().is_none());
    }

    #[test]
    fn test_auth_cache_set_get() {
        let cache = CodexAuthCache::new();
        let auth = CachedCodexAuth {
            access_token: Zeroizing::new("token".to_string()),
            account_id: Some("acc".to_string()),
            refresh_at: Instant::now() + Duration::from_secs(3600),
            refresh_token: None,
        };
        cache.set(auth);
        let cached = cache.get();
        assert!(cached.is_some());
        assert_eq!(*cached.unwrap().access_token, "token");
    }

    #[test]
    fn test_auth_cache_expired() {
        let cache = CodexAuthCache::new();
        let auth = CachedCodexAuth {
            access_token: Zeroizing::new("token".to_string()),
            account_id: None,
            refresh_at: Instant::now() - Duration::from_secs(1), // already expired
            refresh_token: None,
        };
        cache.set(auth);
        assert!(cache.get().is_none()); // should not return expired
    }

    #[test]
    fn test_codex_api_base_url() {
        assert_eq!(CODEX_API_BASE_URL, "https://chatgpt.com/backend-api/codex");
    }
}
