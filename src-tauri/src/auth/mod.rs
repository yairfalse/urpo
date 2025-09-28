//! Authentication module with extensible OAuth provider support
//!
//! Designed to support multiple OAuth providers (GitHub, Google, Okta, etc.)
//! with secure token storage and proper error handling.

pub mod oauth;
pub mod providers;
pub mod storage;
pub mod commands;

pub use oauth::{OAuthProvider, OAuthConfig, UserInfo};
pub use providers::github::GitHubProvider;
pub use storage::SecureTokenStorage;
pub use commands::{login_with_github, logout, get_current_user};

use std::sync::Arc;
use tokio::sync::Mutex;

/// Global authentication state
pub struct AuthState {
    /// Current OAuth provider
    provider: Arc<Mutex<Box<dyn OAuthProvider>>>,
    /// Secure token storage
    storage: Arc<SecureTokenStorage>,
    /// OAuth callback server handle
    server_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl AuthState {
    /// Create new auth state with GitHub as default provider
    pub fn new() -> Self {
        let storage = Arc::new(SecureTokenStorage::new("urpo"));
        let provider = GitHubProvider::new(
            std::env::var("GITHUB_CLIENT_ID").unwrap_or_else(|_| "YOUR_CLIENT_ID".to_string()),
            std::env::var("GITHUB_CLIENT_SECRET").unwrap_or_else(|_| "YOUR_SECRET".to_string()),
        );

        Self {
            provider: Arc::new(Mutex::new(Box::new(provider))),
            storage,
            server_handle: Arc::new(Mutex::new(None)),
        }
    }

    /// Switch to a different OAuth provider
    pub async fn set_provider(&self, provider: Box<dyn OAuthProvider>) {
        let mut current = self.provider.lock().await;
        *current = provider;
    }
}

/// Authentication result
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AuthResult {
    pub user: UserInfo,
    pub token: String,
    pub provider: String,
}

/// Authentication error types
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("OAuth flow failed: {0}")]
    OAuthError(String),

    #[error("Token storage error: {0}")]
    StorageError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("User cancelled authentication")]
    UserCancelled,

    #[error("Invalid configuration: {0}")]
    ConfigError(String),
}

impl From<AuthError> for String {
    fn from(err: AuthError) -> String {
        err.to_string()
    }
}