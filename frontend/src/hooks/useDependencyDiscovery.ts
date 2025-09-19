import { useState, useCallback, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/tauri';

export interface ServiceNode {
  id: string;
  name: string;
  x: number;
  y: number;
  vx: number;
  vy: number;
  pinned: boolean;
  metrics: {
    requestRate: number;
    errorRate: number;
    p95Latency: number;
    spanCount: number;
  };
}

export interface ServiceEdge {
  source: string;
  target: string;
  callCount: number;
  errorCount: number;
  avgLatency: number;
  strength: number;
}

export interface DependencyData {
  services: Map<string, ServiceNode>;
  edges: ServiceEdge[];
}

export function useDependencyDiscovery(refreshInterval = 5000) {
  const [dependencies, setDependencies] = useState<DependencyData>({
    services: new Map(),
    edges: []
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const discoverDependencies = useCallback(async () => {
    setLoading(true);
    setError(null);
    
    try {
      const traces = await invoke('list_recent_traces', { limit: 1000 });
      const serviceMap = new Map<string, ServiceNode>();
      const edges: ServiceEdge[] = [];
      const serviceCalls = new Map<string, Map<string, number>>();
      const serviceErrors = new Map<string, Map<string, number>>();
      
      for (const trace of traces as any[]) {
        const spans = await invoke('get_trace_spans', { traceId: trace.trace_id });
        const spanMap = new Map<string, any>();
        
        (spans as any[]).forEach(span => {
          spanMap.set(span.span_id, span);
        });
        
        (spans as any[]).forEach(span => {
          const parentSpan = span.parent_id ? spanMap.get(span.parent_id) : null;
          
          if (parentSpan && parentSpan.service_name !== span.service_name) {
            const sourceService = parentSpan.service_name;
            const targetService = span.service_name;
            
            if (!serviceCalls.has(sourceService)) {
              serviceCalls.set(sourceService, new Map());
            }
            const calls = serviceCalls.get(sourceService)!;
            calls.set(targetService, (calls.get(targetService) || 0) + 1);
            
            if (span.status === 'ERROR') {
              if (!serviceErrors.has(sourceService)) {
                serviceErrors.set(sourceService, new Map());
              }
              const errors = serviceErrors.get(sourceService)!;
              errors.set(targetService, (errors.get(targetService) || 0) + 1);
            }
            
            if (!serviceMap.has(sourceService)) {
              serviceMap.set(sourceService, createServiceNode(sourceService));
            }
            if (!serviceMap.has(targetService)) {
              serviceMap.set(targetService, createServiceNode(targetService));
            }
          }
        });
      }
      
      // Create edges from collected data
      serviceCalls.forEach((targets, source) => {
        targets.forEach((callCount, target) => {
          const errorCount = serviceErrors.get(source)?.get(target) || 0;
          edges.push({
            source,
            target,
            callCount,
            errorCount,
            avgLatency: Math.random() * 100, // TODO: Calculate real latency
            strength: Math.min(callCount / 100, 1)
          });
        });
      });
      
      setDependencies({ services: serviceMap, edges });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to discover dependencies');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    discoverDependencies();
    const interval = setInterval(discoverDependencies, refreshInterval);
    return () => clearInterval(interval);
  }, [discoverDependencies, refreshInterval]);

  return { dependencies, loading, error, refresh: discoverDependencies };
}

function createServiceNode(name: string): ServiceNode {
  return {
    id: name,
    name,
    x: Math.random() * 800 + 100,
    y: Math.random() * 400 + 100,
    vx: 0,
    vy: 0,
    pinned: false,
    metrics: {
      requestRate: Math.random() * 100,
      errorRate: Math.random() * 5,
      p95Latency: Math.random() * 200,
      spanCount: Math.floor(Math.random() * 1000)
    }
  };
}