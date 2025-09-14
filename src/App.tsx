import { useState, useEffect, useCallback, memo } from 'react';
import { useKeyboardShortcuts } from './hooks/useKeyboardShortcuts';
import { useLocalStorage } from './hooks/useLocalStorage';
import { ErrorBoundary } from './components/common/ErrorBoundary';
import { ServiceHealthDashboard } from './components/tables/ServiceHealthDashboard';
import { TraceExplorer } from './components/tables/TraceExplorer';
import { SystemMetrics } from './components/panels/SystemMetrics';
import { ServiceGraph } from './components/charts/ServiceGraph';
import { ServiceMap } from './components/tables/ServiceMap';
import { FlowTable } from './components/tables/FlowTable';
import { VirtualizedFlowTable } from './components/tables/VirtualizedFlowTable';
import { ServiceMetrics, TraceInfo, SystemMetrics as SystemMetricsType, ViewMode, NavigationItem } from './types';
import { Network, Activity, BarChart3, Layers, GitBranch, Share2 } from 'lucide-react';
import { isTauriAvailable, safeTauriInvoke } from './utils/tauri';
import { 
  getUpdatedMockServices, 
  getUpdatedMockSystemMetrics, 
  mockTraces 
} from './services/mockData';
import { POLLING } from './constants/ui';

