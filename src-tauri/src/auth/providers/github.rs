//! GitHub OAuth provider implementation

use async_trait::async_trait;
use reqwest;
use serde::{Deserialize, Serialize};
use crate::auth::{AuthError, OAuthProvider, UserInfo};

/// GitHub OAuth provider
pub struct GitHubProvider {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

impl GitHubProvider {
    /// Create new GitHub OAuth provider
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            client_secret,
            redirect_uri: "http://localhost:8788/callback".to_string(),
        }
    }
}

#[async_trait]
impl OAuthProvider for GitHubProvider {
    fn name(&self) -> &str {
        "github"
    }

    fn get_auth_url(&self, state: &str) -> String {
        format!(
            "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope={}&state={}",
            self.client_id,
            urlencoding::encode(&self.redirect_uri),
            urlencoding::encode("read:user user:email"),
            state
        )
    }

    async fn exchange_code(&self, code: String) -> Result<String, AuthError> {
        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
            #[allow(dead_code)]
            token_type: String,
            #[allow(dead_code)]
            scope: String,
        }

        #[derive(Deserialize)]
        struct ErrorResponse {
            error: String,
            error_description: String,
        }

        let client = reqwest::Client::new();
        let response = client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .form(&[
                ("client_id", &self.client_id),
                ("client_secret", &self.client_secret),
                ("code", &code),
                ("redirect_uri", &self.redirect_uri),
            ])
            .send()
            .await
            .map_err(|e| AuthError::NetworkError(e.to_string()))?;

        let body = response
            .text()
            .await
            .map_err(|e| AuthError::NetworkError(e.to_string()))?;

        // Try to parse as success response first
        if let Ok(token_response) = serde_json::from_str::<TokenResponse>(&body) {
            Ok(token_response.access_token)
        } else if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&body) {
            Err(AuthError::OAuthError(format!(
                "{}: {}",
                error_response.error, error_response.error_description
            )))
        } else {
            Err(AuthError::OAuthError(format!("Invalid response: {}", body)))
        }
    }

    async fn get_user_info(&self, token: &str) -> Result<UserInfo, AuthError> {
        #[derive(Deserialize)]
        struct GitHubUser {
            id: i64,
            login: String,
            name: Option<String>,
            email: Option<String>,
            avatar_url: String,
            bio: Option<String>,
            company: Option<String>,
            location: Option<String>,
            blog: Option<String>,
            public_repos: i32,
            followers: i32,
            following: i32,
        }

        let client = reqwest::Client::new();
        let response = client
            .get("https://api.github.com/user")
            .header("Authorization", format!("Bearer {}", token))
            .header("User-Agent", "Urpo-Trace-Explorer")
            .send()
            .await
            .map_err(|e| AuthError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AuthError::OAuthError(format!(
                "Failed to get user info: {}",
                response.status()
            )));
        }

        let github_user: GitHubUser = response
            .json()
            .await
            .map_err(|e| AuthError::NetworkError(e.to_string()))?;

        // Get primary email if not public
        let email = if github_user.email.is_none() {
            self.get_primary_email(token).await.ok()
        } else {
            github_user.email
        };

        // Create metadata with additional GitHub info
        let metadata = serde_json::json!({
            "bio": github_user.bio,
            "company": github_user.company,
            "location": github_user.location,
            "blog": github_user.blog,
            "public_repos": github_user.public_repos,
            "followers": github_user.followers,
            "following": github_user.following,
        });

        Ok(UserInfo {
            id: github_user.id.to_string(),
            username: github_user.login,
            name: github_user.name,
            email,
            avatar_url: Some(github_user.avatar_url),
            provider: "github".to_string(),
            metadata: Some(metadata),
        })
    }

    async fn revoke_token(&self, token: &str) -> Result<(), AuthError> {
        let client = reqwest::Client::new();
        let response = client
            .delete(format!(
                "https://api.github.com/applications/{}/token",
                self.client_id
            ))
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .json(&serde_json::json!({ "access_token": token }))
            .send()
            .await
            .map_err(|e| AuthError::NetworkError(e.to_string()))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(AuthError::OAuthError(format!(
                "Failed to revoke token: {}",
                response.status()
            )))
        }
    }
}

impl GitHubProvider {
    /// Get primary email from GitHub API
    async fn get_primary_email(&self, token: &str) -> Result<String, AuthError> {
        #[derive(Deserialize)]
        struct GitHubEmail {
            email: String,
            primary: bool,
            verified: bool,
        }

        let client = reqwest::Client::new();
        let response = client
            .get("https://api.github.com/user/emails")
            .header("Authorization", format!("Bearer {}", token))
            .header("User-Agent", "Urpo-Trace-Explorer")
            .send()
            .await
            .map_err(|e| AuthError::NetworkError(e.to_string()))?;

        let emails: Vec<GitHubEmail> = response
            .json()
            .await
            .map_err(|e| AuthError::NetworkError(e.to_string()))?;

        emails
            .into_iter()
            .find(|e| e.primary && e.verified)
            .map(|e| e.email)
            .ok_or_else(|| AuthError::OAuthError("No primary email found".to_string()))
    }
}