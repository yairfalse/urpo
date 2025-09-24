import { useState, useEffect, useCallback, memo } from 'react';
import { useKeyboardShortcuts } from './hooks/useKeyboardShortcuts';
import { useLocalStorage } from './hooks/useLocalStorage';
import { ErrorBoundary } from './components/common/ErrorBoundary';
import { ServiceHealthDashboard } from './components/tables/ServiceHealthDashboard';
import { TraceExplorer } from './components/tables/TraceExplorer';
import { SystemMetrics } from './components/panels/SystemMetrics';
import { ServiceGraphPro } from './components/charts/ServiceGraphPro';
import { ServiceMap } from './components/tables/ServiceMap';
import { FlowTable } from './components/tables/FlowTable';
import { VirtualizedFlowTable } from './components/tables/VirtualizedFlowTable';
import { ServiceMetrics, TraceInfo, SystemMetrics as SystemMetricsType, ViewMode, NavigationItem } from './types';
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
  ChevronDown
} from 'lucide-react';
import { isTauriAvailable, safeTauriInvoke } from './utils/tauri';
import { POLLING } from './constants/ui';

const App = memo(() => {
  const [activeView, setActiveView] = useLocalStorage<ViewMode>('urpo-active-view', 'graph');
  const [services, setServices] = useState<ServiceMetrics[]>([]);
  const [traces, setTraces] = useState<TraceInfo[]>([]);
  const [systemMetrics, setSystemMetrics] = useState<SystemMetricsType | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [showFilters, setShowFilters] = useState(false);
  const [showNotifications, setShowNotifications] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [showUserMenu, setShowUserMenu] = useState(false);

  const updateMetrics = useCallback(async () => {
    try {
      if (isTauriAvailable()) {
        const [serviceData, systemData] = await Promise.all([
          safeTauriInvoke<ServiceMetrics[]>('get_service_metrics'),
          safeTauriInvoke<SystemMetricsType>('get_system_metrics'),
        ]);

        if (serviceData && systemData) {
          requestAnimationFrame(() => {
            setServices(serviceData);
            setSystemMetrics(systemData);
            setError(null);
          });
        }
      } else {
        requestAnimationFrame(() => {
          setServices([]);
          setSystemMetrics(null);
          setError('Backend not available - ensure OTEL receiver is running');
        });
      }
    } catch (err) {
      console.error('Error updating metrics:', err);
      requestAnimationFrame(() => {
        setServices([]);
        setSystemMetrics(null);
        setError(`Failed to connect to backend`);
      });
    }
  }, []);

  const loadTraces = useCallback(async () => {
    try {
      if (isTauriAvailable()) {
        const traceData = await safeTauriInvoke<TraceInfo[]>('list_recent_traces', {
          limit: 100,
        });

        if (traceData) {
          requestAnimationFrame(() => {
            setTraces(traceData);
            setError(null);
          });
        }
      } else {
        requestAnimationFrame(() => {
          setTraces([]);
          setError('Backend not available');
        });
      }
    } catch (err) {
      console.error('Error loading traces:', err);
      requestAnimationFrame(() => {
        setTraces([]);
        setError(`Failed to load traces`);
      });
    }
  }, []);

  useKeyboardShortcuts([
    { key: '1', handler: () => setActiveView('graph'), description: 'Service Map' },
    { key: '2', handler: () => setActiveView('flows'), description: 'Trace Flows' },
    { key: '3', handler: () => setActiveView('health'), description: 'Health Metrics' },
    { key: '4', handler: () => setActiveView('traces'), description: 'Traces' },
    { key: '5', handler: () => setActiveView('servicemap'), description: 'Dependencies' },
    { key: 'r', handler: updateMetrics, description: 'Refresh', ctrl: true },
    { key: 't', handler: loadTraces, description: 'Reload traces', ctrl: true },
    { key: '/', handler: () => {
      document.getElementById('global-search')?.focus();
      // Close any open dropdowns when focusing search
      setShowFilters(false);
      setShowNotifications(false);
      setShowSettings(false);
      setShowUserMenu(false);
    }, description: 'Search' },
  ]);

  useEffect(() => {
    const startReceiver = async () => {
      if (isTauriAvailable()) {
        try {
          await safeTauriInvoke('start_receiver');
          setLoading(false);
          setError(null);
        } catch (err) {
          setError(`Failed to start OTEL receiver: ${err}`);
          setLoading(false);
        }
      } else {
        setLoading(false);
        setError(null);
      }
    };

    startReceiver();

    return () => {
      if (isTauriAvailable()) {
        safeTauriInvoke('stop_receiver').catch(console.error);
      }
    };
  }, []);

  useEffect(() => {
    if (loading) return;
    updateMetrics();
    const interval = setInterval(updateMetrics, POLLING.METRICS_INTERVAL_MS);
    return () => clearInterval(interval);
  }, [loading, updateMetrics]);

  useEffect(() => {
    if (activeView === 'traces' && !loading) {
      loadTraces();
    }
  }, [activeView, loading, loadTraces]);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-screen bg-dark-50">
        <div className="text-center">
          <div className="w-12 h-12 mx-auto mb-4 rounded-full border-2 border-data-blue border-t-transparent animate-spin"></div>
          <h2 className="text-light-100 font-semibold text-lg mb-2">Initializing URPO</h2>
          <p className="text-light-400 text-sm">Starting observability engine...</p>
        </div>
      </div>
    );
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
        {/* Ultra-polished header */}
        <header className="sharp-panel bg-dark-50 border-b shadow-lg relative overflow-hidden">
          {/* Gradient accent line */}
          <div className="absolute top-0 left-0 right-0 h-0.5 bg-gradient-to-r from-transparent via-data-blue to-transparent opacity-80"></div>

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
                    <button
                      key={key}
                      onClick={() => setActiveView(key)}
                      className={`nav-item-sharp ${activeView === key ? 'active' : ''}`}
                    >
                      <Icon className="w-4 h-4" />
                      <span className="font-medium">{label}</span>
                      <kbd className="hidden lg:inline-block badge-sharp bg-dark-300 text-light-500 text-[10px] px-2 py-0.5">
                        {shortcut}
                      </kbd>
                    </button>
                  ))}
                </nav>
              </div>

              {/* Actions and Status */}
              <div className="flex items-center gap-6">
                {/* Professional Search */}
                <div className="relative group">
                  <div className="absolute inset-0 bg-gradient-to-r from-data-blue/10 to-data-purple/10 rounded-lg blur opacity-0 group-hover:opacity-100 transition-opacity"></div>
                  <div className="relative">
                    <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-light-500" />
                    <input
                      id="global-search"
                      type="text"
                      value={searchQuery}
                      onChange={(e) => setSearchQuery(e.target.value)}
                      placeholder="Search traces, services..."
                      className="input-sharp pl-10 pr-16 w-80"
                    />
                    <div className="absolute right-3 top-1/2 -translate-y-1/2">
                      <kbd className="badge-sharp text-[10px] px-2 py-1">⌘K</kbd>
                    </div>
                  </div>
                </div>

                {/* Status indicators */}
                <div className="flex items-center gap-3 px-4 py-2 sharp-panel bg-dark-100">
                  <div className="flex items-center gap-2">
                    <div className="status-sharp online"></div>
                    <span className="text-xs font-medium text-light-300">Live</span>
                  </div>
                  <div className="w-px h-4 bg-dark-300"></div>
                  <div className="text-xs text-light-400">
                    {services.length} services
                  </div>
                </div>

                {/* Action Buttons */}
                <div className="flex items-center gap-2">
                  <button
                    onClick={() => setShowFilters(!showFilters)}
                    className={`btn-ghost p-2 ${showFilters ? 'bg-dark-200 text-data-blue' : ''}`}
                    title="Toggle Filters"
                  >

                    <Filter className="w-4 h-4" />
                  </button>
                  <button
                    onClick={updateMetrics}
                    className="btn-ghost p-2"
                    title="Refresh Data"
                  >
                    <RefreshCw className="w-4 h-4" />
                  </button>
                  <button
                    onClick={() => setShowNotifications(!showNotifications)}
                    className={`btn-ghost p-2 relative ${showNotifications ? 'bg-dark-200 text-data-blue' : ''}`}
                    title="Notifications"
                  >
                    <Bell className="w-4 h-4" />
                    <span className="absolute -top-1 -right-1 w-2 h-2 bg-semantic-error rounded-full"></span>
                  </button>
                  <button
                    onClick={() => setShowSettings(!showSettings)}
                    className={`btn-ghost p-2 ${showSettings ? 'bg-dark-200 text-data-blue' : ''}`}
                    title="Settings"
                  >

                    <Settings className="w-4 h-4" />
                  </button>
                </div>

                {/* User Menu */}
                <button
                  onClick={() => setShowUserMenu(!showUserMenu)}
                  className="flex items-center gap-2 pl-4 border-l border-dark-300 hover:bg-dark-200 rounded-r-lg px-2 py-1 transition-colors"
                >
                  <div className="w-8 h-8 bg-gradient-to-br from-data-purple to-data-pink rounded-full flex items-center justify-center text-white font-semibold text-sm">
                    U
                  </div>
                  <ChevronDown className={`w-4 h-4 text-light-500 transition-transform ${showUserMenu ? 'rotate-180' : ''}`} />
                </button>

              </div>
            </div>
          </div>

          {/* Sub-header with Metrics */}
          {systemMetrics && (
            <div className="px-4 py-2 bg-dark-150 border-t border-dark-300">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-6">
                  {/* Connection Status */}
                  <div className="flex items-center gap-2">
                    <div className={`w-2 h-2 rounded-full ${
                      isTauriAvailable() ? 'bg-semantic-success' : 'bg-semantic-warning'
                    } animate-pulse`}></div>
                    <span className="text-xs text-light-400">
                      {isTauriAvailable() ? 'OTLP Connected' : 'Demo Mode'}
                    </span>
                  </div>

                  {/* Key Metrics Bar */}
                  <div className="flex items-center gap-4">
                    <div className="flex items-center gap-2">
                      <span className="text-xs text-light-500">Services</span>
                      <span className="text-sm font-medium text-light-200">{services.length}</span>
                    </div>
                    <div className="flex items-center gap-2">
                      <span className="text-xs text-light-500">Traces</span>
                      <span className="text-sm font-medium text-light-200">{traces.length}</span>
                    </div>
                    <div className="flex items-center gap-2">
                      <span className="text-xs text-light-500">Spans/s</span>
                      <span className="text-sm font-medium text-data-cyan">
                        {systemMetrics.spans_per_second.toFixed(0)}
                      </span>
                    </div>
                    <div className="flex items-center gap-2">
                      <span className="text-xs text-light-500">Memory</span>
                      <span className="text-sm font-medium text-data-yellow">
                        {systemMetrics.memory_usage_mb.toFixed(0)}MB
                      </span>
                    </div>
                    <div className="flex items-center gap-2">
                      <span className="text-xs text-light-500">CPU</span>
                      <span className="text-sm font-medium text-data-orange">
                        {systemMetrics.cpu_usage_percent.toFixed(1)}%
                      </span>
                    </div>
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
            </div>
          )}
        </header>

        {/* Filter Panel */}
        {showFilters && (
          <div className="bg-dark-100 border-b border-dark-300 px-4 py-3">
            <div className="flex items-center gap-4">
              <div className="flex items-center gap-2">
                <label className="text-sm font-medium text-light-300">Service:</label>
                <select className="bg-dark-200 border border-dark-400 rounded px-3 py-1 text-sm text-light-200">
                  <option>All Services</option>
                  {services.map(service => (
                    <option key={service.service_name} value={service.service_name}>
                      {service.service_name}
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
          </div>
        )}

        {/* Notifications Dropdown */}
        {showNotifications && (
          <div className="absolute top-16 right-4 w-80 bg-dark-100 border border-dark-300 rounded-lg shadow-xl z-50">
            <div className="p-4">
              <h3 className="text-sm font-semibold text-light-200 mb-3">Recent Notifications</h3>
              <div className="space-y-3">
                <div className="flex items-start gap-3 p-3 bg-dark-200 rounded-lg">
                  <div className="w-2 h-2 bg-semantic-error rounded-full mt-2 flex-shrink-0"></div>
                  <div>
                    <p className="text-sm text-light-200">High error rate detected</p>
                    <p className="text-xs text-light-500">payment-service - 5 minutes ago</p>
                  </div>
                </div>
                <div className="flex items-start gap-3 p-3 bg-dark-200 rounded-lg">
                  <div className="w-2 h-2 bg-semantic-warning rounded-full mt-2 flex-shrink-0"></div>
                  <div>
                    <p className="text-sm text-light-200">Latency spike observed</p>
                    <p className="text-xs text-light-500">auth-service - 12 minutes ago</p>
                  </div>
                </div>
                <div className="flex items-start gap-3 p-3 bg-dark-200 rounded-lg">
                  <div className="w-2 h-2 bg-semantic-success rounded-full mt-2 flex-shrink-0"></div>
                  <div>
                    <p className="text-sm text-light-200">Service recovered</p>
                    <p className="text-xs text-light-500">notification-service - 1 hour ago</p>
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Settings Dropdown */}
        {showSettings && (
          <div className="absolute top-16 right-4 w-64 bg-dark-100 border border-dark-300 rounded-lg shadow-xl z-50">
            <div className="p-4">
              <h3 className="text-sm font-semibold text-light-200 mb-3">Settings</h3>
              <div className="space-y-2">
                <button className="w-full text-left px-3 py-2 text-sm text-light-300 hover:bg-dark-200 rounded">
                  Theme Settings
                </button>
                <button className="w-full text-left px-3 py-2 text-sm text-light-300 hover:bg-dark-200 rounded">
                  Data Refresh Rate
                </button>
                <button className="w-full text-left px-3 py-2 text-sm text-light-300 hover:bg-dark-200 rounded">
                  Export Settings
                </button>
                <button className="w-full text-left px-3 py-2 text-sm text-light-300 hover:bg-dark-200 rounded">
                  Keyboard Shortcuts
                </button>
                <hr className="border-dark-300 my-2" />
                <button className="w-full text-left px-3 py-2 text-sm text-light-300 hover:bg-dark-200 rounded">
                  About URPO
                </button>
              </div>
            </div>
          </div>
        )}

        {/* User Menu Dropdown */}
        {showUserMenu && (
          <div className="absolute top-16 right-4 w-56 bg-dark-100 border border-dark-300 rounded-lg shadow-xl z-50">
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
              <div className="space-y-2">
                <button className="w-full text-left px-3 py-2 text-sm text-light-300 hover:bg-dark-200 rounded">
                  Profile Settings
                </button>
                <button className="w-full text-left px-3 py-2 text-sm text-light-300 hover:bg-dark-200 rounded">
                  API Keys
                </button>
                <button className="w-full text-left px-3 py-2 text-sm text-light-300 hover:bg-dark-200 rounded">
                  Preferences
                </button>
                <hr className="border-dark-300 my-2" />
                <button className="w-full text-left px-3 py-2 text-sm text-light-300 hover:bg-dark-200 rounded">
                  Help & Support
                </button>
                <button className="w-full text-left px-3 py-2 text-sm text-semantic-error hover:bg-semantic-error hover:bg-opacity-10 rounded">
                  Sign Out
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Error Banner */}
        {error && (
          <div className="px-4 py-2 bg-semantic-error bg-opacity-10 border-b border-semantic-error border-opacity-30">
            <div className="flex items-center gap-2">
              <div className="w-2 h-2 bg-semantic-error rounded-full"></div>
              <span className="text-sm text-semantic-error">{error}</span>
            </div>
          </div>
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
                <div className="h-full p-6">
                  <div className="h-full sharp-panel bg-dark-50 p-6">
                    <div className="flex items-center justify-between mb-6">
                      <div>
                        <h2 className="text-xl font-bold text-light-50 mb-1">Service Dependency Map</h2>
                        <p className="text-sm text-light-400">Real-time service interactions and health status</p>
                      </div>
                      <div className="flex items-center gap-2">
                        <button className="btn-sharp-primary text-sm px-4 py-2">
                          Auto Layout
                        </button>
                        <button className="btn-sharp text-sm px-4 py-2">
                          Export
                        </button>
                      </div>
                    </div>
                    <div className="h-[calc(100%-100px)] sharp-card p-4">
                      <ServiceGraphPro services={services} traces={traces} />
                    </div>
                  </div>
                </div>
              </ErrorBoundary>
            )}

            {activeView === 'flows' && (
              <ErrorBoundary componentName="FlowTable" isolate>
                <div className="h-full p-4">
                  {traces.length > 100 ? (
                    <VirtualizedFlowTable traces={traces} onRefresh={loadTraces} />
                  ) : (
                    <FlowTable traces={traces} onRefresh={loadTraces} />
                  )}
                </div>
              </ErrorBoundary>
            )}

            {activeView === 'health' && (
              <ErrorBoundary componentName="HealthView" isolate>
                <div className="p-4 dashboard-grid">
                  <div className="panel-full">
                    <ServiceHealthDashboard services={services} />
                  </div>
                  {systemMetrics && (
                    <div className="panel-full">
                      <SystemMetrics metrics={systemMetrics} />
                    </div>
                  )}
                </div>
              </ErrorBoundary>
            )}

            {activeView === 'traces' && (
              <ErrorBoundary componentName="TraceExplorer" isolate>
                <div className="h-full p-4">
                  <TraceExplorer
                    traces={traces}
                    onRefresh={loadTraces}
                  />
                </div>
              </ErrorBoundary>
            )}

            {activeView === 'servicemap' && (
              <ErrorBoundary componentName="ServiceMap" isolate>
                <div className="h-full p-4 bg-dark-50">
                  <ServiceMap />
                </div>
              </ErrorBoundary>
            )}
          </div>
        </main>

        {/* Status Bar */}
        <footer className="bg-dark-100 border-t border-dark-300 px-4 py-2">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4 text-xs">
              <span className="text-light-500">
                © 2025 URPO • Ultra-Fast OTEL Explorer
              </span>
              <span className="text-light-600">
                v0.1.0
              </span>
            </div>

            <div className="flex items-center gap-4 text-xs">
              {systemMetrics && (
                <>
                  <span className="text-light-500">
                    Total Spans: <span className="font-medium text-light-300">
                      {systemMetrics.total_spans.toLocaleString()}
                    </span>
                  </span>
                  <span className="text-light-500">
                    Uptime: <span className="font-medium text-light-300">
                      {Math.floor(systemMetrics.uptime_seconds / 60)}m
                    </span>
                  </span>
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