import { useState, useEffect, useCallback, memo } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import ServiceHealthDashboard from './components/ServiceHealthDashboard';
import TraceExplorer from './components/TraceExplorer';
import SystemMetrics from './components/SystemMetrics';
import ServiceGraph from './components/ServiceGraph';
import FlowTable from './components/FlowTable';
import { ServiceMetrics, TraceInfo, SystemMetrics as SystemMetricsType } from './types';
import { Network, Activity, BarChart3, Layers, GitBranch, Table } from 'lucide-react';

// PERFORMANCE: Memoize the entire app to prevent unnecessary re-renders
const App = memo(() => {
  const [activeView, setActiveView] = useState<'graph' | 'flows' | 'health' | 'traces'>('graph');
  const [selectedTrace, setSelectedTrace] = useState<TraceInfo | null>(null);
  const [services, setServices] = useState<ServiceMetrics[]>([]);
  const [traces, setTraces] = useState<TraceInfo[]>([]);
  const [systemMetrics, setSystemMetrics] = useState<SystemMetricsType | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // PERFORMANCE: Use requestAnimationFrame for smooth 60fps updates
  const updateMetrics = useCallback(async () => {
    try {
      const [serviceData, systemData] = await Promise.all([
        invoke<ServiceMetrics[]>('get_service_metrics'),
        invoke<SystemMetricsType>('get_system_metrics'),
      ]);

      // Batch state updates for better performance
      requestAnimationFrame(() => {
        setServices(serviceData);
        setSystemMetrics(systemData);
        setError(null);
      });
    } catch (err) {
      setError(String(err));
    }
  }, []);

  const loadTraces = useCallback(async () => {
    try {
      const traceData = await invoke<TraceInfo[]>('list_recent_traces', {
        limit: 100,
      });
      
      requestAnimationFrame(() => {
        setTraces(traceData);
        setError(null);
      });
    } catch (err) {
      setError(String(err));
    }
  }, []);

  // Start OTEL receiver on mount
  useEffect(() => {
    const startReceiver = async () => {
      try {
        await invoke('start_receiver');
        setLoading(false);
      } catch (err) {
        setError(`Failed to start receiver: ${err}`);
        setLoading(false);
      }
    };

    startReceiver();

    // Cleanup on unmount
    return () => {
      invoke('stop_receiver').catch(console.error);
    };
  }, []);

  // Poll for metrics - updates efficiently
  useEffect(() => {
    if (loading) return;

    // Initial load
    updateMetrics();

    // Poll every second for real-time updates
    const interval = setInterval(updateMetrics, 1000);

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
      <div className="flex items-center justify-center h-screen bg-void-950">
        <div className="glass-card p-8 text-center animate-scale-in">
          <div className="electric-glow w-16 h-16 mx-auto mb-4 rounded-full flex items-center justify-center animate-pulse-electric">
            <div className="text-electric-blue text-2xl font-mono font-bold">⚡</div>
          </div>
          <p className="text-steel-100 font-medium mb-2">Starting Urpo...</p>
          <p className="text-steel-300 text-xs font-mono">Target: &lt;200ms • Ultra-fast initialization</p>
          <div className="mt-4 h-0.5 bg-steel-800 rounded-full overflow-hidden">
            <div className="h-full bg-electric-blue animate-knife-shine"></div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="h-screen bg-void-950 text-steel-100 flex flex-col gpu-composite">
      {/* Ultra-Sharp Header */}
      <header className="glass-card border-0 border-b-0.5 border-steel-800 backdrop-blur-knife px-6 py-3 gpu-layer">
        <div className="flex items-center justify-between">
          {/* Brand Section */}
          <div className="flex items-center space-x-6">
            <div className="flex items-center gap-3">
              <div className="electric-glow p-2 rounded-lg">
                <Network className="w-5 h-5 text-electric-blue" />
              </div>
              <div>
                <h1 className="text-lg font-display font-bold text-steel-50 tracking-tight">
                  URPO
                </h1>
                <div className="text-[10px] text-steel-300 font-mono uppercase tracking-wide">
                  Ultra-Fast OTEL Explorer
                </div>
              </div>
            </div>
            
            <div className="hidden md:block h-6 w-0.5 bg-steel-700"></div>
            
            <div className="hidden md:flex items-center gap-2 text-xs text-steel-300 font-mono">
              <div className="status-indicator healthy animate-pulse-electric"></div>
              <span>Collector Active</span>
            </div>
          </div>
          
          {/* Sharp Navigation */}
          <nav className="flex items-center gap-1">
            {[
              { key: 'graph', icon: GitBranch, label: 'Service Map', shortcut: '⌘1' },
              { key: 'flows', icon: Activity, label: 'Trace Flows', shortcut: '⌘2' },
              { key: 'health', icon: BarChart3, label: 'Metrics', shortcut: '⌘3' },
              { key: 'traces', icon: Layers, label: 'Traces', shortcut: '⌘4' },
            ].map(({ key, icon: Icon, label, shortcut }) => (
              <button
                key={key}
                onClick={() => setActiveView(key as any)}
                className={`knife-button px-3 py-2 rounded-lg flex items-center gap-2 text-xs font-medium micro-interaction ${
                  activeView === key
                    ? 'electric-glow bg-electric-blue/10 text-electric-blue border-electric-blue'
                    : 'text-steel-300 hover:text-steel-100 border-knife'
                }`}
              >
                <Icon className="w-3.5 h-3.5" />
                <span className="hidden lg:inline">{label}</span>
                <span className="hidden xl:inline text-[10px] text-steel-400 font-mono">{shortcut}</span>
              </button>
            ))}
          </nav>

          {/* Live Metrics Display */}
          {systemMetrics && (
            <div className="glass-card px-3 py-1.5 flex items-center gap-4">
              <div className="flex items-center gap-1.5 text-[10px] font-mono">
                <div className="w-1.5 h-1.5 bg-electric-green rounded-full animate-pulse-electric"></div>
                <span className="text-steel-300">MEM</span>
                <span className="text-steel-100 font-medium">
                  {systemMetrics.memory_usage_mb.toFixed(0)}MB
                </span>
              </div>
              
              <div className="w-0.5 h-3 bg-steel-700"></div>
              
              <div className="flex items-center gap-1.5 text-[10px] font-mono">
                <div className="w-1.5 h-1.5 bg-electric-amber rounded-full animate-pulse-electric"></div>
                <span className="text-steel-300">CPU</span>
                <span className="text-steel-100 font-medium">
                  {systemMetrics.cpu_usage_percent.toFixed(1)}%
                </span>
              </div>
              
              <div className="w-0.5 h-3 bg-steel-700"></div>
              
              <div className="flex items-center gap-1.5 text-[10px] font-mono">
                <div className="w-1.5 h-1.5 bg-electric-blue rounded-full animate-pulse-electric"></div>
                <span className="text-steel-300">RPS</span>
                <span className="text-electric-blue font-medium">
                  {systemMetrics.spans_per_second.toFixed(0)}
                </span>
              </div>
            </div>
          )}
        </div>
      </header>

      {/* Knife-Edge Error Display */}
      {error && (
        <div className="mx-6 mt-4 animate-slide-down">
          <div className="glass-card border-electric-red bg-electric-red/5 p-4">
            <div className="flex items-center gap-3">
              <div className="status-indicator critical"></div>
              <div>
                <div className="text-electric-red font-medium text-sm">System Error</div>
                <div className="text-steel-300 text-xs font-mono mt-1">{error}</div>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Ultra-Sharp Main Content */}
      <main className="flex-1 overflow-hidden gpu-layer">
        {activeView === 'graph' && (
          <div className="h-full p-6 animate-slide-up">
            <ServiceGraph services={services} traces={traces} />
          </div>
        )}
        
        {activeView === 'flows' && (
          <div className="h-full animate-slide-up">
            <FlowTable traces={traces} onRefresh={loadTraces} />
          </div>
        )}
        
        {activeView === 'health' && (
          <div className="p-6 space-y-6 animate-slide-up">
            <ServiceHealthDashboard services={services} />
            {systemMetrics && <SystemMetrics metrics={systemMetrics} />}
          </div>
        )}
        
        {activeView === 'traces' && (
          <div className="p-6 animate-slide-up">
            <TraceExplorer 
              traces={traces} 
              onRefresh={loadTraces}
            />
          </div>
        )}
      </main>

      {/* Razor-Sharp Status Bar */}
      <footer className="glass-card border-0 border-t-0.5 border-steel-800 px-6 py-2 backdrop-blur-knife gpu-layer">
        <div className="flex justify-between items-center">
          <div className="flex items-center gap-6 text-[10px] font-mono">
            <div className="flex items-center gap-2">
              <div className="status-indicator healthy animate-pulse-electric"></div>
              <span className="text-steel-300">OTEL Collector</span>
              <span className="text-electric-green">ACTIVE</span>
            </div>
            
            <div className="flex items-center gap-4 text-steel-400">
              <span className="flex items-center gap-1">
                <span className="text-steel-300">SERVICES</span>
                <span className="text-steel-100 font-medium">{services.length}</span>
              </span>
              <span className="flex items-center gap-1">
                <span className="text-steel-300">TRACES</span>
                <span className="text-steel-100 font-medium">{traces.length}</span>
              </span>
              <span className="flex items-center gap-1">
                <span className="text-steel-300">SPANS</span>
                <span className="text-steel-100 font-medium">
                  {(systemMetrics?.total_spans || 0).toLocaleString()}
                </span>
              </span>
            </div>
            
            {systemMetrics && (
              <div className="flex items-center gap-1">
                <span className="text-steel-300">THROUGHPUT</span>
                <span className="text-electric-blue font-medium">
                  {systemMetrics.spans_per_second.toFixed(0)} spans/s
                </span>
              </div>
            )}
          </div>
          
          <div className="flex items-center gap-4 text-[10px] text-steel-400 font-mono">
            <div className="flex items-center gap-2">
              <span>Powered by</span>
              <span className="text-steel-100 font-medium">Urpo</span>
            </div>
            <div className="w-0.5 h-3 bg-steel-700"></div>
            <div className="flex items-center gap-1">
              <span className="text-electric-blue">⚡</span>
              <span className="text-electric-blue font-medium">Ultra-Fast OTEL</span>
            </div>
          </div>
        </div>
      </footer>
    </div>
  );
});

App.displayName = 'App';

export default App;