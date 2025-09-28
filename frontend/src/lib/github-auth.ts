/**
 * GitHub OAuth Device Flow Authentication
 * No backend required - works directly with GitHub
 */

const GITHUB_CLIENT_ID = 'YOUR_GITHUB_APP_CLIENT_ID'; // You'll need to register an OAuth app
const GITHUB_DEVICE_AUTH_URL = 'https://github.com/login/device/code';
const GITHUB_TOKEN_URL = 'https://github.com/login/oauth/access_token';

export interface DeviceCodeResponse {
  device_code: string;
  user_code: string;
  verification_uri: string;
  expires_in: number;
  interval: number;
}

export interface AccessTokenResponse {
  access_token?: string;
  error?: string;
  error_description?: string;
}

export interface GitHubUser {
  login: string;
  name: string;
  email: string;
  avatar_url: string;
}

/**
 * Start GitHub device flow authentication
 */
export async function initiateGitHubLogin(): Promise<DeviceCodeResponse> {
  const response = await fetch(GITHUB_DEVICE_AUTH_URL, {
    method: 'POST',
    headers: {
      'Accept': 'application/json',
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({
      client_id: GITHUB_CLIENT_ID,
      scope: 'read:user user:email'
    })
  });

  if (!response.ok) {
    throw new Error('Failed to initiate GitHub login');
  }

  return response.json();
}

/**
 * Poll for access token after user authorizes
 */
export async function pollForAccessToken(deviceCode: string): Promise<string> {
  const pollInterval = 5000; // 5 seconds
  const maxAttempts = 24; // 2 minutes total

  for (let i = 0; i < maxAttempts; i++) {
    const response = await fetch(GITHUB_TOKEN_URL, {
      method: 'POST',
      headers: {
        'Accept': 'application/json',
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        client_id: GITHUB_CLIENT_ID,
        device_code: deviceCode,
        grant_type: 'urn:ietf:params:oauth:grant-type:device_code'
      })
    });

    const data: AccessTokenResponse = await response.json();

    if (data.access_token) {
      return data.access_token;
    }

    if (data.error === 'authorization_pending') {
      // User hasn't authorized yet, keep polling
      await new Promise(resolve => setTimeout(resolve, pollInterval));
      continue;
    }

    if (data.error) {
      throw new Error(data.error_description || data.error);
    }
  }

  throw new Error('Authentication timeout');
}

/**
 * Get GitHub user info with access token
 */
export async function getGitHubUser(accessToken: string): Promise<GitHubUser> {
  const response = await fetch('https://api.github.com/user', {
    headers: {
      'Authorization': `Bearer ${accessToken}`,
      'Accept': 'application/json'
    }
  });

  if (!response.ok) {
    throw new Error('Failed to fetch user info');
  }

  return response.json();
}

/**
 * Complete GitHub login flow
 */
export async function loginWithGitHub(): Promise<{ user: GitHubUser; token: string }> {
  // Step 1: Get device code
  const deviceCode = await initiateGitHubLogin();

  // Step 2: Open GitHub in browser for user to authorize
  window.open(deviceCode.verification_uri, '_blank');

  // Show the user code they need to enter
  alert(`Enter this code on GitHub: ${deviceCode.user_code}`);

  // Step 3: Poll for access token
  const accessToken = await pollForAccessToken(deviceCode.device_code);

  // Step 4: Get user info
  const user = await getGitHubUser(accessToken);

  // Store token securely (in production, use secure storage)
  localStorage.setItem('github_token', accessToken);

  return { user, token: accessToken };
}

/**
 * Check if user is authenticated
 */
export function isAuthenticated(): boolean {
  return !!localStorage.getItem('github_token');
}

/**
 * Logout
 */
export function logout(): void {
  localStorage.removeItem('github_token');
  localStorage.removeItem('github_user');
}