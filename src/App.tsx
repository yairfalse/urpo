import { useState, useEffect, useCallback, memo } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import ServiceHealthDashboard from './components/ServiceHealthDashboard';
import TraceExplorer from './components/TraceExplorer';
import SystemMetrics from './components/SystemMetrics';
import { ServiceMetrics, TraceInfo, SystemMetrics as SystemMetricsType } from './types';

// PERFORMANCE: Memoize the entire app to prevent unnecessary re-renders
const App = memo(() => {
  const [activeView, setActiveView] = useState<'health' | 'traces'>('health');
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

  // Poll for metrics - unlike Jaeger, we update efficiently
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
      <div className="flex items-center justify-center h-screen bg-slate-950">
        <div className="text-center">
          <div className="animate-pulse text-green-500 text-2xl mb-2">⚡</div>
          <p className="text-slate-400">Starting Urpo...</p>
          <p className="text-slate-600 text-sm mt-2">Target: &lt;200ms</p>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-slate-950 text-slate-100">
      {/* Header - Unlike Jaeger's heavy header, ours is minimal */}
      <header className="bg-slate-900 border-b border-slate-800 px-6 py-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-4">
            <h1 className="text-xl font-bold text-green-500">
              URPO
            </h1>
            <span className="text-xs text-slate-500">
              The Ferrari of Trace Explorers 🏎️
            </span>
          </div>
          
          <nav className="flex space-x-2">
            <button
              onClick={() => setActiveView('health')}
              className={`px-4 py-2 rounded transition-colors ${
                activeView === 'health'
                  ? 'bg-green-600 text-white'
                  : 'text-slate-400 hover:text-white hover:bg-slate-800'
              }`}
            >
              Service Health
            </button>
            <button
              onClick={() => setActiveView('traces')}
              className={`px-4 py-2 rounded transition-colors ${
                activeView === 'traces'
                  ? 'bg-green-600 text-white'
                  : 'text-slate-400 hover:text-white hover:bg-slate-800'
              }`}
            >
              Trace Explorer
            </button>
          </nav>

          {systemMetrics && (
            <div className="flex items-center space-x-4 text-xs text-slate-500">
              <span>
                Memory: {systemMetrics.memory_usage_mb.toFixed(1)}MB
              </span>
              <span>
                CPU: {systemMetrics.cpu_usage_percent.toFixed(1)}%
              </span>
              <span>
                {systemMetrics.spans_per_second.toFixed(0)} spans/s
              </span>
            </div>
          )}
        </div>
      </header>

      {/* Error display */}
      {error && (
        <div className="bg-red-900/20 border border-red-800 text-red-400 px-4 py-2 m-4 rounded">
          {error}
        </div>
      )}

      {/* Main content */}
      <main className="p-6">
        {activeView === 'health' ? (
          <>
            <ServiceHealthDashboard services={services} />
            {systemMetrics && <SystemMetrics metrics={systemMetrics} />}
          </>
        ) : (
          <TraceExplorer 
            traces={traces} 
            onRefresh={loadTraces}
          />
        )}
      </main>

      {/* Status bar - Shows we're NOT like Jaeger */}
      <footer className="fixed bottom-0 left-0 right-0 bg-slate-900 border-t border-slate-800 px-6 py-2 text-xs text-slate-500">
        <div className="flex justify-between">
          <span>
            {services.length} services • {traces.length} traces • {systemMetrics?.total_spans || 0} spans
          </span>
          <span>
            Jaeger crying in the corner with its 30s startup time 😢
          </span>
        </div>
      </footer>
    </div>
  );
});

App.displayName = 'App';

export default App;