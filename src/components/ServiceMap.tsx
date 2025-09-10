import { useState, useEffect, useMemo, memo } from 'react';
import { ServiceMap as ServiceMapType, ServiceNode, ServiceEdge, ServiceMapProps, ServiceMapViewMode } from '../types';
import { isTauriAvailable, getServiceMap } from '../utils/tauri';

const ServiceMap = memo(({ className = '' }: ServiceMapProps) => {
  const [serviceMap, setServiceMap] = useState<ServiceMapType | null>(null);
  const [selectedService, setSelectedService] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<ServiceMapViewMode>('topology');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Load service map data
  const loadServiceMap = async () => {
    try {
      setLoading(true);
      setError(null);

      let mapData: ServiceMapType;

      if (isTauriAvailable()) {
        // Use real Tauri backend with new getServiceMap helper
        const result = await getServiceMap();
        if (result) {
          mapData = result;
        } else {
          throw new Error('Failed to get service map from backend');
        }
      } else {
        // Use mock data for demo
        mapData = generateMockServiceMap();
      }

      setServiceMap(mapData);
    } catch (err) {
      console.error('Error loading service map:', err);
      setError('Failed to load service map');
      // Fallback to mock data
      setServiceMap(generateMockServiceMap());
    } finally {
      setLoading(false);
    }
  };

  // Auto-refresh service map
  useEffect(() => {
    loadServiceMap();

    const interval = setInterval(loadServiceMap, 30000); // Refresh every 30s
    return () => clearInterval(interval);
  }, []);

  // Group nodes by tier for display
  const nodesByTier = useMemo(() => {
    if (!serviceMap) return {};
    
    const tiers: Record<number, ServiceNode[]> = {};
    serviceMap.nodes.forEach(node => {
      if (!tiers[node.tier]) tiers[node.tier] = [];
      tiers[node.tier].push(node);
    });
    
    return tiers;
  }, [serviceMap]);

  // Filter edges based on view mode
  const filteredEdges = useMemo(() => {
    if (!serviceMap) return [];
    
    switch (viewMode) {
      case 'focus':
        if (!selectedService) return serviceMap.edges;
        return serviceMap.edges.filter(edge => 
          edge.from === selectedService || edge.to === selectedService
        );
      case 'hotpaths':
        return [...serviceMap.edges]
          .sort((a, b) => b.call_count - a.call_count)
          .slice(0, 10);
      case 'errors':
        return serviceMap.edges.filter(edge => edge.error_count > 0);
      default:
        return serviceMap.edges;
    }
  }, [serviceMap, viewMode, selectedService]);

  // Get health color for a service
  const getHealthColor = (node: ServiceNode) => {
    if (node.error_rate > 0.1) return 'text-status-error';
    if (node.error_rate > 0.01) return 'text-status-warning';
    return 'text-status-healthy';
  };

  // Get health color class for background
  const getHealthBgColor = (node: ServiceNode) => {
    if (node.error_rate > 0.1) return 'bg-status-error bg-opacity-5 border-status-error border-opacity-20';
    if (node.error_rate > 0.01) return 'bg-status-warning bg-opacity-5 border-status-warning border-opacity-20';
    return 'bg-status-healthy bg-opacity-5 border-status-healthy border-opacity-20';
  };

  if (loading) {
    return (
      <div className={`clean-card p-6 ${className}`}>
        <div className="animate-pulse">
          <div className="h-4 bg-surface-200 rounded w-48 mb-4"></div>
          <div className="space-y-3">
            <div className="h-12 bg-surface-200 rounded"></div>
            <div className="h-12 bg-surface-200 rounded"></div>
            <div className="h-12 bg-surface-200 rounded"></div>
          </div>
        </div>
      </div>
    );
  }

  if (error || !serviceMap) {
    return (
      <div className={`clean-card p-6 ${className}`}>
        <div className="text-center">
          <div className="text-status-error mb-2">Service Map Unavailable</div>
          <div className="text-text-muted text-sm mb-4">
            {error || 'No service map data available'}
          </div>
          <button
            onClick={loadServiceMap}
            className="clean-button text-sm"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className={`clean-card ${className}`}>
      {/* Header with view mode selector */}
      <div className="p-4 border-b border-surface-200">
        <div className="flex justify-between items-center">
          <div>
            <h2 className="text-lg font-semibold text-text-primary">Service Map</h2>
            <div className="text-sm text-text-muted">
              {serviceMap.nodes.length} services, {serviceMap.edges.length} dependencies
              • {serviceMap.trace_count} traces analyzed
            </div>
          </div>
          
          <div className="flex gap-2">
            {(['topology', 'focus', 'hotpaths', 'errors'] as ServiceMapViewMode[]).map(mode => (
              <button
                key={mode}
                onClick={() => setViewMode(mode)}
                className={`clean-button text-xs ${
                  viewMode === mode ? 'active' : ''
                }`}
              >
                {mode === 'topology' && 'Topology'}
                {mode === 'focus' && 'Focus'}
                {mode === 'hotpaths' && 'Hot Paths'}
                {mode === 'errors' && 'Errors'}
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* Service map visualization */}
      <div className="p-6 overflow-auto" style={{ maxHeight: '600px' }}>
        {viewMode === 'topology' && (
          <TopologyView
            nodesByTier={nodesByTier}
            edges={filteredEdges}
            selectedService={selectedService}
            onSelectService={setSelectedService}
            getHealthColor={getHealthColor}
            getHealthBgColor={getHealthBgColor}
          />
        )}
        
        {viewMode === 'focus' && selectedService && (
          <FocusView
            serviceMap={serviceMap}
            selectedService={selectedService}
            getHealthColor={getHealthColor}
          />
        )}
        
        {viewMode === 'hotpaths' && (
          <HotPathsView edges={filteredEdges} />
        )}
        
        {viewMode === 'errors' && (
          <ErrorPathsView edges={filteredEdges} />
        )}
      </div>
    </div>
  );
});

// Topology view component
const TopologyView = memo(({ 
  nodesByTier, 
  edges, 
  selectedService, 
  onSelectService,
  getHealthColor,
  getHealthBgColor
}: {
  nodesByTier: Record<number, ServiceNode[]>;
  edges: ServiceEdge[];
  selectedService: string | null;
  onSelectService: (service: string) => void;
  getHealthColor: (node: ServiceNode) => string;
  getHealthBgColor: (node: ServiceNode) => string;
}) => {
  const maxTier = Math.max(...Object.keys(nodesByTier).map(Number));

  return (
    <div className="space-y-8">
      {Array.from({ length: maxTier + 1 }, (_, tier) => (
        <div key={tier} className="space-y-4">
          {nodesByTier[tier] && (
            <>
              <div className="text-sm font-medium text-text-muted border-b border-surface-200 pb-2">
                Tier {tier} {tier === 0 ? '(Root Services)' : tier === maxTier ? '(Leaf Services)' : ''}
              </div>
              
              <div className="flex flex-wrap gap-4">
                {nodesByTier[tier].map(node => (
                  <button
                    key={node.name}
                    onClick={() => onSelectService(node.name)}
                    className={`p-3 rounded-lg border-2 transition-all hover:scale-105 ${
                      selectedService === node.name 
                        ? 'ring-2 ring-status-info' 
                        : ''
                    } ${getHealthBgColor(node)}`}
                  >
                    <div className={`font-medium ${getHealthColor(node)}`}>
                      {node.name}
                    </div>
                    <div className="text-xs text-text-muted space-y-1">
                      <div>{node.request_count.toLocaleString()} requests</div>
                      <div>{(node.error_rate * 100).toFixed(1)}% errors</div>
                      <div>{(node.avg_latency_us / 1000).toFixed(1)}ms avg</div>
                    </div>
                  </button>
                ))}
              </div>
              
              {/* Show connections to next tier */}
              {tier < maxTier && (
                <div className="flex justify-center">
                  <div className="text-status-info text-2xl">↓</div>
                </div>
              )}
            </>
          )}
        </div>
      ))}
    </div>
  );
});

// Focus view component
const FocusView = memo(({ serviceMap, selectedService, getHealthColor }: {
  serviceMap: ServiceMapType;
  selectedService: string;
  getHealthColor: (node: ServiceNode) => string;
}) => {
  const selectedNode = serviceMap.nodes.find(n => n.name === selectedService);
  const incomingEdges = serviceMap.edges.filter(e => e.to === selectedService);
  const outgoingEdges = serviceMap.edges.filter(e => e.from === selectedService);

  if (!selectedNode) return null;

  return (
    <div className="space-y-6">
      {/* Selected service info */}
      <div className="text-center p-4 bg-surface-100 rounded-lg">
        <h3 className={`text-xl font-bold ${getHealthColor(selectedNode)}`}>
          {selectedService}
        </h3>
        <div className="grid grid-cols-3 gap-4 mt-2 text-sm">
          <div>
            <div className="text-text-muted">Requests</div>
            <div className="font-medium">{selectedNode.request_count.toLocaleString()}</div>
          </div>
          <div>
            <div className="text-text-muted">Error Rate</div>
            <div className="font-medium">{(selectedNode.error_rate * 100).toFixed(2)}%</div>
          </div>
          <div>
            <div className="text-text-muted">Avg Latency</div>
            <div className="font-medium">{(selectedNode.avg_latency_us / 1000).toFixed(1)}ms</div>
          </div>
        </div>
      </div>

      <div className="grid md:grid-cols-2 gap-6">
        {/* Incoming dependencies */}
        <div>
          <h4 className="font-medium text-text-primary mb-3">
            Incoming Dependencies ({incomingEdges.length})
          </h4>
          {incomingEdges.length === 0 ? (
            <div className="text-text-muted text-sm italic">Root service - no incoming calls</div>
          ) : (
            <div className="space-y-2">
              {incomingEdges.map(edge => (
                <EdgeCard key={`${edge.from}-${edge.to}`} edge={edge} direction="incoming" />
              ))}
            </div>
          )}
        </div>

        {/* Outgoing dependencies */}
        <div>
          <h4 className="font-medium text-text-primary mb-3">
            Outgoing Dependencies ({outgoingEdges.length})
          </h4>
          {outgoingEdges.length === 0 ? (
            <div className="text-text-muted text-sm italic">Leaf service - no outgoing calls</div>
          ) : (
            <div className="space-y-2">
              {outgoingEdges.map(edge => (
                <EdgeCard key={`${edge.from}-${edge.to}`} edge={edge} direction="outgoing" />
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
});

// Edge card component
const EdgeCard = memo(({ edge, direction }: { edge: ServiceEdge; direction: 'incoming' | 'outgoing' }) => {
  const errorRate = edge.call_count > 0 ? (edge.error_count / edge.call_count) * 100 : 0;
  const serviceName = direction === 'incoming' ? edge.from : edge.to;

  return (
    <div className="clean-card p-3">
      <div className="flex justify-between items-start">
        <div>
          <div className="font-medium text-text-primary">{serviceName}</div>
          <div className="text-xs text-text-muted">
            {edge.operations.slice(0, 2).join(', ')}
            {edge.operations.length > 2 && ` +${edge.operations.length - 2} more`}
          </div>
        </div>
        <div className="text-right text-xs">
          <div className="text-text-primary">{edge.call_count} calls</div>
          <div className={errorRate > 10 ? 'text-status-error' : errorRate > 1 ? 'text-status-warning' : 'text-status-healthy'}>
            {errorRate.toFixed(1)}% errors
          </div>
          <div className="text-text-muted">{(edge.avg_latency_us / 1000).toFixed(1)}ms avg</div>
        </div>
      </div>
    </div>
  );
});

// Hot paths view
const HotPathsView = memo(({ edges }: { edges: ServiceEdge[] }) => (
  <div className="space-y-4">
    <h3 className="text-lg font-semibold text-text-primary">Hottest Paths</h3>
    <div className="space-y-2">
      {edges.map((edge, index) => (
        <div key={`${edge.from}-${edge.to}`} className="clean-card p-4">
          <div className="flex justify-between items-center">
            <div>
              <span className="text-text-muted">#{index + 1}</span>
              <span className="ml-2 font-medium">{edge.from} → {edge.to}</span>
            </div>
            <div className="text-right">
              <div className="font-bold text-status-info">{edge.call_count.toLocaleString()} calls</div>
              <div className="text-sm text-text-muted">{(edge.avg_latency_us / 1000).toFixed(1)}ms avg</div>
            </div>
          </div>
        </div>
      ))}
    </div>
  </div>
));

// Error paths view
const ErrorPathsView = memo(({ edges }: { edges: ServiceEdge[] }) => (
  <div className="space-y-4">
    <h3 className="text-lg font-semibold text-text-primary">Error Paths</h3>
    {edges.length === 0 ? (
      <div className="clean-card text-center p-8">
        <div className="w-12 h-12 mx-auto mb-4 rounded-lg bg-status-healthy bg-opacity-10 flex items-center justify-center">
          <span className="text-status-healthy text-2xl">✓</span>
        </div>
        <p className="text-status-healthy font-medium">No errors detected!</p>
        <p className="text-text-500 text-sm mt-1">All service connections are healthy.</p>
      </div>
    ) : (
      <div className="space-y-2">
        {edges
          .sort((a, b) => (b.error_count / b.call_count) - (a.error_count / a.call_count))
          .map((edge, index) => {
            const errorRate = (edge.error_count / edge.call_count) * 100;
            return (
              <div key={`${edge.from}-${edge.to}`} className="clean-card p-4 border-l-4 border-status-error">
                <div className="flex justify-between items-center">
                  <div>
                    <span className="text-text-muted">#{index + 1}</span>
                    <span className="ml-2 font-medium">{edge.from} → {edge.to}</span>
                  </div>
                  <div className="text-right">
                    <div className="font-bold text-status-error">{errorRate.toFixed(1)}% errors</div>
                    <div className="text-sm text-text-muted">
                      {edge.error_count}/{edge.call_count} calls failed
                    </div>
                  </div>
                </div>
              </div>
            );
          })}
      </div>
    )}
  </div>
));

// Generate mock service map for demo
function generateMockServiceMap(): ServiceMapType {
  return {
    nodes: [
      { name: 'frontend', request_count: 1500, error_rate: 0.002, avg_latency_us: 50000, is_root: true, is_leaf: false, tier: 0 },
      { name: 'api-gateway', request_count: 1500, error_rate: 0.001, avg_latency_us: 25000, is_root: false, is_leaf: false, tier: 1 },
      { name: 'auth-service', request_count: 800, error_rate: 0.005, avg_latency_us: 15000, is_root: false, is_leaf: false, tier: 2 },
      { name: 'user-service', request_count: 600, error_rate: 0.003, avg_latency_us: 30000, is_root: false, is_leaf: false, tier: 2 },
      { name: 'order-service', request_count: 400, error_rate: 0.008, avg_latency_us: 45000, is_root: false, is_leaf: false, tier: 2 },
      { name: 'payment-db', request_count: 200, error_rate: 0.001, avg_latency_us: 5000, is_root: false, is_leaf: true, tier: 3 },
      { name: 'user-db', request_count: 300, error_rate: 0.000, avg_latency_us: 3000, is_root: false, is_leaf: true, tier: 3 },
    ],
    edges: [
      { from: 'frontend', to: 'api-gateway', call_count: 1500, error_count: 2, avg_latency_us: 25000, p99_latency_us: 100000, operations: ['GET /api/*', 'POST /api/*'] },
      { from: 'api-gateway', to: 'auth-service', call_count: 800, error_count: 4, avg_latency_us: 15000, p99_latency_us: 50000, operations: ['validate_token', 'refresh_token'] },
      { from: 'api-gateway', to: 'user-service', call_count: 600, error_count: 2, avg_latency_us: 30000, p99_latency_us: 120000, operations: ['get_user', 'update_profile'] },
      { from: 'api-gateway', to: 'order-service', call_count: 400, error_count: 3, avg_latency_us: 45000, p99_latency_us: 200000, operations: ['create_order', 'get_orders'] },
      { from: 'order-service', to: 'payment-db', call_count: 200, error_count: 0, avg_latency_us: 5000, p99_latency_us: 20000, operations: ['INSERT payment', 'SELECT payment'] },
      { from: 'user-service', to: 'user-db', call_count: 300, error_count: 0, avg_latency_us: 3000, p99_latency_us: 15000, operations: ['SELECT user', 'UPDATE user'] },
    ],
    generated_at: Date.now(),
    trace_count: 1000,
    time_window_seconds: 3600,
  };
}

export default ServiceMap;