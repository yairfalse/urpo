//! GitHub Device Flow Authentication - Zero Config Required!
//! Just like VS Code - user enters a code, no OAuth app setup needed

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::Duration;

/// GitHub's OAuth app for device flow
/// This is GitHub's official device flow client ID - no secret needed!
const GITHUB_DEVICE_CLIENT_ID: &str = "01ab8ac9400c4e429b23"; // GitHub CLI's public client ID
// For production, register your own at: https://github.com/settings/apps

/// Device flow response from GitHub
#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u32,
    interval: u32,
}

/// Token response from GitHub
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

/// GitHub user information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubUser {
    pub username: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
}

/// Device auth state
pub struct DeviceAuthState {
    current_user: Arc<RwLock<Option<GitHubUser>>>,
    access_token: Arc<RwLock<Option<String>>>,
}

impl DeviceAuthState {
    pub fn new() -> Self {
        Self {
            current_user: Arc::new(RwLock::new(None)),
            access_token: Arc::new(RwLock::new(None)),
        }
    }
}

// ============================================================================
// TAURI COMMANDS - Super Simple Device Flow!
// ============================================================================

/// Start device flow login - returns the code for user to enter
#[tauri::command]
pub async fn start_device_login() -> Result<DeviceFlowInfo, String> {
    println!("🔑 Starting GitHub Device Flow authentication...");
    let client = reqwest::Client::new();

    // Request device code from GitHub
    let response = client
        .post("https://github.com/login/device/code")
        .header("Accept", "application/json")
        .form(&[
            ("client_id", GITHUB_DEVICE_CLIENT_ID),
            ("scope", "read:user user:email"),
        ])
        .send()
        .await
        .map_err(|e| format!("Failed to start device flow: {}", e))?;

    // Check response status
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_else(|_| "No response body".to_string());
        println!("❌ GitHub API error: Status={}, Body={}", status, text);
        return Err(format!("GitHub API error: {} - {}", status, text));
    }

    let response_text = response.text().await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    println!("📥 GitHub response: {}", response_text);

    let device_response: DeviceCodeResponse = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse device response: {} - Response was: {}", e, response_text))?;

    println!("✅ Device code: {}", device_response.user_code);
    println!("🔗 Verification URL: {}", device_response.verification_uri);

    Ok(DeviceFlowInfo {
        user_code: device_response.user_code.clone(),
        verification_url: device_response.verification_uri.clone(),
        device_code: device_response.device_code,
        expires_in: device_response.expires_in,
        interval: device_response.interval,
    })
}

/// Info needed for device flow
#[derive(Debug, Clone, Serialize)]
pub struct DeviceFlowInfo {
    pub user_code: String,        // Code user enters (e.g., "ABCD-1234")
    pub verification_url: String, // Where user goes (github.com/login/device)
    pub device_code: String,      // Internal code for polling
    pub expires_in: u32,          // How long the code is valid
    pub interval: u32,            // How often to poll (seconds)
}

/// Poll for device flow completion
#[tauri::command]
pub async fn poll_device_login(
    device_code: String,
    interval: u32,
    window: tauri::Window,
    state: tauri::State<'_, DeviceAuthState>,
) -> Result<GitHubUser, String> {
    let client = reqwest::Client::new();
    let poll_interval = Duration::from_secs(interval as u64);

    // Poll for up to 15 minutes
    let max_attempts = 900 / interval;

    for attempt in 0..max_attempts {
        // Wait before polling (except first attempt)
        if attempt > 0 {
            tokio::time::sleep(poll_interval).await;
        }

        // Check if user has authorized
        let response = client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .form(&[
                ("client_id", GITHUB_DEVICE_CLIENT_ID),
                ("device_code", &device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await
            .map_err(|e| format!("Failed to poll: {}", e))?;

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse token response: {}", e))?;

        // Check for errors
        if let Some(error) = token_response.error {
            match error.as_str() {
                "authorization_pending" => {
                    // User hasn't authorized yet, keep polling
                    window.emit("auth_status", "waiting").ok();
                    continue;
                }
                "slow_down" => {
                    // We're polling too fast, slow down
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
                "expired_token" => {
                    return Err("The device code has expired. Please try again.".to_string());
                }
                "access_denied" => {
                    return Err("Access was denied.".to_string());
                }
                _ => {
                    return Err(format!("Authentication failed: {}", error));
                }
            }
        }

        // Success! We have a token
        if let Some(token) = token_response.access_token {
            // Get user info
            let user = get_github_user(&token).await
                .map_err(|e| format!("Failed to get user info: {}", e))?;

            // Store token and user
            {
                let mut token_lock = state.access_token.write().await;
                *token_lock = Some(token.clone());
            }
            {
                let mut user_lock = state.current_user.write().await;
                *user_lock = Some(user.clone());
            }

            // Store in keychain for persistence
            store_token_securely(&token).await?;

            window.emit("auth_status", "success").ok();
            return Ok(user);
        }
    }

    Err("Authentication timed out. Please try again.".to_string())
}

/// Open GitHub device login page in browser
#[tauri::command]
pub async fn open_device_login_page() -> Result<(), String> {
    webbrowser::open("https://github.com/login/device")
        .map_err(|e| format!("Failed to open browser: {}", e))?;
    Ok(())
}

/// Get current user
#[tauri::command]
pub async fn get_device_user(state: tauri::State<'_, DeviceAuthState>) -> Result<Option<GitHubUser>, String> {
    // Try to load from keychain first
    if let Ok(Some(token)) = load_token_from_keychain().await {
        // Verify token is still valid
        if let Ok(user) = get_github_user(&token).await {
            // Update state
            let mut token_lock = state.access_token.write().await;
            *token_lock = Some(token);

            let mut user_lock = state.current_user.write().await;
            *user_lock = Some(user.clone());

            return Ok(Some(user));
        }
    }

    // Otherwise check memory
    let user = state.current_user.read().await;
    Ok(user.clone())
}

/// Logout
#[tauri::command]
pub async fn device_logout(state: tauri::State<'_, DeviceAuthState>) -> Result<(), String> {
    // Clear state
    {
        let mut user_lock = state.current_user.write().await;
        *user_lock = None;
    }
    {
        let mut token_lock = state.access_token.write().await;
        *token_lock = None;
    }

    // Clear from keychain
    clear_token_from_keychain().await?;

    Ok(())
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Get GitHub user info
async fn get_github_user(access_token: &str) -> Result<GitHubUser, Box<dyn std::error::Error + Send + Sync>> {
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

/// Store token securely in system keychain
async fn store_token_securely(token: &str) -> Result<(), String> {
    let entry = keyring::Entry::new("urpo", "github_token")
        .map_err(|e| format!("Keychain error: {}", e))?;

    entry.set_password(token)
        .map_err(|e| format!("Failed to store token: {}", e))?;

    Ok(())
}

/// Load token from keychain
async fn load_token_from_keychain() -> Result<Option<String>, String> {
    let entry = keyring::Entry::new("urpo", "github_token")
        .map_err(|e| format!("Keychain error: {}", e))?;

    match entry.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Failed to load token: {}", e))
    }
}

/// Clear token from keychain
async fn clear_token_from_keychain() -> Result<(), String> {
    let entry = keyring::Entry::new("urpo", "github_token")
        .map_err(|e| format!("Keychain error: {}", e))?;

    match entry.delete_password() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()), // Already cleared
        Err(e) => Err(format!("Failed to clear token: {}", e))
    }
}