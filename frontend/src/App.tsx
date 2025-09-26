/**
 * App.tsx - CLEAN VERSION using only core design system
 * Single source of truth, consistent everywhere
 */

import React, { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { useDashboardData } from './lib/tauri';
import { Button, Input, StatusDot, Badge, COLORS } from './design-system/core';
import {
  UnifiedHealthView,
  UnifiedTracesView,
  UnifiedServicesView,
  UnifiedDashboardView
} from './pages/unified-views';
import {
  Activity,
  BarChart3,
  Layers,
  GitBranch,
  Share2,
  Search,
  Bell,
  Settings,
  RefreshCw,
  Menu,
  User
} from 'lucide-react';

type ViewMode = 'dashboard' | 'services' | 'traces' | 'health' | 'flows';

const App = () => {
  const [activeView, setActiveView] = useState<ViewMode>('dashboard');
  const [searchQuery, setSearchQuery] = useState('');

  // Data hooks
  const {
    serviceMetrics,
    systemMetrics,
    recentTraces,
    isLoading,
    hasError,
    refetchAll
  } = useDashboardData();

  // Start receiver will be handled by backend when Tauri is available

  const navigation = [
    { key: 'dashboard', icon: BarChart3, label: 'Dashboard', shortcut: '1' },
    { key: 'services', icon: GitBranch, label: 'Services', shortcut: '2' },
    { key: 'traces', icon: Layers, label: 'Traces', shortcut: '3' },
    { key: 'health', icon: Activity, label: 'Health', shortcut: '4' },
    { key: 'flows', icon: Share2, label: 'Flows', shortcut: '5' },
  ] as const;

  return (
    <div style={{ height: '100vh', background: COLORS.bg.primary, display: 'flex', flexDirection: 'column' }}>

      {/* HEADER - Unified Design */}
      <header
        style={{
          background: COLORS.bg.secondary,
          borderBottom: `1px solid ${COLORS.border.subtle}`,
          padding: '12px 24px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between'
        }}
      >
        {/* Logo + Navigation */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '32px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
            <div
              style={{
                width: '32px',
                height: '32px',
                background: COLORS.accent.primary,
                borderRadius: '6px',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center'
              }}
            >
              <Activity size={18} color="white" />
            </div>
            <div>
              <h1 style={{ fontSize: '16px', fontWeight: 600, color: COLORS.text.primary, margin: 0 }}>
                URPO
              </h1>
              <p style={{ fontSize: '11px', color: COLORS.text.tertiary, margin: 0 }}>
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
                  gap: '8px',
                  padding: '8px 12px',
                  background: activeView === key ? COLORS.bg.elevated : 'transparent',
                  color: activeView === key ? COLORS.text.primary : COLORS.text.secondary,
                  border: 'none',
                  borderRadius: '6px',
                  cursor: 'pointer',
                  fontSize: '12px',
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
                <Icon size={16} />
                <span>{label}</span>
                <span
                  style={{
                    fontSize: '10px',
                    padding: '2px 4px',
                    background: COLORS.bg.primary,
                    borderRadius: '3px',
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
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          <div style={{ position: 'relative' }}>
            <Search size={14} style={{ position: 'absolute', left: '12px', top: '50%', transform: 'translateY(-50%)', color: COLORS.text.tertiary }} />
            <Input
              value={searchQuery}
              onChange={setSearchQuery}
              placeholder="Search traces, services..."
              className="search-input"
              style={{ paddingLeft: '36px', width: '300px' }}
            />
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <StatusDot status={hasError ? 'error' : 'success'} pulse />
            <span style={{ fontSize: '11px', color: COLORS.text.tertiary }}>
              {serviceMetrics?.length || 0} services
            </span>
          </div>

          <div style={{ display: 'flex', gap: '4px' }}>
            <Button variant="ghost" size="sm" onClick={refetchAll}>
              <RefreshCw size={14} />
            </Button>
            <Button variant="ghost" size="sm">
              <Bell size={14} />
            </Button>
            <Button variant="ghost" size="sm">
              <Settings size={14} />
            </Button>
          </div>

          <div style={{ marginLeft: '12px', paddingLeft: '12px', borderLeft: `1px solid ${COLORS.border.subtle}` }}>
            <Button variant="ghost" size="sm">
              <User size={14} />
              Admin
            </Button>
          </div>
        </div>
      </header>

      {/* STATUS BAR - System metrics */}
      {systemMetrics && (
        <div
          style={{
            background: COLORS.bg.primary,
            borderBottom: `1px solid ${COLORS.border.subtle}`,
            padding: '8px 24px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between'
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '24px' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <StatusDot status={hasError ? 'warning' : 'success'} />
              <span style={{ fontSize: '11px', color: COLORS.text.secondary }}>
                {hasError ? 'Connection Issues' : 'OTLP Connected'}
              </span>
            </div>

            <div style={{ display: 'flex', gap: '16px' }}>
              <div>
                <span style={{ fontSize: '10px', color: COLORS.text.tertiary }}>SPANS/S</span>
                <span style={{ fontSize: '12px', color: COLORS.text.primary, marginLeft: '8px' }}>
                  {systemMetrics?.spans_per_second?.toFixed(0) || '0'}
                </span>
              </div>
              <div>
                <span style={{ fontSize: '10px', color: COLORS.text.tertiary }}>MEMORY</span>
                <span style={{ fontSize: '12px', color: COLORS.text.primary, marginLeft: '8px' }}>
                  {systemMetrics?.memory_usage_mb?.toFixed(0) || '0'}MB
                </span>
              </div>
              <div>
                <span style={{ fontSize: '10px', color: COLORS.text.tertiary }}>CPU</span>
                <span style={{ fontSize: '12px', color: COLORS.text.primary, marginLeft: '8px' }}>
                  {systemMetrics?.cpu_usage_percent?.toFixed(1) || '0'}%
                </span>
              </div>
            </div>
          </div>

          <div style={{ fontSize: '11px', color: COLORS.text.tertiary }}>
            Last updated: {new Date().toLocaleTimeString()}
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
              <UnifiedDashboardView data={{ services: serviceMetrics, traces: recentTraces }} />
            )}
            {activeView === 'services' && (
              <UnifiedServicesView services={serviceMetrics} />
            )}
            {activeView === 'traces' && (
              <UnifiedTracesView traces={recentTraces} />
            )}
            {activeView === 'health' && (
              <UnifiedHealthView services={serviceMetrics} metrics={systemMetrics} />
            )}
            {activeView === 'flows' && (
              <UnifiedTracesView traces={recentTraces} />
            )}
          </motion.div>
        </AnimatePresence>
      </main>
    </div>
  );
};

export default App;