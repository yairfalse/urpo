/**
 * App.tsx - CLEAN VERSION using only core design system
 * Single source of truth, consistent everywhere
 */

import React, { useState, useMemo } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { useDashboardData } from './lib/tauri/convenience';
import { Button, Input, StatusDot, Badge, COLORS } from './design-system/core';
import {
  UnifiedHealthView,
  UnifiedTracesView,
  UnifiedServicesView,
  UnifiedDashboardView
} from './pages/unified-views';
import { invoke } from '@tauri-apps/api/tauri';
import { LoginPage } from './pages/LoginPage';
import {
  Activity,
  BarChart3,
  Layers,
  GitBranch,
  Share2,
  Search,
  RefreshCw,
  User,
  LogOut
} from 'lucide-react';

type ViewMode = 'dashboard' | 'services' | 'traces' | 'health' | 'flows';

const App = () => {
  const [activeView, setActiveView] = useState<ViewMode>('dashboard');
  const [searchQuery, setSearchQuery] = useState('');
  const [currentUser, setCurrentUser] = useState<string | null>(null);

  // Data hooks
  const {
    serviceMetrics,
    systemMetrics,
    recentTraces,
    isLoading,
    hasError,
    refetchAll
  } = useDashboardData();

  const handleLogin = (username: string, password?: string) => {
    // Handle both GitHub OAuth and regular login
    setCurrentUser(username);
    localStorage.setItem('urpo_user', username);
  };

  const handleLogout = async () => {
    try {
      await invoke('logout');
      setCurrentUser(null);
      localStorage.removeItem('urpo_user');
    } catch (error) {
      console.error('Logout failed:', error);
      // Still clear local state even if backend fails
      setCurrentUser(null);
      localStorage.removeItem('urpo_user');
    }
  };

  // Check for saved user on mount (including GitHub OAuth tokens)
  React.useEffect(() => {
    const checkAuth = async () => {
      try {
        // First check if we have a GitHub user logged in
        const githubUser = await invoke('get_current_user');
        if (githubUser) {
          setCurrentUser(githubUser.username);
          localStorage.setItem('urpo_user', githubUser.username);
          return;
        }
      } catch (error) {
        // No GitHub user, check localStorage
        console.log('No GitHub user found');
      }

      // Fallback to localStorage
      const savedUser = localStorage.getItem('urpo_user');
      if (savedUser) {
        setCurrentUser(savedUser);
      }
    };

    checkAuth();
  }, []);

  // Filter traces based on search
  const filteredTraces = useMemo(() => {
    if (!searchQuery || !recentTraces) return recentTraces;
    const query = searchQuery.toLowerCase();
    return recentTraces.filter((trace: any) =>
      trace.root_service?.toLowerCase().includes(query) ||
      trace.trace_id?.toLowerCase().includes(query) ||
      trace.root_operation?.toLowerCase().includes(query)
    );
  }, [recentTraces, searchQuery]);

  // Filter services based on search
  const filteredServices = useMemo(() => {
    if (!searchQuery || !serviceMetrics) return serviceMetrics;
    const query = searchQuery.toLowerCase();
    return serviceMetrics.filter((service: any) =>
      service.name?.toLowerCase().includes(query)
    );
  }, [serviceMetrics, searchQuery]);

  // Start receiver will be handled by backend when Tauri is available

  const navigation = [
    { key: 'dashboard', icon: BarChart3, label: 'Dashboard', shortcut: '1' },
    { key: 'services', icon: GitBranch, label: 'Services', shortcut: '2' },
    { key: 'traces', icon: Layers, label: 'Traces', shortcut: '3' },
    { key: 'health', icon: Activity, label: 'Health', shortcut: '4' },
    { key: 'flows', icon: Share2, label: 'Flows', shortcut: '5' },
  ] as const;

  // Show login page if not authenticated
  if (!currentUser) {
    return <LoginPage onLogin={handleLogin} />;
  }

  return (
    <div style={{ height: '100vh', background: COLORS.bg.primary, display: 'flex', flexDirection: 'column' }}>

      {/* HEADER - Unified Design */}
      <header
        style={{
          background: COLORS.bg.secondary,
          borderBottom: `1px solid ${COLORS.border.subtle}`,
          padding: '8px 16px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between'
        }}
      >
        {/* Logo + Navigation */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '20px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <div
              style={{
                width: '28px',
                height: '28px',
                background: COLORS.accent.primary,
                borderRadius: '4px',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center'
              }}
            >
              <Activity size={16} color="white" />
            </div>
            <div>
              <h1 style={{ fontSize: '14px', fontWeight: 600, color: COLORS.text.primary, margin: 0 }}>
                URPO
              </h1>
              <p style={{ fontSize: '10px', color: COLORS.text.tertiary, margin: 0 }}>
                Trace Explorer
              </p>
            </div>
          </div>

          <nav style={{ display: 'flex', gap: '4px' }}>
            {navigation.map(({ key, icon: Icon, label, shortcut }) => (
              <button
                key={key}
                onClick={() => setActiveView(key as ViewMode)}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '6px',
                  padding: '6px 10px',
                  background: activeView === key ? COLORS.bg.elevated : 'transparent',
                  color: activeView === key ? COLORS.text.primary : COLORS.text.secondary,
                  border: 'none',
                  borderRadius: '4px',
                  cursor: 'pointer',
                  fontSize: '11px',
                  fontWeight: 500,
                  transition: 'all 0.15s ease'
                }}
                onMouseEnter={(e) => {
                  if (activeView !== key) {
                    e.currentTarget.style.background = COLORS.bg.elevated;
                    e.currentTarget.style.color = COLORS.text.primary;
                  }
                }}
                onMouseLeave={(e) => {
                  if (activeView !== key) {
                    e.currentTarget.style.background = 'transparent';
                    e.currentTarget.style.color = COLORS.text.secondary;
                  }
                }}
              >
                <Icon size={14} />
                <span>{label}</span>
                <span
                  style={{
                    fontSize: '9px',
                    padding: '1px 3px',
                    background: COLORS.bg.primary,
                    borderRadius: '2px',
                    color: COLORS.text.tertiary
                  }}
                >
                  {shortcut}
                </span>
              </button>
            ))}
          </nav>
        </div>

        {/* Search + Actions */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <div style={{ position: 'relative' }}>
            <Search size={12} style={{ position: 'absolute', left: '10px', top: '50%', transform: 'translateY(-50%)', color: COLORS.text.tertiary }} />
            <Input
              value={searchQuery}
              onChange={(value) => setSearchQuery(value)}
              placeholder="Search..."
              className="search-input"
              style={{ paddingLeft: '30px', width: '200px', height: '28px', fontSize: '11px' }}
            />
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
            <StatusDot status={hasError ? 'error' : 'success'} pulse />
            <span style={{ fontSize: '10px', color: COLORS.text.tertiary }}>
              {serviceMetrics?.length || 0} services
            </span>
          </div>

          <Button variant="ghost" size="sm" onClick={refetchAll} style={{ padding: '4px' }}>
            <RefreshCw size={12} />
          </Button>

          {/* User button */}
          <div style={{ marginLeft: '8px', paddingLeft: '8px', borderLeft: `1px solid ${COLORS.border.subtle}` }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
              <span style={{ fontSize: '11px', color: COLORS.text.secondary }}>
                {currentUser}
              </span>
              <Button
                variant="ghost"
                size="sm"
                onClick={handleLogout}
                style={{ padding: '4px' }}
                title="Logout"
              >
                <LogOut size={12} />
              </Button>
            </div>
          </div>
        </div>
      </header>

      {/* STATUS BAR - System metrics */}
      {systemMetrics && (
        <div
          style={{
            background: COLORS.bg.primary,
            borderBottom: `1px solid ${COLORS.border.subtle}`,
            padding: '4px 16px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between'
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
              <StatusDot status={hasError ? 'warning' : 'success'} />
              <span style={{ fontSize: '10px', color: COLORS.text.secondary }}>
                {hasError ? 'Connection Issues' : 'OTLP Connected'}
              </span>
            </div>

            <div style={{ display: 'flex', gap: '12px' }}>
              <div>
                <span style={{ fontSize: '9px', color: COLORS.text.tertiary }}>SPANS/S</span>
                <span style={{ fontSize: '10px', color: COLORS.text.primary, marginLeft: '6px' }}>
                  {systemMetrics?.spans_per_second?.toFixed(0) || '0'}
                </span>
              </div>
              <div>
                <span style={{ fontSize: '9px', color: COLORS.text.tertiary }}>MEM</span>
                <span style={{ fontSize: '10px', color: COLORS.text.primary, marginLeft: '6px' }}>
                  {systemMetrics?.memory_usage_mb?.toFixed(0) || '0'}MB
                </span>
              </div>
              <div>
                <span style={{ fontSize: '9px', color: COLORS.text.tertiary }}>CPU</span>
                <span style={{ fontSize: '10px', color: COLORS.text.primary, marginLeft: '6px' }}>
                  {systemMetrics?.cpu_usage_percent?.toFixed(1) || '0'}%
                </span>
              </div>
            </div>
          </div>

          <div style={{ fontSize: '9px', color: COLORS.text.tertiary }}>
            {new Date().toLocaleTimeString()}
          </div>
        </div>
      )}

      {/* MAIN CONTENT */}
      <main style={{ flex: 1, overflow: 'hidden' }}>
        <AnimatePresence mode="wait">
          <motion.div
            key={activeView}
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -10 }}
            transition={{ duration: 0.15 }}
            style={{ height: '100%' }}
          >
            {activeView === 'dashboard' && (
              <UnifiedDashboardView data={{ services: filteredServices, traces: filteredTraces }} />
            )}
            {activeView === 'services' && (
              <UnifiedServicesView services={filteredServices} />
            )}
            {activeView === 'traces' && (
              <UnifiedTracesView traces={filteredTraces} />
            )}
            {activeView === 'health' && (
              <UnifiedHealthView services={filteredServices} metrics={systemMetrics} />
            )}
            {activeView === 'flows' && (
              <UnifiedTracesView traces={filteredTraces} />
            )}
          </motion.div>
        </AnimatePresence>
      </main>
    </div>
  );
};

export default App;