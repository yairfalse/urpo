/**
 * OAuthSettings.tsx - Configure GitHub OAuth credentials
 * Part of first-run setup or settings page
 */

import React, { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Button, Input, Card, COLORS, SPACING, TYPOGRAPHY, RADIUS } from '../design-system/core';
import { Github, Key, Globe, Check, X, Info, ExternalLink, Copy } from 'lucide-react';
import { invoke } from '@tauri-apps/api/tauri';

interface OAuthSettingsProps {
  onConfigured?: () => void;
  isModal?: boolean;
  onClose?: () => void;
}

export const OAuthSettings: React.FC<OAuthSettingsProps> = ({ onConfigured, isModal = false, onClose }) => {
  const [clientId, setClientId] = useState('');
  const [clientSecret, setClientSecret] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState('');
  const [success, setSuccess] = useState(false);
  const [showInstructions, setShowInstructions] = useState(true);
  const [copiedCallback, setCopiedCallback] = useState(false);

  const CALLBACK_URL = 'http://localhost:8788/callback';

  // Load existing credentials if any
  useEffect(() => {
    const loadCredentials = async () => {
      try {
        const creds = await invoke<{ client_id?: string; client_secret?: string }>('get_oauth_config');
        if (creds?.client_id) setClientId(creds.client_id);
        if (creds?.client_secret) setClientSecret(creds.client_secret);
      } catch (err) {
        // No existing credentials
      }
    };
    loadCredentials();
  }, []);

  const handleSave = async () => {
    if (!clientId || !clientSecret) {
      setError('Please provide both Client ID and Client Secret');
      return;
    }

    setIsLoading(true);
    setError('');

    try {
      await invoke('set_oauth_config', {
        clientId,
        clientSecret
      });

      setSuccess(true);
      setTimeout(() => {
        if (onConfigured) onConfigured();
        if (onClose) onClose();
      }, 1500);
    } catch (err) {
      setError(typeof err === 'string' ? err : 'Failed to save OAuth configuration');
    } finally {
      setIsLoading(false);
    }
  };

  const copyCallbackUrl = () => {
    navigator.clipboard.writeText(CALLBACK_URL);
    setCopiedCallback(true);
    setTimeout(() => setCopiedCallback(false), 2000);
  };

  const content = (
    <div style={{
      padding: isModal ? 0 : SPACING['2xl'],
      maxWidth: isModal ? 'none' : '800px',
      margin: isModal ? 0 : '0 auto'
    }}>
      <div style={{ marginBottom: SPACING.xl }}>
        <h2 style={{
          fontSize: TYPOGRAPHY.size['2xl'],
          fontWeight: TYPOGRAPHY.weight.semibold,
          color: COLORS.text.primary,
          marginBottom: SPACING.sm
        }}>
          Configure GitHub OAuth
        </h2>
        <p style={{
          fontSize: TYPOGRAPHY.size.base,
          color: COLORS.text.secondary
        }}>
          Set up GitHub authentication to access Urpo securely
        </p>
      </div>

      <AnimatePresence>
        {showInstructions && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
          >
            <Card style={{
              padding: SPACING.lg,
              background: COLORS.bg.secondary,
              border: `1px solid ${COLORS.border.subtle}`,
              marginBottom: SPACING.xl
            }}>
              <div style={{
                display: 'flex',
                alignItems: 'flex-start',
                gap: SPACING.md,
                marginBottom: SPACING.lg
              }}>
                <Info size={18} color={COLORS.accent.info} style={{ marginTop: '2px' }} />
                <div style={{ flex: 1 }}>
                  <h3 style={{
                    fontSize: TYPOGRAPHY.size.base,
                    fontWeight: TYPOGRAPHY.weight.semibold,
                    color: COLORS.text.primary,
                    marginBottom: SPACING.md
                  }}>
                    Quick Setup Instructions
                  </h3>

                  <ol style={{
                    margin: 0,
                    paddingLeft: SPACING.lg,
                    color: COLORS.text.secondary,
                    fontSize: TYPOGRAPHY.size.sm,
                    lineHeight: 1.6
                  }}>
                    <li style={{ marginBottom: SPACING.sm }}>
                      Go to GitHub Settings → Developer settings → OAuth Apps →
                      <a
                        href="https://github.com/settings/developers"
                        target="_blank"
                        style={{
                          color: COLORS.accent.primary,
                          marginLeft: '4px',
                          textDecoration: 'none',
                          display: 'inline-flex',
                          alignItems: 'center',
                          gap: '2px'
                        }}
                      >
                        New OAuth App <ExternalLink size={10} />
                      </a>
                    </li>
                    <li style={{ marginBottom: SPACING.sm }}>
                      Fill in the application details:
                      <ul style={{ listStyle: 'none', padding: 0, marginTop: SPACING.xs }}>
                        <li style={{
                          padding: `${SPACING.xs} ${SPACING.sm}`,
                          background: COLORS.bg.primary,
                          borderRadius: RADIUS.sm,
                          marginTop: SPACING.xs,
                          fontFamily: TYPOGRAPHY.font.mono,
                          fontSize: '11px'
                        }}>
                          <strong>Application name:</strong> Urpo Trace Explorer
                        </li>
                        <li style={{
                          padding: `${SPACING.xs} ${SPACING.sm}`,
                          background: COLORS.bg.primary,
                          borderRadius: RADIUS.sm,
                          marginTop: SPACING.xs,
                          fontFamily: TYPOGRAPHY.font.mono,
                          fontSize: '11px'
                        }}>
                          <strong>Homepage URL:</strong> http://localhost:1420
                        </li>
                        <li style={{
                          padding: `${SPACING.xs} ${SPACING.sm}`,
                          background: COLORS.bg.primary,
                          borderRadius: RADIUS.sm,
                          marginTop: SPACING.xs,
                          fontFamily: TYPOGRAPHY.font.mono,
                          fontSize: '11px',
                          display: 'flex',
                          alignItems: 'center',
                          justifyContent: 'space-between'
                        }}>
                          <span>
                            <strong>Callback URL:</strong> {CALLBACK_URL}
                          </span>
                          <button
                            onClick={copyCallbackUrl}
                            style={{
                              background: 'transparent',
                              border: 'none',
                              cursor: 'pointer',
                              padding: '2px',
                              color: copiedCallback ? COLORS.accent.success : COLORS.text.tertiary,
                              display: 'flex',
                              alignItems: 'center'
                            }}
                          >
                            {copiedCallback ? <Check size={12} /> : <Copy size={12} />}
                          </button>
                        </li>
                      </ul>
                    </li>
                    <li style={{ marginBottom: SPACING.sm }}>
                      Click "Register application"
                    </li>
                    <li>
                      Copy the Client ID and generate a Client Secret, then paste them below
                    </li>
                  </ol>
                </div>
                <button
                  onClick={() => setShowInstructions(false)}
                  style={{
                    background: 'transparent',
                    border: 'none',
                    cursor: 'pointer',
                    color: COLORS.text.tertiary,
                    padding: '4px'
                  }}
                >
                  <X size={16} />
                </button>
              </div>
            </Card>
          </motion.div>
        )}
      </AnimatePresence>

      <div style={{ display: 'grid', gap: SPACING.lg }}>
        {/* Client ID */}
        <div>
          <label style={{
            fontSize: TYPOGRAPHY.size.sm,
            color: COLORS.text.secondary,
            marginBottom: SPACING.xs,
            display: 'flex',
            alignItems: 'center',
            gap: SPACING.xs
          }}>
            <Key size={12} />
            Client ID
          </label>
          <Input
            value={clientId}
            onChange={setClientId}
            placeholder="Ov23li..."
            style={{
              width: '100%',
              fontFamily: TYPOGRAPHY.font.mono,
              fontSize: TYPOGRAPHY.size.sm
            }}
          />
        </div>

        {/* Client Secret */}
        <div>
          <label style={{
            fontSize: TYPOGRAPHY.size.sm,
            color: COLORS.text.secondary,
            marginBottom: SPACING.xs,
            display: 'flex',
            alignItems: 'center',
            gap: SPACING.xs
          }}>
            <Key size={12} />
            Client Secret
          </label>
          <Input
            value={clientSecret}
            onChange={setClientSecret}
            placeholder="github_pat_..."
            type="password"
            style={{
              width: '100%',
              fontFamily: TYPOGRAPHY.font.mono,
              fontSize: TYPOGRAPHY.size.sm
            }}
          />
        </div>
      </div>

      {/* Error/Success Messages */}
      {error && (
        <motion.div
          initial={{ opacity: 0, y: -10 }}
          animate={{ opacity: 1, y: 0 }}
          style={{
            marginTop: SPACING.lg,
            padding: SPACING.md,
            background: `${COLORS.accent.error}15`,
            border: `1px solid ${COLORS.accent.error}30`,
            borderRadius: RADIUS.sm,
            fontSize: TYPOGRAPHY.size.sm,
            color: COLORS.accent.error
          }}
        >
          {error}
        </motion.div>
      )}

      {success && (
        <motion.div
          initial={{ opacity: 0, y: -10 }}
          animate={{ opacity: 1, y: 0 }}
          style={{
            marginTop: SPACING.lg,
            padding: SPACING.md,
            background: `${COLORS.accent.success}15`,
            border: `1px solid ${COLORS.accent.success}30`,
            borderRadius: RADIUS.sm,
            fontSize: TYPOGRAPHY.size.sm,
            color: COLORS.accent.success,
            display: 'flex',
            alignItems: 'center',
            gap: SPACING.sm
          }}
        >
          <Check size={16} />
          OAuth configuration saved successfully!
        </motion.div>
      )}

      {/* Actions */}
      <div style={{
        display: 'flex',
        gap: SPACING.md,
        marginTop: SPACING.xl,
        justifyContent: 'flex-end'
      }}>
        {!showInstructions && (
          <Button
            variant="ghost"
            onClick={() => setShowInstructions(true)}
            size="sm"
          >
            Show Instructions
          </Button>
        )}
        {isModal && onClose && (
          <Button
            variant="secondary"
            onClick={onClose}
            disabled={isLoading}
          >
            Cancel
          </Button>
        )}
        <Button
          variant="primary"
          onClick={handleSave}
          disabled={isLoading || !clientId || !clientSecret}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: SPACING.sm
          }}
        >
          {isLoading ? (
            <>
              <div style={{
                width: '14px',
                height: '14px',
                border: `2px solid ${COLORS.border.default}`,
                borderTopColor: 'white',
                borderRadius: '50%',
                animation: 'spin 1s linear infinite'
              }} />
              Saving...
            </>
          ) : (
            <>
              <Github size={14} />
              Save Configuration
            </>
          )}
        </Button>
      </div>
    </div>
  );

  if (isModal) {
    return (
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        style={{
          position: 'fixed',
          top: 0,
          left: 0,
          right: 0,
          bottom: 0,
          background: 'rgba(0, 0, 0, 0.6)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          zIndex: 1000
        }}
      >
        <motion.div
          initial={{ opacity: 0, scale: 0.95, y: 20 }}
          animate={{ opacity: 1, scale: 1, y: 0 }}
          exit={{ opacity: 0, scale: 0.95, y: 20 }}
        >
          <Card style={{
            width: '600px',
            maxHeight: '80vh',
            overflow: 'auto',
            padding: SPACING['2xl'],
            background: COLORS.bg.secondary
          }}>
            {content}
          </Card>
        </motion.div>
      </motion.div>
    );
  }

  return content;
};