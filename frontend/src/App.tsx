import React, { memo } from 'react';
import { motion } from 'framer-motion';
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
} from './components';
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
  type LucideIcon
} from 'lucide-react';

// Navigation item type
interface NavigationItem {
  key: ViewMode;
  icon: LucideIcon;
  label: string;
  shortcut: string;
}

const App = memo(() => {
  // ============================================================================
  // HOOKS AND STATE
  // ============================================================================

  // Global app state from Zustand store
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

  // Data fetching with React Query hooks
  const {
    serviceMetrics,
    systemMetrics,
    recentTraces,
    isLoading,
    hasError,
    refetchAll
  } = useDashboardData();

  // Start the OTLP receiver on app initialization
  const startReceiver = useStartReceiver({
    onError: (error: any) => {
      console.error('Failed to start OTEL receiver:', error);
    }
  });

  // Initialize receiver on mount
  React.useEffect(() => {
    startReceiver.mutate(undefined);
  }, [startReceiver]);

  // ============================================================================
  // KEYBOARD SHORTCUTS
  // ============================================================================

  useKeyboardShortcuts([
    { key: '1', handler: () => setActiveView('graph'), description: 'Service Map' },
    { key: '2', handler: () => setActiveView('flows'), description: 'Trace Flows' },
    { key: '3', handler: () => setActiveView('health'), description: 'Health Metrics' },
    { key: '4', handler: () => setActiveView('traces'), description: 'Traces' },
    { key: '5', handler: () => setActiveView('servicemap'), description: 'Dependencies' },
    { key: 'r', handler: refetchAll, description: 'Refresh', ctrl: true },
    { key: '/', handler: () => {
      document.getElementById('global-search')?.focus();
      // Close any open dropdowns when focusing search
      closeAllDropdowns();
    }, description: 'Search' },
  ]);

  // ============================================================================
  // LOADING AND ERROR STATES
  // ============================================================================

  if (isLoading) {
    return <LoadingScreen />;
  }

  const navigationItems: NavigationItem[] = [
    { key: 'graph', icon: GitBranch, label: 'Service Map', shortcut: '1' },
    { key: 'flows', icon: Activity, label: 'Trace Flows', shortcut: '2' },
    { key: 'health', icon: BarChart3, label: 'Health', shortcut: '3' },
    { key: 'traces', icon: Layers, label: 'Traces', shortcut: '4' },
    { key: 'servicemap', icon: Share2, label: 'Dependencies', shortcut: '5' },
  ];

  return (
    <ErrorBoundary componentName="App">
      <div className="h-screen bg-dark-0 text-light-50 flex flex-col">
        <Header>
          <div className="px-6 py-4">
            <div className="flex items-center justify-between">
              {/* Logo and Brand */}
              <div className="flex items-center gap-8">
                <div className="flex items-center gap-4">
                  <div className="relative">
                    <div className="absolute inset-0 bg-gradient-to-br from-data-blue to-data-cyan rounded-xl blur-md opacity-50"></div>
                    <div className="relative w-12 h-12 bg-gradient-to-br from-data-blue to-data-cyan rounded-xl flex items-center justify-center shadow-glow-sm">
                      <Activity className="w-7 h-7 text-white" />
                    </div>
                  </div>
                  <div>
                    <h1 className="text-2xl font-bold text-glow bg-gradient-to-r from-white to-light-200 bg-clip-text text-transparent">
                      URPO
                    </h1>
                    <p className="text-xs text-light-400 uppercase tracking-widest font-medium">
                      Professional Trace Explorer
                    </p>
                  </div>
                </div>

                {/* Navigation with sharp styling */}
                <nav className="flex items-center gap-2">
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

              {/* Actions and Status */}
              <div className="flex items-center gap-6">
                {/* Professional Search */}
                <Input
                  id="global-search"
                  icon={Search}
                  type="text"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder="Search traces, services..."
                  className="w-80"
                  rightElement={<Badge>⌘K</Badge>}
                />

                {/* Status indicators */}
                <div className="flex items-center gap-3 px-4 py-2 bg-dark-100 rounded-lg border border-dark-400">
                  <StatusIndicator status="online" label="Live" pulse />
                  <div className="w-px h-4 bg-dark-300"></div>
                  <div className="text-xs text-light-400">
                    {(serviceMetrics.data as any)?.length || 0} services
                  </div>
                </div>

                {/* Action Buttons */}
                <div className="flex items-center gap-2">
                  <Button
                    variant="ghost"
                    size="sm"
                    icon={Filter}
                    onClick={toggleFilters}
                    title="Toggle Filters"
                    className={showFilters ? 'bg-dark-200 text-data-blue' : ''}
                  ></Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    icon={RefreshCw}
                    onClick={refetchAll}
                    title="Refresh Data"
                  ></Button>
                  <div className="relative">
                    <Button
                      variant="ghost"
                      size="sm"
                      icon={Bell}
                      onClick={toggleNotifications}
                      title="Notifications"
                      className={showNotifications ? 'bg-dark-200 text-data-blue' : ''}
                    ></Button>
                    <span className="absolute -top-1 -right-1 w-2 h-2 bg-semantic-error rounded-full" />
                  </div>
                  <Button
                    variant="ghost"
                    size="sm"
                    icon={Settings}
                    onClick={toggleSettings}
                    title="Settings"
                    className={showSettings ? 'bg-dark-200 text-data-blue' : ''}
                  ></Button>
                </div>

                {/* User Menu */}
                <div className="relative">
                  <motion.button
                    onClick={toggleUserMenu}
                    className="flex items-center gap-2 pl-4 border-l border-dark-300 hover:bg-dark-200 rounded-r-lg px-2 py-1 transition-colors"
                  >
                    <div className="w-8 h-8 bg-gradient-to-br from-data-purple to-data-pink rounded-full flex items-center justify-center text-white font-semibold text-sm">
                      U
                    </div>
                    <motion.div
                      animate={{ rotate: showUserMenu ? 180 : 0 }}
                      transition={{ duration: 0.2 }}
                    >
                      <ChevronDown className="w-4 h-4 text-light-500" />
                    </motion.div>
                  </motion.button>
                </div>

              </div>
            </div>
          </div>

          {/* Sub-header with Metrics */}
          {systemMetrics.data && (
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              className="px-6 py-3 bg-dark-150 border-t border-dark-300"
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-6">
                  <StatusIndicator
                    status={!hasError ? 'online' : 'warning'}
                    label={!hasError ? 'OTLP Connected' : 'Connection Issues'}
                    pulse
                  />

                  {/* Key Metrics Bar */}
                  <div className="flex items-center gap-4">
                    <Metric
                      label="Services"
                      value={(serviceMetrics.data as any)?.length || 0}
                      color="blue"
                    />
                    <Metric
                      label="Traces"
                      value={(recentTraces.data as any)?.length || 0}
                      color="blue"
                    />
                    <Metric
                      label="Spans/s"
                      value={(systemMetrics.data as any)?.spans_per_second?.toFixed(0) || '0'}
                      color="cyan"
                    />
                    <Metric
                      label="Memory"
                      value={`${(systemMetrics.data as any)?.memory_usage_mb?.toFixed(0) || '0'}MB`}
                      color="yellow"
                    />
                    <Metric
                      label="CPU"
                      value={`${(systemMetrics.data as any)?.cpu_usage_percent?.toFixed(1) || '0'}%`}
                      color="red"
                    />
                  </div>
                </div>

                {/* Time Range Selector */}
                <div className="flex items-center gap-2">
                  <span className="text-xs text-light-500">Time Range</span>
                  <select className="bg-dark-200 border border-dark-400 rounded px-2 py-1 text-xs text-light-200">
                    <option>Last 15 minutes</option>
                    <option>Last 1 hour</option>
                    <option>Last 6 hours</option>
                    <option>Last 24 hours</option>
                  </select>
                </div>
              </div>
            </motion.div>
          )}
        </Header>

        {/* Filter Panel */}
        {showFilters && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
            className="bg-dark-100 border-b border-dark-300 px-6 py-4"
          >
            <div className="flex items-center gap-4">
              <div className="flex items-center gap-2">
                <label className="text-sm font-medium text-light-300">Service:</label>
                <select className="bg-dark-200 border border-dark-400 rounded px-3 py-1 text-sm text-light-200">
                  <option>All Services</option>
                  {(serviceMetrics.data as any)?.map((service: any) => (
                    <option key={service.name} value={service.name}>
                      {service.name}
                    </option>
                  ))}
                </select>
              </div>
              <div className="flex items-center gap-2">
                <label className="text-sm font-medium text-light-300">Status:</label>
                <select className="bg-dark-200 border border-dark-400 rounded px-3 py-1 text-sm text-light-200">
                  <option>All Status</option>
                  <option>Healthy</option>
                  <option>Warning</option>
                  <option>Error</option>
                </select>
              </div>
              <div className="flex items-center gap-2">
                <label className="text-sm font-medium text-light-300">Time Range:</label>
                <select className="bg-dark-200 border border-dark-400 rounded px-3 py-1 text-sm text-light-200">
                  <option>Last 15 minutes</option>
                  <option>Last 1 hour</option>
                  <option>Last 6 hours</option>
                  <option>Last 24 hours</option>
                </select>
              </div>
            </div>
          </motion.div>
        )}

        {/* Notifications Dropdown */}
        <div className="relative">
          <Dropdown isOpen={showNotifications} onClose={() => toggleNotifications()}>
            <div className="p-4">
              <h3 className="text-sm font-semibold text-light-200 mb-3">Recent Notifications</h3>
              <div className="space-y-3">
                <div className="flex items-start gap-3 p-3 bg-dark-200 rounded-lg">
                  <StatusIndicator status="error" />
                  <div>
                    <p className="text-sm text-light-200">High error rate detected</p>
                    <p className="text-xs text-light-500">payment-service - 5 minutes ago</p>
                  </div>
                </div>
                <div className="flex items-start gap-3 p-3 bg-dark-200 rounded-lg">
                  <StatusIndicator status="warning" />
                  <div>
                    <p className="text-sm text-light-200">Latency spike observed</p>
                    <p className="text-xs text-light-500">auth-service - 12 minutes ago</p>
                  </div>
                </div>
                <div className="flex items-start gap-3 p-3 bg-dark-200 rounded-lg">
                  <StatusIndicator status="online" />
                  <div>
                    <p className="text-sm text-light-200">Service recovered</p>
                    <p className="text-xs text-light-500">notification-service - 1 hour ago</p>
                  </div>
                </div>
              </div>
            </div>
          </Dropdown>
        </div>

        {/* Settings Dropdown */}
        <div className="relative">
          <Dropdown isOpen={showSettings} onClose={() => toggleSettings()} className="w-64">
            <div className="p-2">
              <DropdownItem>Theme Settings</DropdownItem>
              <DropdownItem>Data Refresh Rate</DropdownItem>
              <DropdownItem>Export Settings</DropdownItem>
              <DropdownItem>Keyboard Shortcuts</DropdownItem>
              <hr className="border-dark-300 my-2" />
              <DropdownItem>About URPO</DropdownItem>
            </div>
          </Dropdown>
        </div>

        {/* User Menu Dropdown */}
        <div className="relative">
          <Dropdown isOpen={showUserMenu} onClose={() => toggleUserMenu()} className="w-56">
            <div className="p-4">
              <div className="flex items-center gap-3 mb-4 pb-3 border-b border-dark-300">
                <div className="w-10 h-10 bg-gradient-to-br from-data-purple to-data-pink rounded-full flex items-center justify-center text-white font-semibold">
                  U
                </div>
                <div>
                  <p className="text-sm font-medium text-light-200">Admin User</p>
                  <p className="text-xs text-light-500">admin@urpo.dev</p>
                </div>
              </div>
              <div className="space-y-1">
                <DropdownItem>Profile Settings</DropdownItem>
                <DropdownItem>API Keys</DropdownItem>
                <DropdownItem>Preferences</DropdownItem>
                <hr className="border-dark-300 my-2" />
                <DropdownItem>Help & Support</DropdownItem>
                <DropdownItem variant="danger">Sign Out</DropdownItem>
              </div>
            </div>
          </Dropdown>
        </div>

        {/* Error Banner */}
        {hasError && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
            className="px-6 py-3 bg-semantic-error bg-opacity-10 border-b border-semantic-error border-opacity-30"
          >
            <div className="flex items-center gap-2">
              <StatusIndicator status="error" />
              <span className="text-sm text-semantic-error">
                Connection error - check OTEL receiver status
              </span>
            </div>
          </motion.div>
        )}

        {/* Main Content Area with polished styling */}
        <main className="flex-1 overflow-hidden bg-dark-0 relative">
          {/* Subtle background pattern */}
          <div className="absolute inset-0 opacity-[0.02]" style={{
            backgroundImage: `
              linear-gradient(rgba(255,255,255,0.03) 1px, transparent 1px),
              linear-gradient(90deg, rgba(255,255,255,0.03) 1px, transparent 1px)
            `,
            backgroundSize: '50px 50px'
          }}></div>

          <div className="relative h-full">
            {activeView === 'graph' && (
              <ErrorBoundary componentName="ServiceGraphPro" isolate>
                <Page>
                  <Section
                    title="Service Dependency Map"
                    subtitle="Real-time service interactions and health status"
                    action={
                      <div className="flex gap-2">
                        <Button variant="primary" size="sm">Auto Layout</Button>
                        <Button variant="secondary" size="sm">Export</Button>
                      </div>
                    }
                  >
                    <ServiceGraphPro
                      services={(serviceMetrics.data as any) || []}
                      traces={(recentTraces.data as any) || []}
                    />
                  </Section>
                </Page>
              </ErrorBoundary>
            )}

            {activeView === 'flows' && (
              <ErrorBoundary componentName="FlowTable" isolate>
                <Page className="p-4">
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
                </Page>
              </ErrorBoundary>
            )}

            {activeView === 'health' && (
              <ErrorBoundary componentName="HealthView" isolate>
                <Page className="space-y-6">
                  <ServiceHealthDashboard services={(serviceMetrics.data as any) || []} />
                  {systemMetrics.data && (
                    <SystemMetrics metrics={(systemMetrics.data as any)} />
                  )}
                </Page>
              </ErrorBoundary>
            )}

            {activeView === 'traces' && (
              <ErrorBoundary componentName="TraceExplorer" isolate>
                <Page className="p-4">
                  <TraceExplorer
                    traces={(recentTraces.data as any) || []}
                    onRefresh={recentTraces.refetch}
                  />
                </Page>
              </ErrorBoundary>
            )}

            {activeView === 'servicemap' && (
              <ErrorBoundary componentName="ServiceMap" isolate>
                <Page>
                  <ServiceMap />
                </Page>
              </ErrorBoundary>
            )}
          </div>
        </main>

        {/* Status Bar */}
        <footer className="bg-dark-100 border-t border-dark-300 px-6 py-2">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4 text-xs">
              <span className="text-light-500">
                © 2025 URPO • Ultra-Fast OTEL Explorer
              </span>
              <Badge variant="info" size="sm">v0.1.0</Badge>
            </div>

            <div className="flex items-center gap-4">
              {systemMetrics.data && (
                <>
                  <Metric
                    label="Total Spans"
                    value={(systemMetrics.data as any)?.total_spans?.toLocaleString() || '0'}
                    color="green"
                  />
                  <Metric
                    label="Uptime"
                    value={`${Math.floor(((systemMetrics.data as any)?.uptime_seconds || 0) / 60)}m`}
                    color="cyan"
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