/**
 * DeviceLogin.tsx - GitHub Device Flow Login (like VS Code!)
 * Zero configuration required - just enter a code
 */

import React, { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { Button, Card, COLORS, SPACING, TYPOGRAPHY, RADIUS } from '../design-system/core';
import { Github, Copy, CheckCircle, ExternalLink, Loader } from 'lucide-react';
import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';

interface DeviceLoginProps {
  onSuccess: (user: any) => void;
  onCancel?: () => void;
}

export const DeviceLogin: React.FC<DeviceLoginProps> = ({ onSuccess, onCancel }) => {
  const [step, setStep] = useState<'start' | 'code' | 'waiting' | 'success'>('start');
  const [deviceInfo, setDeviceInfo] = useState<any>(null);
  const [error, setError] = useState('');
  const [copied, setCopied] = useState(false);
  const [isPolling, setIsPolling] = useState(false);
  const [isStarting, setIsStarting] = useState(false);

  useEffect(() => {
    // Listen for auth status updates
    const unlisten = listen('auth_status', (event) => {
      if (event.payload === 'success') {
        setStep('success');
      }
    });

    return () => {
      unlisten.then(fn => fn());
    };
  }, []);

  // Auto-start the device login flow when component mounts
  useEffect(() => {
    if (!isStarting && step === 'start') {
      setIsStarting(true);
      startDeviceLogin();
    }
  }, []);

  const startDeviceLogin = async () => {
    setError('');
    console.log('Starting device login...');
    try {
      // Get device code from backend
      console.log('Invoking start_device_login...');
      const info = await invoke('start_device_login');
      console.log('Device info received:', info);
      console.log('Setting deviceInfo to:', info);
      console.log('user_code from info:', info?.user_code);
      setDeviceInfo(info);
      console.log('Setting step to code');
      setStep('code');

      // Open GitHub in browser
      await invoke('open_device_login_page');

      // Start polling for completion
      startPolling(info);
    } catch (err) {
      console.error('Error in startDeviceLogin:', err);
      setError(typeof err === 'string' ? err : 'Failed to start login');
      setStep('start');
      setIsStarting(false);
    }
  };

  const startPolling = async (info: any) => {
    setIsPolling(true);
    // Don't immediately change to waiting - let user see the code first
    setTimeout(() => {
      if (step === 'code') {
        setStep('waiting');
      }
    }, 3000); // Give user 3 seconds to see the code

    try {
      const user = await invoke('poll_device_login', {
        deviceCode: info.device_code,
        interval: info.interval
      });

      setStep('success');
      setTimeout(() => {
        onSuccess(user);
      }, 1500);
    } catch (err) {
      setError(typeof err === 'string' ? err : 'Authentication failed');
      setStep('code');
    } finally {
      setIsPolling(false);
    }
  };

  const copyCode = () => {
    if (deviceInfo?.user_code) {
      navigator.clipboard.writeText(deviceInfo.user_code);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  return (
    <Card style={{
      padding: SPACING['2xl'],
      background: COLORS.bg.secondary,
      maxWidth: '500px',
      margin: '0 auto'
    }}>
        {/* Step 1: Loading */}
        {step === 'start' && (
          <div>
            <div style={{ textAlign: 'center' }}>
              <div style={{
                width: '64px',
                height: '64px',
                background: `linear-gradient(135deg, ${COLORS.accent.primary}, ${COLORS.accent.info})`,
                borderRadius: RADIUS.lg,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                margin: '0 auto',
                marginBottom: SPACING.xl
              }}>
                <Github size={32} color="white" />
              </div>

              <h2 style={{
                fontSize: TYPOGRAPHY.size['2xl'],
                fontWeight: TYPOGRAPHY.weight.semibold,
                color: COLORS.text.primary,
                marginBottom: SPACING.md
              }}>
                Connecting to GitHub...
              </h2>

              <p style={{
                fontSize: TYPOGRAPHY.size.base,
                color: COLORS.text.secondary,
                marginBottom: SPACING.xl,
                lineHeight: 1.5
              }}>
                Setting up secure authentication
              </p>

              <div style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: SPACING.sm,
                color: COLORS.text.secondary
              }}>
                <Loader size={20} style={{ animation: 'spin 1s linear infinite' }} />
                <span>Initializing...</span>
              </div>

              {error && (
                <div style={{
                  marginTop: SPACING.lg,
                  padding: SPACING.md,
                  background: `${COLORS.accent.error}15`,
                  border: `1px solid ${COLORS.accent.error}30`,
                  borderRadius: RADIUS.sm,
                  fontSize: TYPOGRAPHY.size.sm,
                  color: COLORS.accent.error
                }}>
                  {error}
                  <Button
                    variant="ghost"
                    onClick={() => {
                      setError('');
                      setIsStarting(false);
                      startDeviceLogin();
                    }}
                    style={{
                      marginTop: SPACING.sm,
                      fontSize: TYPOGRAPHY.size.sm
                    }}
                  >
                    Retry
                  </Button>
                </div>
              )}

              {onCancel && (
                <Button
                  variant="ghost"
                  onClick={onCancel}
                  style={{
                    marginTop: SPACING.md,
                    fontSize: TYPOGRAPHY.size.sm
                  }}
                >
                  Cancel
                </Button>
              )}
            </div>
          </div>
        )}

        {/* Step 2: Show Code */}
        {(step === 'code' || step === 'waiting') && deviceInfo && (
          <div>
            <div style={{ textAlign: 'center' }}>
              <h2 style={{
                fontSize: TYPOGRAPHY.size.xl,
                fontWeight: TYPOGRAPHY.weight.semibold,
                color: COLORS.text.primary,
                marginBottom: SPACING.lg
              }}>
                {step === 'waiting' ? 'Waiting for authorization...' : 'Enter this code on GitHub'}
              </h2>

              {/* The Code */}
              <div style={{
                background: COLORS.bg.primary,
                border: `2px solid ${COLORS.accent.primary}`,
                borderRadius: RADIUS.lg,
                padding: SPACING.xl,
                marginBottom: SPACING.xl,
                position: 'relative'
              }}>
                <div style={{
                  fontSize: '32px',
                  fontFamily: TYPOGRAPHY.font.mono,
                  fontWeight: TYPOGRAPHY.weight.bold,
                  color: COLORS.accent.primary,
                  letterSpacing: '0.2em'
                }}>
                  {deviceInfo?.user_code || 'Loading...'}
                </div>

                <button
                  onClick={copyCode}
                  style={{
                    position: 'absolute',
                    top: SPACING.md,
                    right: SPACING.md,
                    background: 'transparent',
                    border: 'none',
                    cursor: 'pointer',
                    color: copied ? COLORS.accent.success : COLORS.text.tertiary,
                    display: 'flex',
                    alignItems: 'center',
                    gap: '4px',
                    fontSize: TYPOGRAPHY.size.xs
                  }}
                >
                  {copied ? (
                    <>
                      <CheckCircle size={14} />
                      Copied!
                    </>
                  ) : (
                    <>
                      <Copy size={14} />
                      Copy
                    </>
                  )}
                </button>
              </div>

              {/* Instructions */}
              <div style={{
                background: COLORS.bg.elevated,
                borderRadius: RADIUS.md,
                padding: SPACING.lg,
                marginBottom: SPACING.xl
              }}>
                <p style={{
                  fontSize: TYPOGRAPHY.size.sm,
                  color: COLORS.text.secondary,
                  marginBottom: SPACING.sm,
                  fontWeight: 500
                }}>
                  1. Open this link in your browser:
                </p>
                <a
                  href="https://github.com/login/device"
                  target="_blank"
                  rel="noopener noreferrer"
                  style={{
                    display: 'inline-flex',
                    alignItems: 'center',
                    gap: '4px',
                    fontSize: TYPOGRAPHY.size.sm,
                    color: COLORS.accent.primary,
                    textDecoration: 'underline',
                    marginBottom: SPACING.md
                  }}
                >
                  <ExternalLink size={14} />
                  github.com/login/device
                </a>
                <p style={{
                  fontSize: TYPOGRAPHY.size.sm,
                  color: COLORS.text.secondary,
                  marginBottom: SPACING.xs
                }}>
                  2. Enter the code above
                </p>
                <p style={{
                  fontSize: TYPOGRAPHY.size.xs,
                  color: COLORS.text.tertiary
                }}>
                  Note: Use a web browser, not the GitHub mobile app
                </p>
              </div>

              {/* Waiting indicator */}
              {step === 'waiting' && (
                <motion.div
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    gap: SPACING.sm,
                    color: COLORS.text.secondary,
                    fontSize: TYPOGRAPHY.size.sm
                  }}
                >
                  <Loader size={16} style={{ animation: 'spin 1s linear infinite' }} />
                  Waiting for authorization...
                </motion.div>
              )}

              {/* Error */}
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
            </div>
          </div>
        )}

        {/* Step 3: Success */}
        {step === 'success' && (
          <div>
            <div style={{ textAlign: 'center' }}>
              <motion.div
                initial={{ scale: 0 }}
                animate={{ scale: 1 }}
                transition={{ type: 'spring', stiffness: 200 }}
                style={{
                  width: '80px',
                  height: '80px',
                  background: `linear-gradient(135deg, ${COLORS.accent.success}, ${COLORS.accent.primary})`,
                  borderRadius: '50%',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  margin: '0 auto',
                  marginBottom: SPACING.xl
                }}
              >
                <CheckCircle size={40} color="white" />
              </motion.div>

              <h2 style={{
                fontSize: TYPOGRAPHY.size['2xl'],
                fontWeight: TYPOGRAPHY.weight.semibold,
                color: COLORS.text.primary,
                marginBottom: SPACING.md
              }}>
                Success!
              </h2>

              <p style={{
                fontSize: TYPOGRAPHY.size.base,
                color: COLORS.text.secondary
              }}>
                You're now logged in with GitHub
              </p>
            </div>
          </div>
        )}

      {/* Add spinning animation */}
      <style>{`
        @keyframes spin {
          to { transform: rotate(360deg); }
        }
      `}</style>
    </Card>
  );
};