// PERFORMANCE: Memoize the entire app to prevent unnecessary re-renders
const App = memo(() => {
  const [activeView, setActiveView] = useLocalStorage<ViewMode>('urpo-active-view', 'graph');
  const [services, setServices] = useState<ServiceMetrics[]>([]);
  const [traces, setTraces] = useState<TraceInfo[]>([]);
  const [systemMetrics, setSystemMetrics] = useState<SystemMetricsType | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Add keyboard shortcuts
  useKeyboardShortcuts([
    { key: '1', handler: () => setActiveView('graph'), description: 'Switch to graph view' },
    { key: '2', handler: () => setActiveView('flows'), description: 'Switch to flows view' },
    { key: '3', handler: () => setActiveView('health'), description: 'Switch to health view' },
    { key: '4', handler: () => setActiveView('traces'), description: 'Switch to traces view' },
    { key: '5', handler: () => setActiveView('servicemap'), description: 'Switch to service map view' },
    { key: 'r', handler: updateMetrics, description: 'Refresh metrics', ctrl: true },
    { key: 't', handler: loadTraces, description: 'Reload traces', ctrl: true },
  ]);

  // PERFORMANCE: Use requestAnimationFrame for smooth 60fps updates
  const updateMetrics = useCallback(async () => {
    try {
      if (isTauriAvailable()) {
        // Use real Tauri backend when available
        const [serviceData, systemData] = await Promise.all([
          safeTauriInvoke<ServiceMetrics[]>('get_service_metrics'),
          safeTauriInvoke<SystemMetricsType>('get_system_metrics'),
        ]);

        if (serviceData && systemData) {
          // Batch state updates for better performance
          requestAnimationFrame(() => {
            setServices(serviceData);
            setSystemMetrics(systemData);
            setError(null);
          });
        }
      } else {
        // Use mock data when Tauri is not available
        const serviceData = getUpdatedMockServices();
        const systemData = getUpdatedMockSystemMetrics();

        requestAnimationFrame(() => {
          setServices(serviceData);
          setSystemMetrics(systemData);
          setError(null);
        });
      }
    } catch (err) {
      console.error('Error updating metrics:', err);
      // Fallback to mock data on error
      const serviceData = getUpdatedMockServices();
      const systemData = getUpdatedMockSystemMetrics();

      requestAnimationFrame(() => {
        setServices(serviceData);
        setSystemMetrics(systemData);
        setError(`Backend unavailable - showing demo data`);
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
        // Use mock trace data when Tauri is not available
        requestAnimationFrame(() => {
          setTraces(mockTraces);
          setError(null);
        });
      }
    } catch (err) {
      console.error('Error loading traces:', err);
      // Fallback to mock data
      requestAnimationFrame(() => {
        setTraces(mockTraces);
        setError(`Backend unavailable - showing demo traces`);
      });
    }
  }, []);

  // Start OTEL receiver on mount
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
        // Running in web mode - skip backend initialization
        console.log('Running in web mode with demo data');
        setLoading(false);
        setError(null);
      }
    };

    startReceiver();

    // Cleanup on unmount
    return () => {
      if (isTauriAvailable()) {
        safeTauriInvoke('stop_receiver').catch(console.error);
      }
    };
  }, []);

  // Poll for metrics - updates efficiently
  useEffect(() => {
    if (loading) return;

    // Initial load
    updateMetrics();

    // Poll for real-time updates using configured interval
    const interval = setInterval(updateMetrics, POLLING.METRICS_INTERVAL_MS);

    return () => clearInterval(interval);
  }, [loading, updateMetrics]);

  // Load traces when switching to trace view
  useEffect(() => {
    if (activeView === 'traces' && !loading) {
      loadTraces();
    }
  }, [activeView, loading, loadTraces]);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-screen bg-surface-50">
        <div className="clean-card p-8 text-center ">
          <div className="w-16 h-16 mx-auto mb-4 rounded-lg flex items-center justify-center bg-surface-200">
            <Network className="w-8 h-8 text-text-700" />
          </div>
          <p className="text-text-900 font-medium mb-2">Starting URPO</p>
          <p className="text-text-500 text-xs font-mono">Ultra-Fast OTEL Explorer</p>
          <div className="mt-4 h-1 bg-surface-200 rounded-full overflow-hidden">
            <div className="h-full bg-status-healthy  w-3/4"></div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <ErrorBoundary componentName="App">
      <div className="h-screen bg-surface-50 text-text-900 flex flex-col gpu-composite">
      {/* Clean Professional Header */}
      <header className="clean-card border-0 border-b border-surface-300 px-6 py-3 gpu-layer rounded-none">
        <div className="flex items-center justify-between">
          {/* Brand Section */}
          <div className="flex items-center space-x-6">
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-lg bg-surface-200">
                <Network className="w-5 h-5 text-text-700" />
              </div>
              <div>
                <h1 className="text-lg font-display font-bold text-text-900 tracking-tight">
                  URPO
                </h1>
                <div className="text-[10px] text-text-500 font-mono uppercase tracking-wide">
                  Ultra-Fast OTEL Explorer
                </div>
              </div>
            </div>
            
            <div className="hidden md:block h-6 w-0.5 bg-surface-400"></div>
            
            <div className="hidden md:flex items-center gap-2 text-xs text-text-500 font-mono">
              <div className={`status-indicator  ${isTauriAvailable() ? 'healthy' : 'warning'}`}></div>
              <span>{isTauriAvailable() ? 'Collector Active' : 'Demo Mode'}</span>
            </div>
          </div>
          
          {/* Sharp Navigation */}
          <nav className="flex items-center gap-1">
            {([
              { key: 'graph', icon: GitBranch, label: 'Service Map', shortcut: '⌘1' },
              { key: 'flows', icon: Activity, label: 'Trace Flows', shortcut: '⌘2' },
              { key: 'health', icon: BarChart3, label: 'Metrics', shortcut: '⌘3' },
              { key: 'traces', icon: Layers, label: 'Traces', shortcut: '⌘4' },
              { key: 'servicemap', icon: Share2, label: 'Dependencies', shortcut: '⌘5' },
            ] as NavigationItem[]).map(({ key, icon: Icon, label, shortcut }) => (
              <button
                key={key}
                onClick={() => setActiveView(key)}
                className={`clean-button px-3 py-2 rounded-lg flex items-center gap-2 text-xs font-medium micro-interaction ${
                  activeView === key
                    ? 'active'
                    : ''
                }`}
              >
                <Icon className="w-3.5 h-3.5" />
                <span className="hidden lg:inline">{label}</span>
                <span className="hidden xl:inline text-[10px] text-text-300 font-mono">{shortcut}</span>
              </button>
            ))}
          </nav>

          {/* Live Metrics Display */}
          {systemMetrics && (
            <div className="clean-card px-3 py-1.5 flex items-center gap-4">
              <div className="flex items-center gap-1.5 text-[10px] font-mono">
                <div className="w-1.5 h-1.5 bg-status-healthy rounded-full "></div>
                <span className="text-text-500">MEM</span>
                <span className="text-text-900 font-medium">
                  {systemMetrics.memory_usage_mb.toFixed(0)}MB
                </span>
              </div>
              
              <div className="w-0.5 h-3 bg-surface-400"></div>
              
              <div className="flex items-center gap-1.5 text-[10px] font-mono">
                <div className="w-1.5 h-1.5 bg-status-warning rounded-full "></div>
                <span className="text-text-500">CPU</span>
                <span className="text-text-900 font-medium">
                  {systemMetrics.cpu_usage_percent.toFixed(1)}%
                </span>
              </div>
              
              <div className="w-0.5 h-3 bg-surface-400"></div>
              
              <div className="flex items-center gap-1.5 text-[10px] font-mono">
                <div className="w-1.5 h-1.5 bg-text-700 rounded-full "></div>
                <span className="text-text-500">RPS</span>
                <span className="text-text-900 font-medium">
                  {systemMetrics.spans_per_second.toFixed(0)}
                </span>
              </div>
            </div>
          )}
        </div>
      </header>

      {/* Clean Error Display */}
      {error && (
        <div className="mx-6 mt-4 ">
          <div className="clean-card border-status-error bg-status-error bg-opacity-5 p-4">
            <div className="flex items-center gap-3">
              <div className="status-indicator critical"></div>
              <div>
                <div className="text-status-error font-medium text-sm">System Notice</div>
                <div className="text-text-500 text-xs font-mono mt-1">{error}</div>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Ultra-Sharp Main Content */}
      <main className="flex-1 overflow-hidden gpu-layer">
        {activeView === 'graph' && (
          <ErrorBoundary componentName="ServiceGraph" isolate>
            <div className="h-full p-6">
              <ServiceGraph services={services} traces={traces} />
            </div>
          </ErrorBoundary>
        )}
        
        {activeView === 'flows' && (
          <ErrorBoundary componentName="FlowTable" isolate>
            <div className="h-full">
              {/* Use virtualized table for large datasets */}
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
            <div className="p-6 space-y-6">
              <ErrorBoundary componentName="ServiceHealthDashboard" isolate>
                <ServiceHealthDashboard services={services} />
              </ErrorBoundary>
              {systemMetrics && (
                <ErrorBoundary componentName="SystemMetrics" isolate>
                  <SystemMetrics metrics={systemMetrics} />
                </ErrorBoundary>
              )}
            </div>
          </ErrorBoundary>
        )}
        
        {activeView === 'traces' && (
          <ErrorBoundary componentName="TraceExplorer" isolate>
            <div className="p-6">
              <TraceExplorer 
                traces={traces} 
                onRefresh={loadTraces}
              />
            </div>
          </ErrorBoundary>
        )}
        
        {activeView === 'servicemap' && (
          <ErrorBoundary componentName="ServiceMap" isolate>
            <div className="h-full">
              <ServiceMap />
            </div>
          </ErrorBoundary>
        )}
      </main>

      {/* Clean Status Bar */}
      <footer className="clean-card border-0 border-t border-surface-300 px-6 py-2 gpu-layer rounded-none">
        <div className="flex justify-between items-center">
          <div className="flex items-center gap-6 text-[10px] font-mono">
            <div className="flex items-center gap-2">
              <div className={`status-indicator  ${isTauriAvailable() ? 'healthy' : 'warning'}`}></div>
              <span className="text-text-500">{isTauriAvailable() ? 'OTEL Collector' : 'Demo Mode'}</span>
              <span className={isTauriAvailable() ? 'text-status-healthy' : 'text-status-warning'}>
                {isTauriAvailable() ? 'ACTIVE' : 'OFFLINE'}
              </span>
            </div>
            
            <div className="flex items-center gap-4 text-text-300">
              <span className="flex items-center gap-1">
                <span className="text-text-500">SERVICES</span>
                <span className="text-text-900 font-medium">{services.length}</span>
              </span>
              <span className="flex items-center gap-1">
                <span className="text-text-500">TRACES</span>
                <span className="text-text-900 font-medium">{traces.length}</span>
              </span>
              <span className="flex items-center gap-1">
                <span className="text-text-500">SPANS</span>
                <span className="text-text-900 font-medium">
                  {(systemMetrics?.total_spans || 0).toLocaleString()}
                </span>
              </span>
            </div>
            
            {systemMetrics && (
              <div className="flex items-center gap-1">
                <span className="text-text-500">THROUGHPUT</span>
                <span className="text-text-900 font-medium">
                  {systemMetrics.spans_per_second.toFixed(0)} spans/s
                </span>
              </div>
            )}
          </div>
          
          <div className="flex items-center gap-4 text-[10px] text-text-300 font-mono">
            <div className="flex items-center gap-2">
              <span>Powered by</span>
              <span className="text-text-900 font-medium">Urpo</span>
            </div>
            <div className="w-0.5 h-3 bg-surface-400"></div>
            <div className="flex items-center gap-1">
              <Network className="w-3 h-3 text-text-700" />
              <span className="text-text-700 font-medium">Ultra-Fast OTEL</span>
            </div>
          </div>
        </div>
      </footer>
      </div>
    </ErrorBoundary>
  );
});

App.displayName = 'App';

export default App;