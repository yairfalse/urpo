/**
 * App.tsx - Linear/Vercel Quality Interface
 *
 * Minimal, consistent, and professional
 */

import React, { memo } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { useKeyboardShortcuts } from './hooks/useKeyboardShortcuts';
import { ErrorBoundary } from './components/common/ErrorBoundary';
import { ServiceHealthDashboard } from './components/tables/ServiceHealthDashboard';
import { TraceExplorer } from './components/tables/TraceExplorer';
import { SystemMetrics } from './components/panels/SystemMetrics';
import { ServiceGraphPro } from './components/charts/ServiceGraphPro';
import { ServiceMap } from './components/tables/ServiceMap';
import { FlowTable } from './components/tables/FlowTable';
import { VirtualizedFlowTable } from './components/tables/VirtualizedFlowTable';
import {
  useDashboardData,
  useAppStore,
  useStartReceiver,
  type ViewMode
} from './lib/tauri';
import {
  Button,
  Input,
  NavItem,
  StatusIndicator,
  Dropdown,
  DropdownItem,
  LoadingScreen,
  Header,
  Page,
  Section,
  Badge,
  Metric
} from './components-refined';
import {
  UnifiedPage,
  UnifiedTable,
  UnifiedCard,
  UnifiedMetrics,
  UnifiedEmptyState,
  UnifiedLoadingState,
  UnifiedList
} from './components/unified-layout';
import {
  Activity,
  BarChart3,
  Layers,
  GitBranch,
  Share2,
  Search,
  Filter,
  RefreshCw,
  Settings,
  Bell,
  ChevronDown,
  Download,
  type LucideIcon
} from 'lucide-react';

interface NavigationItem {
  key: ViewMode;
  icon: LucideIcon;
  label: string;
  shortcut: string;
}

