//! Tauri commands for authentication
//!
//! These commands are exposed to the frontend via Tauri's IPC bridge

use crate::auth::{AuthError, AuthState, UserInfo};
use tauri::{Manager, State};
use tokio::sync::{oneshot, Mutex};
use warp::Filter;
use std::sync::Arc;

/// Login with GitHub OAuth
#[tauri::command]
pub async fn login_with_github(
    state: State<'_, Arc<AuthState>>,
    app_handle: tauri::AppHandle,
) -> Result<UserInfo, String> {
    // Generate random state for CSRF protection
    let csrf_state = uuid::Uuid::new_v4().to_string();

    // Get auth URL from provider
    let provider = state.provider.lock().await;
    let auth_url = provider.get_auth_url(&csrf_state);

    // Create channel for receiving OAuth callback
    let (tx, rx) = oneshot::channel::<Result<String, AuthError>>();

    // Start OAuth callback server
    let server_handle = start_callback_server(tx, csrf_state.clone());

    // Store server handle
    {
        let mut handle = state.server_handle.lock().await;
        *handle = Some(server_handle);
    }

    // Open browser for user to authorize
    open::that(&auth_url).map_err(|e| format!("Failed to open browser: {}", e))?;

    // Wait for OAuth callback (with timeout)
    let code = tokio::time::timeout(
        std::time::Duration::from_secs(300), // 5 minute timeout
        rx
    )
    .await
    .map_err(|_| "Authentication timeout".to_string())?
    .map_err(|_| "Authentication cancelled".to_string())??;

    // Exchange code for token
    let token = provider.exchange_code(code).await?;

    // Get user info
    let user = provider.get_user_info(&token).await?;

    // Store token and user securely
    state.storage.store_token(&user.username, &token)?;
    state.storage.store_user(&user)?;

    // Emit event to frontend
    app_handle.emit_all("auth:login", &user)
        .map_err(|e| e.to_string())?;

    Ok(user)
}

/// Logout current user
#[tauri::command]
pub async fn logout(
    state: State<'_, Arc<AuthState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    // Get current user
    let user = state.storage.get_user()
        .map_err(|_| "No user logged in".to_string())?;

    // Get token to revoke
    if let Ok(token) = state.storage.get_token(&user.username) {
        // Try to revoke token with provider (ignore errors)
        let provider = state.provider.lock().await;
        let _ = provider.revoke_token(&token).await;
    }

    // Clear stored data
    state.storage.delete_token(&user.username)?;
    state.storage.clear_user()?;

    // Emit event to frontend
    app_handle.emit_all("auth:logout", ())
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Get current logged in user
#[tauri::command]
pub fn get_current_user(state: State<'_, Arc<AuthState>>) -> Result<Option<UserInfo>, String> {
    match state.storage.get_user() {
        Ok(user) => Ok(Some(user)),
        Err(_) => Ok(None),
    }
}

/// Check if user is authenticated
#[tauri::command]
pub fn is_authenticated(state: State<'_, Arc<AuthState>>) -> bool {
    state.storage.is_authenticated()
}

/// Start OAuth callback server
fn start_callback_server(
    tx: oneshot::Sender<Result<String, AuthError>>,
    expected_state: String,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        // Wrap the sender in Arc<Mutex<Option<_>>> to handle single-use channel
        let sender = Arc::new(Mutex::new(Some(tx)));
        let state = Arc::new(expected_state);

        // Define callback route
        let sender_clone = sender.clone();
        let state_clone = state.clone();

        let callback = warp::path("callback")
            .and(warp::query::<CallbackParams>())
            .and_then(move |params: CallbackParams| {
                let sender = sender_clone.clone();
                let expected_state = state_clone.clone();

                async move {
                    // Verify CSRF state
                    if params.state != *expected_state {
                        if let Some(tx) = sender.lock().await.take() {
                            let _ = tx.send(Err(AuthError::OAuthError("Invalid state".to_string())));
                        }
                        return Ok::<_, warp::Rejection>(warp::reply::html(ERROR_HTML));
                    }

                    // Check for error
                    if let Some(error) = params.error {
                        if let Some(tx) = sender.lock().await.take() {
                            let _ = tx.send(Err(AuthError::OAuthError(error)));
                        }
                        return Ok::<_, warp::Rejection>(warp::reply::html(ERROR_HTML));
                    }

                    // Send code through channel
                    if let Some(code) = params.code {
                        if let Some(tx) = sender.lock().await.take() {
                            let _ = tx.send(Ok(code));
                        }
                        return Ok::<_, warp::Rejection>(warp::reply::html(SUCCESS_HTML));
                    }

                    if let Some(tx) = sender.lock().await.take() {
                        let _ = tx.send(Err(AuthError::OAuthError("No code received".to_string())));
                    }
                    Ok::<_, warp::Rejection>(warp::reply::html(ERROR_HTML))
                }
            });

        // Start server on port 8788
        warp::serve(callback)
            .run(([127, 0, 0, 1], 8788))
            .await;
    })
}

/// OAuth callback parameters
#[derive(Debug, serde::Deserialize)]
struct CallbackParams {
    code: Option<String>,
    state: String,
    error: Option<String>,
    error_description: Option<String>,
}

/// Success HTML page
const SUCCESS_HTML: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <title>Login Successful</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            margin: 0;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        }
        .container {
            text-align: center;
            background: white;
            padding: 40px;
            border-radius: 10px;
            box-shadow: 0 10px 40px rgba(0,0,0,0.1);
        }
        h1 { color: #333; margin-bottom: 10px; }
        p { color: #666; }
        .checkmark {
            width: 60px;
            height: 60px;
            margin: 0 auto 20px;
            background: #10B981;
            border-radius: 50%;
            display: flex;
            align-items: center;
            justify-content: center;
        }
        .checkmark::after {
            content: "✓";
            color: white;
            font-size: 30px;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="checkmark"></div>
        <h1>Login Successful!</h1>
        <p>You can now close this window and return to Urpo.</p>
        <script>
            setTimeout(() => window.close(), 2000);
        </script>
    </div>
</body>
</html>
"#;

/// Error HTML page
const ERROR_HTML: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <title>Login Failed</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            margin: 0;
            background: linear-gradient(135deg, #f5f7fa 0%, #c3cfe2 100%);
        }
        .container {
            text-align: center;
            background: white;
            padding: 40px;
            border-radius: 10px;
            box-shadow: 0 10px 40px rgba(0,0,0,0.1);
        }
        h1 { color: #333; margin-bottom: 10px; }
        p { color: #666; }
        .error {
            width: 60px;
            height: 60px;
            margin: 0 auto 20px;
            background: #EF4444;
            border-radius: 50%;
            display: flex;
            align-items: center;
            justify-content: center;
        }
        .error::after {
            content: "✕";
            color: white;
            font-size: 30px;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="error"></div>
        <h1>Login Failed</h1>
        <p>Please try again or contact support if the issue persists.</p>
        <script>
            setTimeout(() => window.close(), 3000);
        </script>
    </div>
</body>
</html>
"#;