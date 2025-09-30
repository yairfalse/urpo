# Simple GitHub OAuth for Urpo - Like GitKraken/Zed

## Two Approaches for Zero-Config OAuth

### Option 1: Centralized OAuth App (Like GitKraken)
**How it works:**
1. We register ONE GitHub OAuth app for all Urpo users
2. Users just click "Login with GitHub"
3. No setup, no configuration, just works!

**Pros:**
- Zero user configuration
- Professional experience
- We control the app

**Implementation:**
```rust
// In production, these would come from our server
const URPO_CLIENT_ID: &str = "Ov23liYourAppId";
const URPO_CLIENT_SECRET: &str = "encrypted_on_server";
```

The flow:
1. User clicks login → Opens `auth.urpo.dev/github`
2. Our server redirects to GitHub with our app's credentials
3. User approves → GitHub sends token to our server
4. Our server redirects back to `localhost:8788` with token
5. Done!

### Option 2: GitHub Device Flow (Like VS Code)
**How it works:**
1. App shows a code: `XXXX-XXXX`
2. User goes to `github.com/login/device`
3. Enters code, approves
4. App automatically gets token

**Pros:**
- Works everywhere (even SSH/headless)
- No callback URLs needed
- Super secure

**Implementation:**
```rust
#[tauri::command]
pub async fn device_flow_login() -> Result<GitHubUser, String> {
    // 1. Request device code
    let device_code = request_device_code().await?;

    // 2. Show code to user
    show_code_to_user(&device_code.user_code);

    // 3. Open browser to github.com/login/device
    webbrowser::open("https://github.com/login/device")?;

    // 4. Poll for completion
    let token = poll_for_token(&device_code).await?;

    // 5. Get user info
    get_github_user(&token).await
}
```

### Option 3: Hybrid Approach (Best of Both)
1. Try centralized OAuth first (seamless)
2. Fall back to device flow if needed
3. Always works, maximum compatibility

## The User Experience

### What Users See:

1. **First Launch:**
   ```
   ┌─────────────────────────────┐
   │         Welcome to Urpo      │
   │                             │
   │  [Login with GitHub]  🚀    │
   │                             │
   │  One click, no setup needed │
   └─────────────────────────────┘
   ```

2. **Click Login → Browser Opens:**
   ```
   GitHub Authorization Page:

   Urpo Trace Explorer wants to access your account

   Permissions:
   ✓ Read user profile
   ✓ Read user email

   [Authorize Urpo]  [Cancel]
   ```

3. **Success → Auto Return:**
   ```
   ✨ Logged in as @username
   Welcome to Urpo!
   ```

## Implementation Plan

### Step 1: Register Urpo OAuth App
Go to github.com/settings/apps and create:
- **Name:** Urpo Trace Explorer
- **Callback:** https://auth.urpo.dev/callback
- **Permissions:** user:email, read:user

### Step 2: Auth Server (Simple Cloudflare Worker)
```javascript
export default {
  async fetch(request) {
    const url = new URL(request.url);

    if (url.pathname === '/github') {
      // Redirect to GitHub OAuth
      return Response.redirect(
        `https://github.com/login/oauth/authorize?client_id=${CLIENT_ID}&redirect_uri=${CALLBACK_URL}`
      );
    }

    if (url.pathname === '/callback') {
      // Exchange code for token
      const code = url.searchParams.get('code');
      const token = await exchangeCodeForToken(code);

      // Redirect back to app with token
      return Response.redirect(
        `http://localhost:8788/callback?token=${token}`
      );
    }
  }
};
```

### Step 3: Update Frontend
```typescript
// Super simple - no configuration UI needed!
const handleGitHubLogin = async () => {
  try {
    const user = await invoke('simple_github_login');
    setUser(user);
  } catch (error) {
    // Fallback to device flow
    const user = await invoke('device_flow_login');
    setUser(user);
  }
};
```

## Why This is Better

### Current (Manual) Approach:
1. Create GitHub OAuth app ❌
2. Copy Client ID ❌
3. Generate Client Secret ❌
4. Paste into Urpo ❌
5. Finally can login ❌

### New Simple Approach:
1. Click "Login with GitHub" ✅
2. Approve in browser ✅
3. Done! ✅

## Security Considerations

1. **Token Storage:** Use OS keychain (already implemented)
2. **Token Refresh:** Auto-refresh expired tokens
3. **Revocation:** Provide logout that revokes token
4. **Encryption:** All tokens encrypted at rest

## Examples from Popular Apps

### GitKraken:
- Click login → Browser opens → Approve → Done
- No configuration needed

### Zed Editor:
- Click GitHub → Browser opens → Approve → Returns to app
- Seamless experience

### VS Code:
- Shows device code → Enter on GitHub → Activated
- Works everywhere

### Postman:
- OAuth through their servers
- Zero configuration

## Next Steps

1. **Quick Win:** Implement device flow (no server needed)
2. **Professional:** Set up auth.urpo.dev server
3. **Future:** Add more providers (GitLab, Bitbucket)

The key insight: **Users should NEVER have to create OAuth apps themselves!**