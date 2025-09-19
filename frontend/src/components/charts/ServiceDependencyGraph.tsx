import { useRef, useEffect, useState, useCallback, memo } from 'react';
import { useDependencyDiscovery } from '../../hooks/useDependencyDiscovery';
import { 
  applyForceLayout, 
  applyCircularLayout, 
  applyHierarchicalLayout,
  LayoutMode 
} from '../../utils/graph/layouts';
import { renderGraph, getNodeAtPosition, RenderOptions } from '../../utils/graph/renderer';

const ServiceDependencyGraphImpl = () => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animationRef = useRef<number>();
  const { dependencies, loading, error, refresh } = useDependencyDiscovery();
  
  const [selectedService, setSelectedService] = useState<string | null>(null);
  const [hoveredService, setHoveredService] = useState<string | null>(null);
  const [draggedNode, setDraggedNode] = useState<string | null>(null);
  const [showMetrics, setShowMetrics] = useState(true);
  const [layoutMode, setLayoutMode] = useState<LayoutMode>('force');
  
  const theme: RenderOptions['theme'] = {
    nodeRadius: 30,
    fontSize: 12,
    colors: {
      node: '#F3F4F6',
      nodeSelected: '#111827',
      nodeHovered: '#D1D5DB',
      edge: '#9CA3AF',
      text: '#111827',
      background: '#FFFFFF'
    }
  };

  // Apply layout when dependencies or mode changes
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    
    const { width, height } = canvas;
    
    switch (layoutMode) {
      case 'circular':
        applyCircularLayout(dependencies.services, width, height);
        break;
      case 'hierarchical':
        applyHierarchicalLayout(dependencies.services, dependencies.edges, width, height);
        break;
      case 'force':
      default:
        applyForceLayout(dependencies.services, dependencies.edges, width, height);
        break;
    }
  }, [dependencies, layoutMode]);

  // Animation loop
  useEffect(() => {
    const canvas = canvasRef.current;
    const ctx = canvas?.getContext('2d');
    if (!canvas || !ctx) return;

    const animate = () => {
      renderGraph(ctx, dependencies.services, dependencies.edges, {
        showMetrics,
        selectedService,
        hoveredService,
        theme
      });
      
      animationRef.current = requestAnimationFrame(animate);
    };

    animate();

    return () => {
      if (animationRef.current) {
        cancelAnimationFrame(animationRef.current);
      }
    };
  }, [dependencies, showMetrics, selectedService, hoveredService]);

  // Mouse event handlers
  const handleMouseMove = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    
    if (draggedNode) {
      const node = dependencies.services.get(draggedNode);
      if (node) {
        node.x = x;
        node.y = y;
        node.pinned = true;
      }
    } else {
      const nodeId = getNodeAtPosition(dependencies.services, x, y, theme.nodeRadius);
      setHoveredService(nodeId);
      canvas.style.cursor = nodeId ? 'pointer' : 'default';
    }
  }, [dependencies.services, draggedNode]);

  const handleMouseDown = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    
    const nodeId = getNodeAtPosition(dependencies.services, x, y, theme.nodeRadius);
    if (nodeId) {
      setDraggedNode(nodeId);
      setSelectedService(nodeId);
    }
  }, [dependencies.services]);

  const handleMouseUp = useCallback(() => {
    if (draggedNode) {
      const node = dependencies.services.get(draggedNode);
      if (node) {
        node.pinned = false;
      }
      setDraggedNode(null);
    }
  }, [draggedNode, dependencies.services]);

  if (loading && dependencies.services.size === 0) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <div className="w-8 h-8 border-2 border-text-700 rounded-full "></div>
          <p className="mt-4 text-text-500">Discovering service dependencies...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center text-status-error">
          <p>Failed to load dependencies</p>
          <p className="text-sm mt-2">{error}</p>
          <button onClick={refresh} className="mt-4 clean-button">
            Retry
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col bg-surface-50">
      {/* Controls */}
      <div className="clean-card border-b border-surface-300 p-4 rounded-none">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <h2 className="text-lg font-semibold text-text-900">Service Dependencies</h2>
            <div className="flex gap-2">
              {(['force', 'circular', 'hierarchical'] as LayoutMode[]).map(mode => (
                <button
                  key={mode}
                  onClick={() => setLayoutMode(mode)}
                  className={`clean-button text-xs ${layoutMode === mode ? 'active' : ''}`}
                >
                  {mode}
                </button>
              ))}
            </div>
          </div>
          
          <div className="flex items-center gap-4">
            <button
              onClick={() => setShowMetrics(!showMetrics)}
              className={`clean-button text-xs ${showMetrics ? 'active' : ''}`}
            >
              Metrics
            </button>
            <button onClick={refresh} className="clean-button text-xs">
              Refresh
            </button>
          </div>
        </div>
      </div>

      {/* Canvas */}
      <div className="flex-1 relative">
        <canvas
          ref={canvasRef}
          width={1200}
          height={600}
          className="w-full h-full"
          onMouseMove={handleMouseMove}
          onMouseDown={handleMouseDown}
          onMouseUp={handleMouseUp}
          onMouseLeave={handleMouseUp}
        />
        
        {/* Service details panel */}
        {selectedService && (
          <div className="absolute top-4 right-4 w-80 clean-card p-4">
            <h3 className="font-semibold text-text-900 mb-2">{selectedService}</h3>
            <div className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span className="text-text-500">Request Rate:</span>
                <span className="text-text-900">
                  {dependencies.services.get(selectedService)?.metrics.requestRate.toFixed(0)} req/s
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-text-500">Error Rate:</span>
                <span className="text-status-error">
                  {dependencies.services.get(selectedService)?.metrics.errorRate.toFixed(2)}%
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-text-500">P95 Latency:</span>
                <span className="text-text-900">
                  {dependencies.services.get(selectedService)?.metrics.p95Latency.toFixed(0)}ms
                </span>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export const ServiceDependencyGraph = memo(ServiceDependencyGraphImpl);
ServiceDependencyGraph.displayName = 'ServiceDependencyGraph';