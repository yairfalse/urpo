# GitHub SSO Setup for Urpo

## 🔐 Secure OAuth Implementation

We've implemented a **production-ready**, **extensible** OAuth system for Urpo with the following features:

### Architecture Highlights

1. **Trait-based Provider System** - Easy to add Google, Microsoft, Okta later
2. **Secure Token Storage** - Uses OS keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service)
3. **CSRF Protection** - Random state parameter validation
4. **Local Callback Server** - Runs on port 8788 for OAuth callbacks
5. **Zero Frontend Secrets** - All secrets stay in Tauri backend

### File Structure

```
src-tauri/src/auth/
├── mod.rs              # Main auth module
├── oauth.rs            # OAuth provider trait
├── storage.rs          # Secure token storage
├── commands.rs         # Tauri IPC commands
└── providers/
    ├── mod.rs          # Provider modules
    └── github.rs       # GitHub OAuth implementation
```

## 🚀 Setup Instructions

### 1. Create GitHub OAuth App

1. Go to: https://github.com/settings/developers
2. Click "New OAuth App"
3. Fill in:
   - **Application name**: Urpo Trace Explorer
   - **Homepage URL**: https://github.com/yairfalse/urpo
   - **Authorization callback URL**: `http://localhost:8788/callback`
4. Click "Register application"
5. Copy your **Client ID**
6. Generate a new **Client Secret** and copy it

### 2. Set Environment Variables

```bash
# In your shell or .env file:
export GITHUB_CLIENT_ID="your_client_id_here"
export GITHUB_CLIENT_SECRET="your_client_secret_here"
```

### 3. Build and Run

```bash
# Build Tauri backend
cd src-tauri
cargo build --release

# Run the app
cargo tauri dev
```

## 🔥 Security Features

### Token Storage
- **Never in localStorage** - Uses OS keychain
- **Encrypted at rest** - OS handles encryption
- **Per-user isolation** - Tokens stored per username

### OAuth Flow
```
User → Click "Login with GitHub"
     → Tauri opens browser
     → GitHub authorization page
     → User approves
     → Redirect to localhost:8788/callback
     → Tauri exchanges code for token
     → Token stored in keychain
     → User info returned to frontend
```

### CSRF Protection
- Random UUID state parameter
- Validated on callback
- Prevents replay attacks

## 🎯 Frontend Usage

```typescript
import { invoke } from '@tauri-apps/api/tauri';

// Login with GitHub
async function loginWithGitHub() {
  try {
    const user = await invoke('login_with_github');
    console.log('Logged in as:', user.username);
  } catch (error) {
    console.error('Login failed:', error);
  }
}

// Get current user
async function getCurrentUser() {
  const user = await invoke('get_current_user');
  return user; // null if not logged in
}

// Logout
async function logout() {
  await invoke('logout');
}

// Check authentication
async function isAuthenticated() {
  return await invoke('is_authenticated');
}
```

## 🔮 Future Enhancements

### Easy to Add:
- **Google OAuth** - Just implement `GoogleProvider`
- **Microsoft/Azure AD** - Add `MicrosoftProvider`
- **Okta/Auth0** - Enterprise SSO providers
- **SAML** - For enterprise customers

### Token Refresh
```rust
// Already prepared for in TokenResponse
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
}
```

### Organization Access
```rust
// Add to scopes for org access
fn scopes(&self) -> Vec<&str> {
    vec!["read:user", "user:email", "read:org"]
}
```

## 🏆 Production Ready Features

✅ **Secure by default** - No secrets in frontend
✅ **Graceful error handling** - User-friendly error pages
✅ **Timeout protection** - 5-minute auth timeout
✅ **Token revocation** - Properly revokes on logout
✅ **Cross-platform** - Works on macOS, Windows, Linux
✅ **Extensible design** - Easy to add providers
✅ **Performance** - Async/await throughout
✅ **Type-safe** - Full TypeScript + Rust types

## 🐛 Troubleshooting

### Port 8788 Already in Use
```bash
# Find process using port
lsof -i :8788

# Kill process
kill -9 <PID>
```

### Keychain Access Denied (macOS)
1. Open Keychain Access
2. Find "urpo" entry
3. Set to "Always Allow"

### Token Storage Failed
- Windows: Check Credential Manager
- Linux: Install gnome-keyring or kwallet
- macOS: Check Keychain permissions

## 📝 Environment Variables

For production, set these in `.env.local`:

```env
# Required
GITHUB_CLIENT_ID=your_client_id
GITHUB_CLIENT_SECRET=your_client_secret

# Optional (defaults shown)
OAUTH_CALLBACK_PORT=8788
OAUTH_TIMEOUT_SECS=300
```

## 🎨 Customization

### Change Callback Port
In `github.rs`:
```rust
redirect_uri: "http://localhost:9999/callback".to_string(),
```

### Add Custom Scopes
```rust
fn scopes(&self) -> Vec<&str> {
    vec!["read:user", "user:email", "repo", "gist"]
}
```

### Custom Success Page
Edit `SUCCESS_HTML` in `commands.rs` for branded experience.

---

**Built with security and extensibility in mind.** Ready for production use! 🚀