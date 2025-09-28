//! Secure token storage using OS keychain
//!
//! Uses the keyring crate to securely store tokens in:
//! - macOS: Keychain
//! - Windows: Credential Manager
//! - Linux: Secret Service (GNOME Keyring, KWallet)

use keyring::Entry;
use serde::{Deserialize, Serialize};
use crate::auth::{AuthError, UserInfo};

/// Secure token storage
pub struct SecureTokenStorage {
    service_name: String,
}

impl SecureTokenStorage {
    /// Create new secure storage instance
    pub fn new(service_name: &str) -> Self {
        Self {
            service_name: service_name.to_string(),
        }
    }

    /// Store access token securely
    pub fn store_token(&self, username: &str, token: &str) -> Result<(), AuthError> {
        let entry = Entry::new(&self.service_name, &format!("{}_token", username))
            .map_err(|e| AuthError::StorageError(e.to_string()))?;

        entry
            .set_password(token)
            .map_err(|e| AuthError::StorageError(e.to_string()))?;

        Ok(())
    }

    /// Retrieve access token
    pub fn get_token(&self, username: &str) -> Result<String, AuthError> {
        let entry = Entry::new(&self.service_name, &format!("{}_token", username))
            .map_err(|e| AuthError::StorageError(e.to_string()))?;

        entry
            .get_password()
            .map_err(|e| AuthError::StorageError(e.to_string()))
    }

    /// Delete access token
    pub fn delete_token(&self, username: &str) -> Result<(), AuthError> {
        let entry = Entry::new(&self.service_name, &format!("{}_token", username))
            .map_err(|e| AuthError::StorageError(e.to_string()))?;

        entry
            .delete_password()
            .map_err(|e| AuthError::StorageError(e.to_string()))?;

        Ok(())
    }

    /// Store user information
    pub fn store_user(&self, user: &UserInfo) -> Result<(), AuthError> {
        let entry = Entry::new(&self.service_name, "current_user")
            .map_err(|e| AuthError::StorageError(e.to_string()))?;

        let user_json = serde_json::to_string(user)
            .map_err(|e| AuthError::StorageError(e.to_string()))?;

        entry
            .set_password(&user_json)
            .map_err(|e| AuthError::StorageError(e.to_string()))?;

        Ok(())
    }

    /// Get current user information
    pub fn get_user(&self) -> Result<UserInfo, AuthError> {
        let entry = Entry::new(&self.service_name, "current_user")
            .map_err(|e| AuthError::StorageError(e.to_string()))?;

        let user_json = entry
            .get_password()
            .map_err(|e| AuthError::StorageError(e.to_string()))?;

        serde_json::from_str(&user_json)
            .map_err(|e| AuthError::StorageError(e.to_string()))
    }

    /// Clear current user
    pub fn clear_user(&self) -> Result<(), AuthError> {
        let entry = Entry::new(&self.service_name, "current_user")
            .map_err(|e| AuthError::StorageError(e.to_string()))?;

        // Ignore error if entry doesn't exist
        let _ = entry.delete_password();
        Ok(())
    }

    /// Check if user is logged in
    pub fn is_authenticated(&self) -> bool {
        self.get_user().is_ok()
    }

    /// Clear all stored data
    pub fn clear_all(&self) -> Result<(), AuthError> {
        // Clear user info
        self.clear_user()?;

        // Note: We don't clear all tokens as we don't track all usernames
        // In production, you might want to maintain a list of stored usernames

        Ok(())
    }
}

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub user: UserInfo,
    pub token: String,
    pub expires_at: Option<i64>,
    pub created_at: i64,
}

impl Session {
    /// Create new session
    pub fn new(user: UserInfo, token: String) -> Self {
        Self {
            user,
            token,
            expires_at: None,
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Check if session is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            chrono::Utc::now().timestamp() > expires_at
        } else {
            false
        }
    }
}