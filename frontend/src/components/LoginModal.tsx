/**
 * LoginModal.tsx - Clean login modal using core design system
 */

import React, { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Button, Input, Card, COLORS } from '../design-system/core';
import { User, Lock, X, Github } from 'lucide-react';
import { invoke } from '@tauri-apps/api/tauri';

interface LoginModalProps {
  isOpen: boolean;
  onClose: () => void;
  onLogin: (username: string, password: string) => void;
}

export const LoginModal: React.FC<LoginModalProps> = ({ isOpen, onClose, onLogin }) => {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [isGitHubLoading, setIsGitHubLoading] = useState(false);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setError('');

    if (!username || !password) {
      setError('Please enter username and password');
      return;
    }

    // Simple validation - you can enhance this
    if (username === 'admin' && password === 'admin') {
      onLogin(username, password);
      setUsername('');
      setPassword('');
      onClose();
    } else {
      setError('Invalid credentials');
    }
  };

  const handleGitHubLogin = async () => {
    setIsGitHubLoading(true);
    setError('');

    try {
      const user = await invoke<{ username: string; name?: string; email?: string }>('login_with_github');
      // Pass the GitHub username to parent
      onLogin(user.username, 'github_oauth');
      // Clear form fields
      setUsername('');
      setPassword('');
      onClose();
    } catch (error) {
      // Handle error as string or Error object
      const errorMessage = typeof error === 'string' ? error :
                          error instanceof Error ? error.message :
                          'GitHub login failed. Please check your OAuth app configuration.';
      setError(errorMessage);
    } finally {
      setIsGitHubLoading(false);
    }
  };

  return (
    <AnimatePresence>
      {isOpen && (
        <>
          {/* Backdrop */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            onClick={onClose}
            style={{
              position: 'fixed',
              top: 0,
              left: 0,
              right: 0,
              bottom: 0,
              background: 'rgba(0, 0, 0, 0.5)',
              zIndex: 1000,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center'
            }}
          />

          {/* Modal */}
          <motion.div
            initial={{ opacity: 0, scale: 0.95, y: 20 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.95, y: 20 }}
            style={{
              position: 'fixed',
              top: '50%',
              left: '50%',
              transform: 'translate(-50%, -50%)',
              zIndex: 1001,
              width: '360px'
            }}
          >
            <Card style={{ padding: '24px', background: COLORS.bg.elevated }}>
              {/* Header */}
              <div style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                marginBottom: '24px'
              }}>
                <h2 style={{
                  fontSize: '18px',
                  fontWeight: 600,
                  color: COLORS.text.primary,
                  margin: 0
                }}>
                  Login to Urpo
                </h2>
                <button
                  onClick={onClose}
                  style={{
                    background: 'none',
                    border: 'none',
                    cursor: 'pointer',
                    padding: '4px',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    color: COLORS.text.tertiary
                  }}
                >
                  <X size={18} />
                </button>
              </div>

              {/* GitHub Login Button */}
              <Button
                variant="secondary"
                onClick={handleGitHubLogin}
                disabled={isGitHubLoading}
                style={{
                  width: '100%',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  gap: '8px',
                  marginBottom: '16px',
                  background: COLORS.bg.primary,
                  border: `1px solid ${COLORS.border.default}`
                }}
              >
                <Github size={16} />
                <span>{isGitHubLoading ? 'Connecting to GitHub...' : 'Login with GitHub'}</span>
              </Button>

              <div style={{
                display: 'flex',
                alignItems: 'center',
                gap: '12px',
                marginBottom: '16px'
              }}>
                <div style={{ flex: 1, height: '1px', background: COLORS.border.subtle }} />
                <span style={{ fontSize: '11px', color: COLORS.text.tertiary }}>or</span>
                <div style={{ flex: 1, height: '1px', background: COLORS.border.subtle }} />
              </div>

              {/* Form */}
              <form onSubmit={handleSubmit}>
                <div style={{ marginBottom: '16px' }}>
                  <label style={{
                    fontSize: '12px',
                    color: COLORS.text.secondary,
                    marginBottom: '6px',
                    display: 'block'
                  }}>
                    Username
                  </label>
                  <div style={{ position: 'relative' }}>
                    <User
                      size={14}
                      style={{
                        position: 'absolute',
                        left: '12px',
                        top: '50%',
                        transform: 'translateY(-50%)',
                        color: COLORS.text.tertiary
                      }}
                    />
                    <Input
                      value={username}
                      onChange={setUsername}
                      placeholder="Enter username"
                      style={{ paddingLeft: '36px', width: '100%' }}
                    />
                  </div>
                </div>

                <div style={{ marginBottom: '20px' }}>
                  <label style={{
                    fontSize: '12px',
                    color: COLORS.text.secondary,
                    marginBottom: '6px',
                    display: 'block'
                  }}>
                    Password
                  </label>
                  <div style={{ position: 'relative' }}>
                    <Lock
                      size={14}
                      style={{
                        position: 'absolute',
                        left: '12px',
                        top: '50%',
                        transform: 'translateY(-50%)',
                        color: COLORS.text.tertiary
                      }}
                    />
                    <Input
                      value={password}
                      onChange={setPassword}
                      placeholder="Enter password"
                      type="password"
                      style={{ paddingLeft: '36px', width: '100%' }}
                    />
                  </div>
                </div>

                {/* Error message */}
                {error && (
                  <div style={{
                    padding: '8px 12px',
                    background: `${COLORS.accent.error}20`,
                    border: `1px solid ${COLORS.accent.error}40`,
                    borderRadius: '4px',
                    marginBottom: '16px'
                  }}>
                    <span style={{ fontSize: '12px', color: COLORS.accent.error }}>
                      {error}
                    </span>
                  </div>
                )}

                {/* Actions */}
                <div style={{ display: 'flex', gap: '8px' }}>
                  <Button
                    variant="secondary"
                    onClick={onClose}
                    style={{ flex: 1 }}
                  >
                    Cancel
                  </Button>
                  <Button
                    variant="primary"
                    type="submit"
                    style={{ flex: 1 }}
                  >
                    Login
                  </Button>
                </div>

                {/* Help text */}
                <div style={{
                  textAlign: 'center',
                  marginTop: '16px',
                  paddingTop: '16px',
                  borderTop: `1px solid ${COLORS.border.subtle}`
                }}>
                  <span style={{ fontSize: '11px', color: COLORS.text.tertiary }}>
                    Default: admin / admin
                  </span>
                </div>
              </form>
            </Card>
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
};