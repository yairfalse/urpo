/**
 * LoginPage.tsx - Full-page login with GitHub SSO
 * Using Urpo's core design system
 */

import React, { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { Button, Card, COLORS, SPACING, TYPOGRAPHY, RADIUS } from '../design-system/core';
import { Activity, Github, ArrowRight, Shield, Zap, Eye, Settings } from 'lucide-react';
import { invoke } from '@tauri-apps/api/tauri';
import { OAuthSettings } from '../components/OAuthSettings';

interface LoginPageProps {
  onLogin: (username: string) => void;
}

export const LoginPage: React.FC<LoginPageProps> = ({ onLogin }) => {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState('');
  const [showSettings, setShowSettings] = useState(false);
  const [hasOAuthConfig, setHasOAuthConfig] = useState(false);

  // Check if OAuth is configured on mount
  useEffect(() => {
    const checkConfig = async () => {
      try {
        const config = await invoke('get_oauth_config');
        setHasOAuthConfig(!!config);
      } catch (err) {
        console.log('No OAuth config found');
      }
    };
    checkConfig();
  }, []);

  const handleGitHubLogin = async () => {
    setIsLoading(true);
    setError('');

    try {
      const user = await invoke<{ username: string; name?: string; email?: string }>('login_with_github');
      onLogin(user.username);
    } catch (error) {
      const errorMessage = typeof error === 'string' ? error :
                          error instanceof Error ? error.message :
                          'GitHub login failed';
      setError(errorMessage);
    } finally {
      setIsLoading(false);
    }
  };

  const features = [
    { icon: Zap, text: 'Lightning-fast trace analysis' },
    { icon: Shield, text: 'Secure GitHub authentication' },
    { icon: Eye, text: 'Real-time observability insights' },
  ];

  return (
    <div style={{
      minHeight: '100vh',
      background: COLORS.bg.primary,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      position: 'relative',
      overflow: 'hidden'
    }}>
      {/* Background gradient effect */}
      <div style={{
        position: 'absolute',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        background: `radial-gradient(circle at 30% 50%, ${COLORS.accent.primary}15 0%, transparent 50%),
                     radial-gradient(circle at 70% 50%, ${COLORS.accent.info}10 0%, transparent 50%)`,
        pointerEvents: 'none'
      }} />

      {/* Grid pattern overlay */}
      <div style={{
        position: 'absolute',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        backgroundImage: `linear-gradient(${COLORS.border.subtle}40 1px, transparent 1px),
                          linear-gradient(90deg, ${COLORS.border.subtle}40 1px, transparent 1px)`,
        backgroundSize: '50px 50px',
        opacity: 0.3,
        pointerEvents: 'none'
      }} />

      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5 }}
        style={{
          width: '100%',
          maxWidth: '900px',
          padding: SPACING['2xl'],
          zIndex: 1
        }}
      >
        <div style={{
          display: 'grid',
          gridTemplateColumns: '1fr 1fr',
          gap: SPACING['3xl'],
          alignItems: 'center'
        }}>
          {/* Left side - Branding */}
          <div>
            <motion.div
              initial={{ opacity: 0, x: -20 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ delay: 0.2 }}
            >
              {/* Logo */}
              <div style={{ display: 'flex', alignItems: 'center', gap: SPACING.md, marginBottom: SPACING['2xl'] }}>
                <div style={{
                  width: '48px',
                  height: '48px',
                  background: `linear-gradient(135deg, ${COLORS.accent.primary}, ${COLORS.accent.info})`,
                  borderRadius: RADIUS.lg,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  boxShadow: `0 10px 30px ${COLORS.accent.primary}40`
                }}>
                  <Activity size={24} color="white" />
                </div>
                <div>
                  <h1 style={{
                    fontSize: TYPOGRAPHY.size['3xl'],
                    fontWeight: TYPOGRAPHY.weight.bold,
                    color: COLORS.text.primary,
                    margin: 0
                  }}>
                    URPO
                  </h1>
                  <p style={{
                    fontSize: TYPOGRAPHY.size.sm,
                    color: COLORS.text.tertiary,
                    margin: 0
                  }}>
                    OpenTelemetry Trace Explorer
                  </p>
                </div>
              </div>

              {/* Tagline */}
              <h2 style={{
                fontSize: TYPOGRAPHY.size['2xl'],
                fontWeight: TYPOGRAPHY.weight.semibold,
                color: COLORS.text.primary,
                marginBottom: SPACING.lg,
                lineHeight: 1.3
              }}>
                The fastest way to explore
                <span style={{
                  background: `linear-gradient(135deg, ${COLORS.accent.primary}, ${COLORS.accent.info})`,
                  backgroundClip: 'text',
                  WebkitBackgroundClip: 'text',
                  WebkitTextFillColor: 'transparent',
                  marginLeft: '8px'
                }}>
                  distributed traces
                </span>
              </h2>

              {/* Features */}
              <div style={{ marginTop: SPACING['2xl'] }}>
                {features.map((feature, index) => (
                  <motion.div
                    key={index}
                    initial={{ opacity: 0, x: -20 }}
                    animate={{ opacity: 1, x: 0 }}
                    transition={{ delay: 0.3 + index * 0.1 }}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: SPACING.md,
                      marginBottom: SPACING.lg
                    }}
                  >
                    <div style={{
                      width: '32px',
                      height: '32px',
                      background: COLORS.bg.secondary,
                      borderRadius: RADIUS.md,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      border: `1px solid ${COLORS.border.subtle}`
                    }}>
                      <feature.icon size={16} color={COLORS.accent.primary} />
                    </div>
                    <span style={{
                      fontSize: TYPOGRAPHY.size.base,
                      color: COLORS.text.secondary
                    }}>
                      {feature.text}
                    </span>
                  </motion.div>
                ))}
              </div>
            </motion.div>
          </div>

          {/* Right side - Login */}
          <motion.div
            initial={{ opacity: 0, x: 20 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ delay: 0.3 }}
          >
            <Card style={{
              padding: SPACING['2xl'],
              background: COLORS.bg.secondary,
              border: `1px solid ${COLORS.border.subtle}`,
              boxShadow: '0 20px 60px rgba(0,0,0,0.5)'
            }}>
              <h3 style={{
                fontSize: TYPOGRAPHY.size.xl,
                fontWeight: TYPOGRAPHY.weight.semibold,
                color: COLORS.text.primary,
                marginBottom: SPACING.md
              }}>
                Welcome back
              </h3>
              <p style={{
                fontSize: TYPOGRAPHY.size.sm,
                color: COLORS.text.secondary,
                marginBottom: SPACING['2xl']
              }}>
                Sign in to access your traces and metrics
              </p>

              {/* GitHub Login Button */}
              <button
                onClick={hasOAuthConfig ? handleGitHubLogin : () => setShowSettings(true)}
                disabled={isLoading}
                style={{
                  width: '100%',
                  padding: '14px',
                  background: COLORS.bg.primary,
                  border: `1px solid ${COLORS.border.default}`,
                  borderRadius: RADIUS.md,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  gap: SPACING.md,
                  cursor: isLoading ? 'not-allowed' : 'pointer',
                  transition: 'all 0.2s ease',
                  fontSize: TYPOGRAPHY.size.base,
                  fontWeight: TYPOGRAPHY.weight.medium,
                  color: COLORS.text.primary,
                  position: 'relative',
                  overflow: 'hidden'
                }}
                onMouseEnter={(e) => {
                  if (!isLoading) {
                    e.currentTarget.style.background = COLORS.bg.elevated;
                    e.currentTarget.style.borderColor = COLORS.border.strong;
                  }
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = COLORS.bg.primary;
                  e.currentTarget.style.borderColor = COLORS.border.default;
                }}
              >
                {isLoading ? (
                  <>
                    <div style={{
                      width: '16px',
                      height: '16px',
                      border: `2px solid ${COLORS.border.default}`,
                      borderTopColor: COLORS.accent.primary,
                      borderRadius: '50%',
                      animation: 'spin 1s linear infinite'
                    }} />
                    <span>Connecting to GitHub...</span>
                  </>
                ) : (
                  <>
                    <Github size={18} />
                    <span>{hasOAuthConfig ? 'Continue with GitHub' : 'Setup GitHub OAuth'}</span>
                    {hasOAuthConfig ? (
                      <ArrowRight size={14} style={{ marginLeft: 'auto' }} />
                    ) : (
                      <Settings size={14} style={{ marginLeft: 'auto' }} />
                    )}
                  </>
                )}
              </button>

              {/* Error message */}
              {error && (
                <motion.div
                  initial={{ opacity: 0, y: -10 }}
                  animate={{ opacity: 1, y: 0 }}
                  style={{
                    marginTop: SPACING.lg,
                    padding: SPACING.md,
                    background: `${COLORS.accent.error}15`,
                    border: `1px solid ${COLORS.accent.error}30`,
                    borderRadius: RADIUS.sm
                  }}
                >
                  <p style={{
                    fontSize: TYPOGRAPHY.size.sm,
                    color: COLORS.accent.error,
                    margin: 0
                  }}>
                    {error}
                  </p>
                </motion.div>
              )}

              {/* Divider */}
              <div style={{
                display: 'flex',
                alignItems: 'center',
                gap: SPACING.md,
                margin: `${SPACING.xl} 0`
              }}>
                <div style={{ flex: 1, height: '1px', background: COLORS.border.subtle }} />
                <span style={{ fontSize: TYPOGRAPHY.size.xs, color: COLORS.text.tertiary }}>
                  SECURE AUTHENTICATION
                </span>
                <div style={{ flex: 1, height: '1px', background: COLORS.border.subtle }} />
              </div>

              {/* Security note */}
              <div style={{
                padding: SPACING.md,
                background: COLORS.bg.primary,
                borderRadius: RADIUS.sm,
                border: `1px solid ${COLORS.border.subtle}`
              }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: SPACING.sm, marginBottom: SPACING.sm }}>
                  <Shield size={14} color={COLORS.accent.success} />
                  <span style={{
                    fontSize: TYPOGRAPHY.size.xs,
                    fontWeight: TYPOGRAPHY.weight.medium,
                    color: COLORS.text.secondary
                  }}>
                    Enterprise-grade security
                  </span>
                </div>
                <p style={{
                  fontSize: TYPOGRAPHY.size.xs,
                  color: COLORS.text.tertiary,
                  margin: 0,
                  lineHeight: 1.5
                }}>
                  Your credentials are encrypted and stored securely in your system's keychain.
                  We never store passwords or tokens in plain text.
                </p>
              </div>
            </Card>
          </motion.div>
        </div>

        {/* Footer */}
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 0.6 }}
          style={{
            textAlign: 'center',
            marginTop: SPACING['3xl'],
            fontSize: TYPOGRAPHY.size.xs,
            color: COLORS.text.tertiary
          }}
        >
          Built with Rust + Tauri for maximum performance
        </motion.div>
      </motion.div>

      {/* Settings button */}
      {!hasOAuthConfig && (
        <motion.button
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 0.7 }}
          onClick={() => setShowSettings(true)}
          style={{
            position: 'fixed',
            top: '20px',
            right: '20px',
            padding: '10px',
            background: COLORS.bg.secondary,
            border: `1px solid ${COLORS.border.subtle}`,
            borderRadius: RADIUS.md,
            cursor: 'pointer',
            display: 'flex',
            alignItems: 'center',
            gap: SPACING.sm,
            fontSize: TYPOGRAPHY.size.sm,
            color: COLORS.text.secondary
          }}
        >
          <Settings size={16} />
          Configure GitHub OAuth
        </motion.button>
      )}

      {/* OAuth Settings Modal */}
      {showSettings && (
        <OAuthSettings
          isModal={true}
          onClose={() => setShowSettings(false)}
          onConfigured={() => {
            setShowSettings(false);
            setHasOAuthConfig(true);
          }}
        />
      )}

      {/* Add spinning animation */}
      <style>{`
        @keyframes spin {
          to { transform: rotate(360deg); }
        }
      `}</style>
    </div>
  );
};