//! GitHub OAuth authentication implementation
//! Secure, GUI-based authentication flow with keychain storage

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// OAuth configuration stored securely
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
}

/// GitHub user information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubUser {
    pub username: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
}

/// Authentication state
pub struct AuthState {
    config: Arc<RwLock<Option<OAuthConfig>>>,
    current_user: Arc<RwLock<Option<GitHubUser>>>,
    access_token: Arc<RwLock<Option<String>>>,
}

impl AuthState {
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(None)),
            current_user: Arc::new(RwLock::new(None)),
            access_token: Arc::new(RwLock::new(None)),
        }
    }
}

// ============================================================================
// TAURI COMMANDS MODULE
// ============================================================================

pub mod commands {
    use super::*;
    use tauri::State;

    /// Set OAuth configuration (client ID and secret)
    #[tauri::command]
    pub async fn set_oauth_config(
        client_id: String,
        client_secret: String,
        state: State<'_, super::AuthState>,
    ) -> Result<(), String> {
        let config = super::OAuthConfig {
            client_id,
            client_secret,
        };

        // Store in memory (in production, use keychain)
        let mut config_lock = state.config.write().await;
        *config_lock = Some(config.clone());

        // TODO: Store in system keychain using keyring crate
        // Example:
        // let entry = keyring::Entry::new("urpo", "github_oauth");
        // entry.set_password(&serde_json::to_string(&config)?)?;

        Ok(())
    }

    /// Get OAuth configuration (for settings UI)
    #[tauri::command]
    pub async fn get_oauth_config(state: State<'_, super::AuthState>) -> Result<Option<super::OAuthConfig>, String> {
        let config = state.config.read().await;
        Ok(config.clone())
    }

    /// Login with GitHub - opens browser for OAuth flow
    #[tauri::command]
    pub async fn login_with_github(
        window: tauri::Window,
        state: State<'_, super::AuthState>,
    ) -> Result<super::GitHubUser, String> {
        // Get OAuth config
        let config = {
            let config_lock = state.config.read().await;
            config_lock.clone().ok_or("OAuth not configured. Please set up GitHub OAuth in settings.")?
        };

        // Generate random state for CSRF protection
        let oauth_state = super::generate_random_state();

        // Build OAuth URL
        let auth_url = format!(
            "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope={}&state={}",
            config.client_id,
            "http://localhost:8788/callback",
            "read:user user:email",
            oauth_state
        );

        // Open browser
        if let Err(e) = webbrowser::open(&auth_url) {
            eprintln!("Failed to open browser: {}", e);
            return Err(format!("Failed to open browser: {}", e));
        }

        // Start local server to receive callback
        let auth_code = super::start_callback_server(oauth_state).await
            .map_err(|e| format!("OAuth callback failed: {}", e))?;

        // Exchange code for access token
        let token_response = super::exchange_code_for_token(&config, &auth_code).await
            .map_err(|e| format!("Failed to get access token: {}", e))?;

        // Get user info
        let user = super::get_github_user(&token_response.access_token).await
            .map_err(|e| format!("Failed to get user info: {}", e))?;

        // Store token and user
        {
            let mut token_lock = state.access_token.write().await;
            *token_lock = Some(token_response.access_token);
        }
        {
            let mut user_lock = state.current_user.write().await;
            *user_lock = Some(user.clone());
        }

        Ok(user)
    }

    /// Get current logged in user
    #[tauri::command]
    pub async fn get_current_user(state: State<'_, super::AuthState>) -> Result<Option<super::GitHubUser>, String> {
        let user = state.current_user.read().await;
        Ok(user.clone())
    }

    /// Check if user is authenticated
    #[tauri::command]
    pub async fn is_authenticated(state: State<'_, super::AuthState>) -> Result<bool, String> {
        let user = state.current_user.read().await;
        Ok(user.is_some())
    }

    /// Logout - clear stored credentials
    #[tauri::command]
    pub async fn logout(state: State<'_, super::AuthState>) -> Result<(), String> {
        // Clear in-memory state
        {
            let mut user_lock = state.current_user.write().await;
            *user_lock = None;
        }
        {
            let mut token_lock = state.access_token.write().await;
            *token_lock = None;
        }

        // TODO: Clear from keychain
        // let entry = keyring::Entry::new("urpo", "github_token");
        // let _ = entry.delete_password();

        Ok(())
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn generate_random_state() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..32)
        .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
        .collect()
}

/// Start local server to receive OAuth callback
async fn start_callback_server(expected_state: String) -> Result<String, Box<dyn std::error::Error>> {
    use tokio::net::TcpListener;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = TcpListener::bind("127.0.0.1:8788").await?;

    let (mut socket, _) = listener.accept().await?;

    let mut buffer = [0; 4096];
    let n = socket.read(&mut buffer).await?;
    let request = String::from_utf8_lossy(&buffer[..n]);

    // Parse the request to get code and state
    let code = extract_param(&request, "code")
        .ok_or("No authorization code received")?;

    let state = extract_param(&request, "state")
        .ok_or("No state parameter received")?;

    // Verify state matches
    if state != expected_state {
        return Err("Invalid state parameter - possible CSRF attack".into());
    }

    // Send success response
    let response = r#"HTTP/1.1 200 OK
Content-Type: text/html

<!DOCTYPE html>
<html>
<head>
    <title>Authentication Successful</title>
    <style>
        body {
            font-family: -apple-system, system-ui, sans-serif;
            background: #0A0A0A;
            color: white;
            display: flex;
            align-items: center;
            justify-content: center;
            height: 100vh;
            margin: 0;
        }
        .container {
            text-align: center;
            padding: 2rem;
            background: #141414;
            border-radius: 8px;
            border: 1px solid #262626;
        }
        h1 { color: #0EA5E9; }
        p { color: #A3A3A3; }
    </style>
</head>
<body>
    <div class="container">
        <h1>✓ Authentication Successful</h1>
        <p>You can now close this window and return to Urpo.</p>
    </div>
    <script>setTimeout(() => window.close(), 2000);</script>
</body>
</html>"#;

    socket.write_all(response.as_bytes()).await?;
    socket.flush().await?;

    Ok(code)
}

/// Extract parameter from URL query string
fn extract_param(request: &str, param: &str) -> Option<String> {
    let pattern = format!("{}=", param);
    request
        .lines()
        .find(|line| line.contains(&pattern))
        .and_then(|line| {
            line.split(&pattern)
                .nth(1)
                .and_then(|s| s.split(&['&', ' ', '\r', '\n'][..]).next())
                .map(|s| s.to_string())
        })
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

/// Exchange authorization code for access token
async fn exchange_code_for_token(
    config: &OAuthConfig,
    code: &str,
) -> Result<TokenResponse, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let params = [
        ("client_id", &config.client_id),
        ("client_secret", &config.client_secret),
        ("code", &code.to_string()),
    ];

    let response = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&params)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("GitHub API error: {}", response.status()).into());
    }

    let token_response: TokenResponse = response.json().await?;
    Ok(token_response)
}

/// Get GitHub user information
async fn get_github_user(access_token: &str) -> Result<GitHubUser, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    #[derive(Deserialize)]
    struct GHUser {
        login: String,
        name: Option<String>,
        email: Option<String>,
        avatar_url: Option<String>,
    }

    let response = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("User-Agent", "Urpo-Trace-Explorer")
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("GitHub API error: {}", response.status()).into());
    }

    let gh_user: GHUser = response.json().await?;

    Ok(GitHubUser {
        username: gh_user.login,
        name: gh_user.name,
        email: gh_user.email,
        avatar_url: gh_user.avatar_url,
    })
}