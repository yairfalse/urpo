/**
 * GitHub SSO Login Component
 */

import React, { useState } from 'react';
import { Button, COLORS } from '../design-system/core';
import { Github } from 'lucide-react';

interface GitHubLoginProps {
  onSuccess: (user: any) => void;
  onError: (error: string) => void;
}

export const GitHubLogin: React.FC<GitHubLoginProps> = ({ onSuccess, onError }) => {
  const [isLoading, setIsLoading] = useState(false);

  const handleGitHubLogin = async () => {
    setIsLoading(true);

    try {
      // For demo purposes - in production you'd use the real GitHub OAuth flow
      // This would require:
      // 1. Register OAuth App at: https://github.com/settings/applications/new
      // 2. Set Authorization callback URL to: http://localhost:5173/auth/callback
      // 3. Get Client ID and Client Secret
      // 4. Implement proper OAuth flow or use Device Flow

      // Demo implementation - simulates GitHub login
      setTimeout(() => {
        const mockUser = {
          login: 'developer',
          name: 'Urpo Developer',
          email: 'dev@urpo.local',
          avatar_url: 'https://github.com/ghost.png'
        };

        onSuccess(mockUser);
        setIsLoading(false);
      }, 1000);

      // Real implementation would look like:
      /*
      const { user } = await loginWithGitHub();
      onSuccess(user);
      */

    } catch (error) {
      onError(error instanceof Error ? error.message : 'GitHub login failed');
      setIsLoading(false);
    }
  };

  return (
    <div>
      <Button
        variant="secondary"
        onClick={handleGitHubLogin}
        disabled={isLoading}
        style={{
          width: '100%',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          gap: '8px',
          padding: '10px',
          background: COLORS.bg.primary,
          border: `1px solid ${COLORS.border.default}`,
          marginBottom: '12px'
        }}
      >
        <Github size={16} />
        <span>{isLoading ? 'Connecting...' : 'Login with GitHub'}</span>
      </Button>

      {/* Setup instructions */}
      <details style={{ marginTop: '12px' }}>
        <summary style={{
          fontSize: '11px',
          color: COLORS.text.tertiary,
          cursor: 'pointer',
          userSelect: 'none'
        }}>
          How to enable real GitHub SSO
        </summary>
        <div style={{
          marginTop: '8px',
          padding: '8px',
          background: COLORS.bg.primary,
          borderRadius: '4px',
          fontSize: '11px',
          color: COLORS.text.secondary,
          lineHeight: '1.5'
        }}>
          <ol style={{ margin: 0, paddingLeft: '20px' }}>
            <li>Go to GitHub Settings → Developer settings → OAuth Apps</li>
            <li>Click "New OAuth App"</li>
            <li>Set Authorization callback URL: <code>http://localhost:5173/auth/callback</code></li>
            <li>Get your Client ID</li>
            <li>Update <code>GITHUB_CLIENT_ID</code> in github-auth.ts</li>
            <li>Implement OAuth callback handler</li>
          </ol>
        </div>
      </details>
    </div>
  );
};