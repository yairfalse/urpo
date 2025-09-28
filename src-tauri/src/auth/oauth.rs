//! OAuth provider trait and common types
//!
//! This trait allows us to easily add new OAuth providers (Google, Okta, Auth0, etc.)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// OAuth provider trait - implement this for each provider
#[async_trait]
pub trait OAuthProvider: Send + Sync {
    /// Get the provider name (e.g., "github", "google")
    fn name(&self) -> &str;

    /// Get the authorization URL for user to visit
    fn get_auth_url(&self, state: &str) -> String;

    /// Exchange authorization code for access token
    async fn exchange_code(&self, code: String) -> Result<String, crate::auth::AuthError>;

    /// Get user information using access token
    async fn get_user_info(&self, token: &str) -> Result<UserInfo, crate::auth::AuthError>;

    /// Revoke the access token (optional)
    async fn revoke_token(&self, token: &str) -> Result<(), crate::auth::AuthError> {
        // Default no-op implementation
        Ok(())
    }

    /// Get OAuth scopes required
    fn scopes(&self) -> Vec<&str> {
        vec!["user:email", "read:user"]
    }
}

/// OAuth configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub auth_url: String,
    pub token_url: String,
    pub scopes: Vec<String>,
}

/// User information returned from OAuth provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    /// Unique user ID from provider
    pub id: String,
    /// Username/login
    pub username: String,
    /// Display name
    pub name: Option<String>,
    /// Email address
    pub email: Option<String>,
    /// Profile picture URL
    pub avatar_url: Option<String>,
    /// Provider name (github, google, etc.)
    pub provider: String,
    /// Additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// OAuth token response
#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: Option<String>,
    pub scope: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
}

/// OAuth error response
#[derive(Debug, Deserialize)]
pub struct OAuthErrorResponse {
    pub error: String,
    pub error_description: Option<String>,
    pub error_uri: Option<String>,
}