const App = memo(() => {
  // ============================================================================
  // STATE AND DATA
  // ============================================================================

  const {
    activeView,
    setActiveView,
    searchQuery,
    setSearchQuery,
    showFilters,
    showNotifications,
    showSettings,
    showUserMenu,
    toggleFilters,
    toggleNotifications,
    toggleSettings,
    toggleUserMenu,
    closeAllDropdowns,
  } = useAppStore();

  const {
    serviceMetrics,
    systemMetrics,
    recentTraces,
    isLoading,
    hasError,
    refetchAll
  } = useDashboardData();

  const startReceiver = useStartReceiver({
    onError: (error: any) => {
      console.error('Failed to start OTEL receiver:', error);
    }
  });

  React.useEffect(() => {
    if (!startReceiver.isSuccess && !startReceiver.isLoading) {
      startReceiver.mutate();
    }
  }, [startReceiver.isSuccess, startReceiver.isLoading, startReceiver.mutate]);

  // ============================================================================
  // KEYBOARD SHORTCUTS
  // ============================================================================

  useKeyboardShortcuts([
    { key: '1', handler: () => setActiveView('graph'), description: 'Service Map' },
    { key: '2', handler: () => setActiveView('flows'), description: 'Trace Flows' },
    { key: '3', handler: () => setActiveView('health'), description: 'Health' },
    { key: '4', handler: () => setActiveView('traces'), description: 'Traces' },
    { key: '5', handler: () => setActiveView('servicemap'), description: 'Dependencies' },
    { key: 'r', handler: refetchAll, description: 'Refresh', ctrl: true },
    { key: '/', handler: () => {
      document.getElementById('global-search')?.focus();
      closeAllDropdowns();
    }, description: 'Search' },
  ]);

  // ============================================================================
  // LOADING STATE
  // ============================================================================

  if (isLoading) {
    return <LoadingScreen />;
  }

  const navigationItems: NavigationItem[] = [
    { key: 'graph', icon: GitBranch, label: 'Service Map', shortcut: '1' },
    { key: 'flows', icon: Activity, label: 'Flows', shortcut: '2' },
    { key: 'health', icon: BarChart3, label: 'Health', shortcut: '3' },
    { key: 'traces', icon: Layers, label: 'Traces', shortcut: '4' },
    { key: 'servicemap', icon: Share2, label: 'Dependencies', shortcut: '5' },
  ];

  return (
    <ErrorBoundary componentName="App">
      <div className="h-screen bg-gray-950 text-gray-50 flex flex-col">

        {/* ============================================================================
            HEADER - MINIMAL & REFINED
            ============================================================================ */}
        <Header>
          <div className="px-6 py-4">
            <div className="flex items-center justify-between">

              {/* Logo & Navigation */}
              <div className="flex items-center gap-8">
                <div className="flex items-center gap-3">
                  <div className="w-8 h-8 rounded-md bg-blue-600 flex items-center justify-center">
                    <Activity size={18} className="text-white" />
                  </div>
                  <div>
                    <h1 className="text-lg font-semibold text-gray-50">URPO</h1>
                    <p className="text-xs text-gray-500 font-medium">Trace Explorer</p>
                  </div>
                </div>

                <nav className="flex items-center gap-1">
                  {navigationItems.map(({ key, icon: Icon, label, shortcut }) => (
                    <NavItem
                      key={key}
                      icon={Icon}
                      label={label}
                      active={activeView === key}
                      shortcut={shortcut}
                      onClick={() => setActiveView(key)}
                    />
                  ))}
                </nav>
              </div>

              {/* Actions */}
              <div className="flex items-center gap-4">
                <Input
                  id="global-search"
                  icon={Search}
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder="Search traces, services..."
                  className="w-80"
                  rightElement={<Badge>⌘K</Badge>}
                />

                <div className="flex items-center gap-2">
                  <StatusIndicator
                    status={hasError ? 'warning' : 'online'}
                    label="Live"
                    pulse
                  />
                  <div className="text-xs text-gray-500">
                    {(serviceMetrics.data as any)?.length || 0} services
                  </div>
                </div>

                <div className="flex items-center gap-1">
                  <Button
                    variant="ghost"
                    size="sm"
                    icon={Filter}
                    onClick={toggleFilters}
                    className={showFilters ? 'bg-gray-800' : ''}
                  />
                  <Button
                    variant="ghost"
                    size="sm"
                    icon={RefreshCw}
                    onClick={refetchAll}
                  />

                  <div className="relative">
                    <Button
                      variant="ghost"
                      size="sm"
                      icon={Bell}
                      onClick={toggleNotifications}
                      className={showNotifications ? 'bg-gray-800' : ''}
                    />
                    <div className="absolute -top-1 -right-1 w-2 h-2 bg-red-500 rounded-full" />
                  </div>

                  <Button
                    variant="ghost"
                    size="sm"
                    icon={Settings}
                    onClick={toggleSettings}
                    className={showSettings ? 'bg-gray-800' : ''}
                  />
                </div>

                <div className="relative ml-2 pl-2 border-l border-gray-800">
                  <motion.button
                    onClick={toggleUserMenu}
                    className="flex items-center gap-2 hover:bg-gray-800 rounded-md px-2 py-1 transition-colors"
                  >
                    <div className="w-6 h-6 bg-blue-600 rounded-full flex items-center justify-center text-white text-xs font-semibold">
                      U
                    </div>
                    <motion.div
                      animate={{ rotate: showUserMenu ? 180 : 0 }}
                      transition={{ duration: 0.15 }}
                    >
                      <ChevronDown size={14} className="text-gray-400" />
                    </motion.div>
                  </motion.button>

                  <Dropdown isOpen={showUserMenu} onClose={() => toggleUserMenu()} className="w-48">
                    <div className="p-2">
                      <div className="flex items-center gap-3 p-2 mb-2 rounded-sm bg-gray-800">
                        <div className="w-8 h-8 bg-blue-600 rounded-full flex items-center justify-center text-white text-sm font-semibold">
                          U
                        </div>
                        <div>
                          <p className="text-sm font-medium text-gray-50">Admin</p>
                          <p className="text-xs text-gray-500">admin@urpo.dev</p>
                        </div>
                      </div>
                      <DropdownItem>Profile</DropdownItem>
                      <DropdownItem>Settings</DropdownItem>
                      <DropdownItem>API Keys</DropdownItem>
                      <div className="border-t border-gray-800 my-1" />
                      <DropdownItem>Help</DropdownItem>
                      <DropdownItem variant="danger">Sign Out</DropdownItem>
                    </div>
                  </Dropdown>
                </div>
              </div>
            </div>
          </div>

          {/* Metrics Bar */}
          {systemMetrics.data && (
            <motion.div
              initial={{ opacity: 0, height: 0 }}
              animate={{ opacity: 1, height: 'auto' }}
              className="px-6 py-3 bg-gray-900 border-t border-gray-800"
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-6">
                  <StatusIndicator
                    status={hasError ? 'warning' : 'online'}
                    label={hasError ? 'Connection Issues' : 'OTLP Connected'}
                  />

                  <div className="flex items-center gap-4">
                    <Metric
                      label="Services"
                      value={(serviceMetrics.data as any)?.length || 0}
                    />
                    <Metric
                      label="Traces"
                      value={(recentTraces.data as any)?.length || 0}
                    />
                    <Metric
                      label="Spans/s"
                      value={(systemMetrics.data as any)?.spans_per_second?.toFixed(0) || '0'}
                    />
                    <Metric
                      label="Memory"
                      value={`${(systemMetrics.data as any)?.memory_usage_mb?.toFixed(0) || '0'}MB`}
                    />
                    <Metric
                      label="CPU"
                      value={`${(systemMetrics.data as any)?.cpu_usage_percent?.toFixed(1) || '0'}%`}
                    />
                  </div>
                </div>

                <select className="bg-gray-800 border border-gray-700 rounded-base px-3 py-1 text-xs text-gray-200">
                  <option>Last 15 minutes</option>
                  <option>Last 1 hour</option>
                  <option>Last 6 hours</option>
                  <option>Last 24 hours</option>
                </select>
              </div>
            </motion.div>
          )}
        </Header>

        {/* ============================================================================
            FILTERS
            ============================================================================ */}
        <AnimatePresence>
          {showFilters && (
            <motion.div
              initial={{ opacity: 0, height: 0 }}
              animate={{ opacity: 1, height: 'auto' }}
              exit={{ opacity: 0, height: 0 }}
              className="bg-gray-900 border-b border-gray-800 px-6 py-4"
            >
              <div className="flex items-center gap-4">
                <div className="flex items-center gap-2">
                  <label className="text-sm font-medium text-gray-300">Service:</label>
                  <select className="bg-gray-800 border border-gray-700 rounded-base px-3 py-1 text-sm text-gray-200">
                    <option>All Services</option>
                    {(serviceMetrics.data as any)?.map((service: any) => (
                      <option key={service.name} value={service.name}>
                        {service.name}
                      </option>
                    ))}
                  </select>
                </div>
                <div className="flex items-center gap-2">
                  <label className="text-sm font-medium text-gray-300">Status:</label>
                  <select className="bg-gray-800 border border-gray-700 rounded-base px-3 py-1 text-sm text-gray-200">
                    <option>All Status</option>
                    <option>Healthy</option>
                    <option>Warning</option>
                    <option>Error</option>
                  </select>
                </div>
              </div>
            </motion.div>
          )}
        </AnimatePresence>

        {/* ============================================================================
            DROPDOWNS (NOTIFICATIONS & SETTINGS)
            ============================================================================ */}
        <div className="relative">
          <Dropdown isOpen={showNotifications} onClose={() => toggleNotifications()}>
            <div className="p-4">
              <h3 className="text-sm font-semibold text-gray-50 mb-3">Notifications</h3>
              <div className="space-y-2">
                <div className="flex items-start gap-3 p-3 bg-gray-800 rounded-base">
                  <StatusIndicator status="error" />
                  <div>
                    <p className="text-sm text-gray-200">High error rate detected</p>
                    <p className="text-xs text-gray-500">payment-service • 5m ago</p>
                  </div>
                </div>
                <div className="flex items-start gap-3 p-3 bg-gray-800 rounded-base">
                  <StatusIndicator status="warning" />
                  <div>
                    <p className="text-sm text-gray-200">Latency spike observed</p>
                    <p className="text-xs text-gray-500">auth-service • 12m ago</p>
                  </div>
                </div>
              </div>
            </div>
          </Dropdown>
        </div>

        <div className="relative">
          <Dropdown isOpen={showSettings} onClose={() => toggleSettings()} className="w-48">
            <div className="p-2">
              <DropdownItem>Theme</DropdownItem>
              <DropdownItem>Refresh Rate</DropdownItem>
              <DropdownItem>Export</DropdownItem>
              <DropdownItem>Shortcuts</DropdownItem>
              <div className="border-t border-gray-800 my-1" />
              <DropdownItem>About</DropdownItem>
            </div>
          </Dropdown>
        </div>

        {/* ============================================================================
            ERROR BANNER
            ============================================================================ */}
        <AnimatePresence>
          {hasError && (
            <motion.div
              initial={{ opacity: 0, height: 0 }}
              animate={{ opacity: 1, height: 'auto' }}
              exit={{ opacity: 0, height: 0 }}
              className="px-6 py-3 bg-red-500/10 border-b border-red-500/30"
            >
              <div className="flex items-center gap-2">
                <StatusIndicator status="error" />
                <span className="text-sm text-red-400">
                  Connection error - check OTEL receiver
                </span>
              </div>
            </motion.div>
          )}
        </AnimatePresence>

        {/* ============================================================================
            MAIN CONTENT
            ============================================================================ */}
        <main className="flex-1 overflow-hidden bg-gray-950">
          <AnimatePresence mode="wait">
            <motion.div
              key={activeView}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -10 }}
              transition={{ duration: 0.15 }}
              className="h-full"
            >
              {activeView === 'graph' && (
                <ErrorBoundary componentName="ServiceGraphPro" isolate>
                  <UnifiedPage
                    title="Service Dependency Map"
                    subtitle="Real-time service interactions and health visualization"
                    icon={GitBranch}
                    onRefresh={refetchAll}
                    isLoading={isLoading}
                    actions={
                      <>
                        <Button variant="secondary" size="sm" icon={Settings}>Auto Layout</Button>
                        <Button variant="secondary" size="sm" icon={Download}>Export</Button>
                      </>
                    }
                  >
                    <ServiceGraphPro
                      services={(serviceMetrics.data as any) || []}
                      traces={(recentTraces.data as any) || []}
                    />
                  </UnifiedPage>
                </ErrorBoundary>
              )}

              {activeView === 'flows' && (
                <ErrorBoundary componentName="FlowTable" isolate>
                  <UnifiedPage
                    title="Trace Flows"
                    subtitle="Real-time trace flow visualization and analysis"
                    icon={Activity}
                    onRefresh={recentTraces.refetch}
                    isLoading={recentTraces.isLoading}
                    actions={
                      <>
                        <Button variant="secondary" size="sm" icon={Filter}>Filter</Button>
                        <Button variant="secondary" size="sm" icon={Download}>Export</Button>
                      </>
                    }
                  >
                    {((recentTraces.data as any)?.length || 0) > 100 ? (
                      <VirtualizedFlowTable
                        traces={(recentTraces.data as any) || []}
                        onRefresh={recentTraces.refetch}
                      />
                    ) : (
                      <FlowTable
                        traces={(recentTraces.data as any) || []}
                        onRefresh={recentTraces.refetch}
                      />
                    )}
                  </UnifiedPage>
                </ErrorBoundary>
              )}

              {activeView === 'health' && (
                <ErrorBoundary componentName="HealthView" isolate>
                  <UnifiedPage
                    title="Service Health"
                    subtitle="System health metrics and performance indicators"
                    icon={BarChart3}
                    onRefresh={refetchAll}
                    isLoading={isLoading}
                    actions={
                      <>
                        <Button variant="secondary" size="sm" icon={Settings}>Configure</Button>
                        <Button variant="secondary" size="sm" icon={Download}>Report</Button>
                      </>
                    }
                  >
                    <div className="space-y-6">
                      <ServiceHealthDashboard services={(serviceMetrics.data as any) || []} />
                      {systemMetrics.data && (
                        <SystemMetrics metrics={(systemMetrics.data as any)} />
                      )}
                    </div>
                  </UnifiedPage>
                </ErrorBoundary>
              )}

              {activeView === 'traces' && (
                <ErrorBoundary componentName="TraceExplorer" isolate>
                  <UnifiedPage
                    title="Trace Explorer"
                    subtitle="Deep dive into distributed trace data"
                    icon={Layers}
                    onRefresh={recentTraces.refetch}
                    isLoading={recentTraces.isLoading}
                    actions={
                      <>
                        <Button variant="secondary" size="sm" icon={Search}>Search</Button>
                        <Button variant="secondary" size="sm" icon={Filter}>Filter</Button>
                      </>
                    }
                  >
                    <TraceExplorer
                      traces={(recentTraces.data as any) || []}
                      onRefresh={recentTraces.refetch}
                    />
                  </UnifiedPage>
                </ErrorBoundary>
              )}

              {activeView === 'servicemap' && (
                <ErrorBoundary componentName="ServiceMap" isolate>
                  <UnifiedPage
                    title="Service Dependencies"
                    subtitle="Service topology and dependency mapping"
                    icon={Share2}
                    onRefresh={refetchAll}
                    isLoading={isLoading}
                    actions={
                      <>
                        <Button variant="secondary" size="sm" icon={Settings}>Layout</Button>
                        <Button variant="secondary" size="sm" icon={Download}>Export</Button>
                      </>
                    }
                  >
                    <ServiceMap />
                  </UnifiedPage>
                </ErrorBoundary>
              )}
            </motion.div>
          </AnimatePresence>
        </main>

        {/* ============================================================================
            STATUS BAR
            ============================================================================ */}
        <footer className="bg-gray-900 border-t border-gray-800 px-6 py-2">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4 text-xs">
              <span className="text-gray-500">© 2025 URPO</span>
              <Badge size="sm">v0.1.0</Badge>
            </div>

            <div className="flex items-center gap-4">
              {systemMetrics.data && (
                <>
                  <Metric
                    label="Spans"
                    value={(systemMetrics.data as any)?.total_spans?.toLocaleString() || '0'}
                  />
                  <Metric
                    label="Uptime"
                    value={`${Math.floor(((systemMetrics.data as any)?.uptime_seconds || 0) / 60)}m`}
                  />
                </>
              )}
            </div>
          </div>
        </footer>
      </div>
    </ErrorBoundary>
  );
});

App.displayName = 'App';

export default App;