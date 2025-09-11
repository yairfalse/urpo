// SERVICE DEPENDENCY GRAPH - AUTO-DISCOVERED FROM TRACES
import React, { useRef, useEffect, useState, useCallback, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/tauri';

interface ServiceNode {
  id: string;
  name: string;
  x: number;
  y: number;
  vx: number;  // velocity for physics
  vy: number;
  pinned: boolean;
  metrics: {
    requestRate: number;
    errorRate: number;
    p95Latency: number;
    spanCount: number;
  };
}

interface ServiceEdge {
  source: string;
  target: string;
  callCount: number;
  errorCount: number;
  avgLatency: number;
  strength: number; // 0-1 for visualization
}

interface DependencyData {
  services: Map<string, ServiceNode>;
  edges: ServiceEdge[];
}

const ServiceDependencyGraph: React.FC = () => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animationRef = useRef<number>();
  const [dependencies, setDependencies] = useState<DependencyData>({
    services: new Map(),
    edges: []
  });
  const [selectedService, setSelectedService] = useState<string | null>(null);
  const [hoveredService, setHoveredService] = useState<string | null>(null);
  const [draggedNode, setDraggedNode] = useState<string | null>(null);
  const [showMetrics, setShowMetrics] = useState(true);
  const [layoutMode, setLayoutMode] = useState<'force' | 'hierarchical' | 'circular'>('force');
  
  // Discover dependencies from traces
  const discoverDependencies = useCallback(async () => {
    try {
      // Get recent traces
      const traces = await invoke('list_recent_traces', { limit: 1000 });
      
      // Build dependency graph from spans
      const serviceMap = new Map<string, ServiceNode>();
      const edgeMap = new Map<string, ServiceEdge>();
      const serviceCalls = new Map<string, Map<string, number>>();
      const serviceErrors = new Map<string, Map<string, number>>();
      
      // Analyze each trace
      for (const trace of traces as any[]) {
        const spans = await invoke('get_trace_spans', { traceId: trace.trace_id });
        
        // Build parent-child relationships
        const spanMap = new Map<string, any>();
        (spans as any[]).forEach(span => {
          spanMap.set(span.span_id, span);
        });
        
        // Find service dependencies
        (spans as any[]).forEach(span => {
          const parentSpan = span.parent_id ? spanMap.get(span.parent_id) : null;
          
          if (parentSpan && parentSpan.service_name !== span.service_name) {
            // This is a cross-service call
            const sourceService = parentSpan.service_name;
            const targetService = span.service_name;
            
            // Track calls
            if (!serviceCalls.has(sourceService)) {
              serviceCalls.set(sourceService, new Map());
            }
            const calls = serviceCalls.get(sourceService)!;
            calls.set(targetService, (calls.get(targetService) || 0) + 1);
            
            // Track errors
            if (span.status === 'ERROR') {
              if (!serviceErrors.has(sourceService)) {
                serviceErrors.set(sourceService, new Map());
              }
              const errors = serviceErrors.get(sourceService)!;
              errors.set(targetService, (errors.get(targetService) || 0) + 1);
            }
            
            // Create/update service nodes
            if (!serviceMap.has(sourceService)) {
              serviceMap.set(sourceService, {
                id: sourceService,
                name: sourceService,
                x: Math.random() * 800 + 100,
                y: Math.random() * 400 + 100,
                vx: 0,
                vy: 0,
                pinned: false,
                metrics: {
                  requestRate: 0,
                  errorRate: 0,
                  p95Latency: 0,
                  spanCount: 0
                }
              });
            }
            
            if (!serviceMap.has(targetService)) {
              serviceMap.set(targetService, {
                id: targetService,
                name: targetService,
                x: Math.random() * 800 + 100,
                y: Math.random() * 400 + 100,
                vx: 0,
                vy: 0,
                pinned: false,
                metrics: {
                  requestRate: 0,
                  errorRate: 0,
                  p95Latency: 0,
                  spanCount: 0
                }
              });
            }
            
            // Update metrics
            const sourceNode = serviceMap.get(sourceService)!;
            sourceNode.metrics.spanCount++;
            
            const targetNode = serviceMap.get(targetService)!;
            targetNode.metrics.spanCount++;
            targetNode.metrics.p95Latency = Math.max(
              targetNode.metrics.p95Latency,
              span.duration / 1000 // Convert to ms
            );
          }
        });
      }
      
      // Build edges from discovered dependencies
      const edges: ServiceEdge[] = [];
      serviceCalls.forEach((targets, source) => {
        targets.forEach((callCount, target) => {
          const errorCount = serviceErrors.get(source)?.get(target) || 0;
          edges.push({
            source,
            target,
            callCount,
            errorCount,
            avgLatency: 0, // TODO: Calculate from spans
            strength: Math.min(callCount / 100, 1) // Normalize
          });
        });
      });
      
      // Get real metrics for services
      const metrics = await invoke('get_service_metrics');
      (metrics as any[]).forEach(metric => {
        const node = serviceMap.get(metric.name);
        if (node) {
          node.metrics.requestRate = metric.request_rate;
          node.metrics.errorRate = metric.error_rate;
          node.metrics.p95Latency = metric.latency_p95;
        }
      });
      
      setDependencies({ services: serviceMap, edges });
    } catch (error) {
      console.error('Failed to discover dependencies:', error);
    }
  }, []);
  
  // Apply force-directed layout
  const applyForceLayout = useCallback(() => {
    const nodes = Array.from(dependencies.services.values());
    const edges = dependencies.edges;
    
    // Apply forces
    nodes.forEach(node => {
      if (node.pinned || node.id === draggedNode) return;
      
      // Reset forces
      let fx = 0, fy = 0;
      
      // Repulsion between all nodes
      nodes.forEach(other => {
        if (node.id === other.id) return;
        
        const dx = node.x - other.x;
        const dy = node.y - other.y;
        const distance = Math.sqrt(dx * dx + dy * dy) || 1;
        const force = 5000 / (distance * distance);
        
        fx += (dx / distance) * force;
        fy += (dy / distance) * force;
      });
      
      // Attraction along edges
      edges.forEach(edge => {
        let other: ServiceNode | undefined;
        if (edge.source === node.id) {
          other = dependencies.services.get(edge.target);
        } else if (edge.target === node.id) {
          other = dependencies.services.get(edge.source);
        }
        
        if (other) {
          const dx = other.x - node.x;
          const dy = other.y - node.y;
          const distance = Math.sqrt(dx * dx + dy * dy) || 1;
          const force = distance * 0.1 * edge.strength;
          
          fx += dx * force / distance;
          fy += dy * force / distance;
        }
      });
      
      // Center gravity
      fx += (500 - node.x) * 0.01;
      fy += (300 - node.y) * 0.01;
      
      // Apply velocity with damping
      node.vx = (node.vx + fx) * 0.8;
      node.vy = (node.vy + fy) * 0.8;
      
      // Update position
      node.x += node.vx;
      node.y += node.vy;
      
      // Keep in bounds
      node.x = Math.max(50, Math.min(950, node.x));
      node.y = Math.max(50, Math.min(550, node.y));
    });
  }, [dependencies, draggedNode]);
  
  // Apply hierarchical layout
  const applyHierarchicalLayout = useCallback(() => {
    const nodes = Array.from(dependencies.services.values());
    const edges = dependencies.edges;
    
    // Find root nodes (no incoming edges)
    const roots = nodes.filter(node => 
      !edges.some(edge => edge.target === node.id)
    );
    
    // Build levels
    const levels = new Map<string, number>();
    const visited = new Set<string>();
    
    const assignLevel = (nodeId: string, level: number) => {
      if (visited.has(nodeId)) return;
      visited.add(nodeId);
      levels.set(nodeId, Math.max(levels.get(nodeId) || 0, level));
      
      // Find children
      edges.filter(e => e.source === nodeId).forEach(edge => {
        assignLevel(edge.target, level + 1);
      });
    };
    
    roots.forEach(root => assignLevel(root.id, 0));
    
    // Position nodes by level
    const levelNodes = new Map<number, string[]>();
    levels.forEach((level, nodeId) => {
      if (!levelNodes.has(level)) {
        levelNodes.set(level, []);
      }
      levelNodes.get(level)!.push(nodeId);
    });
    
    levelNodes.forEach((nodeIds, level) => {
      const y = 100 + level * 150;
      const spacing = 800 / (nodeIds.length + 1);
      
      nodeIds.forEach((nodeId, index) => {
        const node = dependencies.services.get(nodeId);
        if (node && !node.pinned) {
          node.x = 100 + spacing * (index + 1);
          node.y = y;
        }
      });
    });
  }, [dependencies]);
  
  // Apply circular layout
  const applyCircularLayout = useCallback(() => {
    const nodes = Array.from(dependencies.services.values());
    const centerX = 500;
    const centerY = 300;
    const radius = 200;
    
    nodes.forEach((node, index) => {
      if (!node.pinned) {
        const angle = (index / nodes.length) * Math.PI * 2;
        node.x = centerX + Math.cos(angle) * radius;
        node.y = centerY + Math.sin(angle) * radius;
      }
    });
  }, [dependencies]);
  
  // Main render loop
  const render = useCallback(() => {
    const canvas = canvasRef.current;
    const ctx = canvas?.getContext('2d');
    if (!ctx) return;
    
    // Clear
    ctx.fillStyle = '#0a0a0a';
    ctx.fillRect(0, 0, 1000, 600);
    
    // Apply layout
    if (layoutMode === 'force') {
      applyForceLayout();
    }
    
    // Draw edges
    dependencies.edges.forEach(edge => {
      const source = dependencies.services.get(edge.source);
      const target = dependencies.services.get(edge.target);
      
      if (!source || !target) return;
      
      // Edge thickness based on call count
      ctx.lineWidth = Math.max(1, Math.min(5, edge.callCount / 10));
      
      // Edge color based on errors
      const errorRate = edge.errorCount / edge.callCount;
      if (errorRate > 0.1) {
        ctx.strokeStyle = '#ff3366';
      } else if (errorRate > 0.01) {
        ctx.strokeStyle = '#ffaa00';
      } else {
        ctx.strokeStyle = '#333';
      }
      
      // Highlight selected paths
      if (selectedService && (edge.source === selectedService || edge.target === selectedService)) {
        ctx.strokeStyle = '#6B7280';
        ctx.lineWidth *= 2;
      }
      
      // Draw arrow
      ctx.beginPath();
      ctx.moveTo(source.x, source.y);
      ctx.lineTo(target.x, target.y);
      ctx.stroke();
      
      // Arrowhead
      const angle = Math.atan2(target.y - source.y, target.x - source.x);
      const arrowLength = 10;
      const arrowAngle = Math.PI / 6;
      
      ctx.beginPath();
      ctx.moveTo(target.x, target.y);
      ctx.lineTo(
        target.x - arrowLength * Math.cos(angle - arrowAngle),
        target.y - arrowLength * Math.sin(angle - arrowAngle)
      );
      ctx.moveTo(target.x, target.y);
      ctx.lineTo(
        target.x - arrowLength * Math.cos(angle + arrowAngle),
        target.y - arrowLength * Math.sin(angle + arrowAngle)
      );
      ctx.stroke();
      
      // Edge label (call count)
      if (showMetrics && edge.callCount > 0) {
        const midX = (source.x + target.x) / 2;
        const midY = (source.y + target.y) / 2;
        
        ctx.fillStyle = '#666';
        ctx.font = '10px monospace';
        ctx.textAlign = 'center';
        ctx.fillText(`${edge.callCount}`, midX, midY - 5);
        
        if (edge.errorCount > 0) {
          ctx.fillStyle = '#ff3366';
          ctx.fillText(`${edge.errorCount} err`, midX, midY + 5);
        }
      }
    });
    
    // Draw nodes
    dependencies.services.forEach(node => {
      const isHovered = node.id === hoveredService;
      const isSelected = node.id === selectedService;
      
      // Node size based on request rate
      const baseSize = 20;
      const size = baseSize + Math.sqrt(node.metrics.requestRate) * 2;
      
      // Pulsing effect for active nodes
      const pulse = Math.sin(Date.now() * 0.001 * node.metrics.requestRate * 0.1) * 2;
      const actualSize = size + pulse;
      
      // Node color based on health
      let fillColor = '#374151';
      let borderColor = '#6B7280';
      
      if (node.metrics.errorRate > 0.05) {
        fillColor = '#220011';
        borderColor = '#ff3366';
      } else if (node.metrics.errorRate > 0.01) {
        fillColor = '#221100';
        borderColor = '#ffaa00';
      }
      
      // Glow effect for active nodes
      if (node.metrics.requestRate > 0) {
        ctx.shadowBlur = 10 + pulse;
        ctx.shadowColor = borderColor;
      }
      
      // Draw node
      ctx.fillStyle = fillColor;
      ctx.fillRect(node.x - actualSize, node.y - actualSize/2, actualSize * 2, actualSize);
      
      ctx.strokeStyle = borderColor;
      ctx.lineWidth = isSelected ? 3 : isHovered ? 2 : 1;
      ctx.strokeRect(node.x - actualSize, node.y - actualSize/2, actualSize * 2, actualSize);
      
      ctx.shadowBlur = 0;
      
      // Node label
      ctx.fillStyle = '#fff';
      ctx.font = `${isSelected || isHovered ? 'bold' : ''} 11px monospace`;
      ctx.textAlign = 'center';
      ctx.fillText(node.name, node.x, node.y + 3);
      
      // Metrics
      if (showMetrics) {
        ctx.fillStyle = '#888';
        ctx.font = '9px monospace';
        ctx.fillText(`${node.metrics.requestRate.toFixed(1)} rps`, node.x, node.y + 15);
        
        if (node.metrics.errorRate > 0) {
          ctx.fillStyle = '#ff3366';
          ctx.fillText(`${(node.metrics.errorRate * 100).toFixed(1)}% err`, node.x, node.y + 25);
        }
      }
      
      // Pin indicator
      if (node.pinned) {
        ctx.fillStyle = '#ffaa00';
        ctx.font = '12px monospace';
        ctx.fillText('📌', node.x + actualSize - 5, node.y - actualSize/2 + 10);
      }
    });
    
    // Legend
    ctx.fillStyle = '#666';
    ctx.font = '10px monospace';
    ctx.textAlign = 'left';
    ctx.fillText('DEPENDENCY GRAPH', 10, 20);
    ctx.fillText(`${dependencies.services.size} services, ${dependencies.edges.length} dependencies`, 10, 35);
    
    // Controls hint
    ctx.textAlign = 'right';
    ctx.fillText('Drag nodes • Click to select • Double-click to pin', 990, 20);
    
    // Continue animation
    animationRef.current = requestAnimationFrame(render);
  }, [dependencies, selectedService, hoveredService, showMetrics, layoutMode, applyForceLayout]);
  
  // Mouse handlers
  const handleMouseMove = useCallback((e: React.MouseEvent) => {
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
        node.vx = 0;
        node.vy = 0;
      }
    } else {
      // Check hover
      let hovered: string | null = null;
      dependencies.services.forEach(node => {
        const dx = x - node.x;
        const dy = y - node.y;
        if (Math.abs(dx) < 30 && Math.abs(dy) < 20) {
          hovered = node.id;
        }
      });
      setHoveredService(hovered);
    }
  }, [dependencies, draggedNode]);
  
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    if (hoveredService) {
      setDraggedNode(hoveredService);
      setSelectedService(hoveredService);
    }
  }, [hoveredService]);
  
  const handleMouseUp = useCallback(() => {
    setDraggedNode(null);
  }, []);
  
  const handleDoubleClick = useCallback(() => {
    if (hoveredService) {
      const node = dependencies.services.get(hoveredService);
      if (node) {
        node.pinned = !node.pinned;
      }
    }
  }, [hoveredService, dependencies]);
  
  // Initial load and animation
  useEffect(() => {
    discoverDependencies();
    const interval = setInterval(discoverDependencies, 5000); // Refresh every 5s
    
    return () => clearInterval(interval);
  }, [discoverDependencies]);
  
  useEffect(() => {
    animationRef.current = requestAnimationFrame(render);
    
    return () => {
      if (animationRef.current) {
        cancelAnimationFrame(animationRef.current);
      }
    };
  }, [render]);
  
  // Apply layout when mode changes
  useEffect(() => {
    if (layoutMode === 'hierarchical') {
      applyHierarchicalLayout();
    } else if (layoutMode === 'circular') {
      applyCircularLayout();
    }
  }, [layoutMode, applyHierarchicalLayout, applyCircularLayout]);
  
  return (
    <div className="relative">
      {/* Controls */}
      <div className="absolute top-2 left-2 z-10 flex gap-2">
        <button
          onClick={() => setLayoutMode('force')}
          className={`px-3 py-1 text-xs font-mono ${
            layoutMode === 'force' ? 'bg-text-900 text-white' : 'bg-surface-200 text-text-500'
          }`}
        >
          FORCE
        </button>
        <button
          onClick={() => setLayoutMode('hierarchical')}
          className={`px-3 py-1 text-xs font-mono ${
            layoutMode === 'hierarchical' ? 'bg-text-900 text-white' : 'bg-surface-200 text-text-500'
          }`}
        >
          HIERARCHY
        </button>
        <button
          onClick={() => setLayoutMode('circular')}
          className={`px-3 py-1 text-xs font-mono ${
            layoutMode === 'circular' ? 'bg-text-900 text-white' : 'bg-surface-200 text-text-500'
          }`}
        >
          CIRCULAR
        </button>
        <button
          onClick={() => setShowMetrics(!showMetrics)}
          className={`px-3 py-1 text-xs font-mono ${
            showMetrics ? 'bg-text-900 text-white' : 'bg-surface-100 text-text-500'
          }`}
        >
          METRICS
        </button>
      </div>
      
      {/* Canvas */}
      <canvas
        ref={canvasRef}
        width={1000}
        height={600}
        className="border border-surface-400 cursor-move"
        onMouseMove={handleMouseMove}
        onMouseDown={handleMouseDown}
        onMouseUp={handleMouseUp}
        onMouseLeave={() => setDraggedNode(null)}
        onDoubleClick={handleDoubleClick}
      />
      
      {/* Selected service details */}
      {selectedService && dependencies.services.get(selectedService) && (
        <div className="absolute bottom-2 right-2 bg-surface-100 border border-surface-400 p-3 text-xs font-mono w-64">
          <div className="text-text-900 font-bold mb-2">{selectedService}</div>
          <div className="space-y-1 text-text-700">
            <div>Request Rate: {dependencies.services.get(selectedService)!.metrics.requestRate.toFixed(2)} rps</div>
            <div>Error Rate: {(dependencies.services.get(selectedService)!.metrics.errorRate * 100).toFixed(2)}%</div>
            <div>P95 Latency: {dependencies.services.get(selectedService)!.metrics.p95Latency}ms</div>
            <div>Span Count: {dependencies.services.get(selectedService)!.metrics.spanCount}</div>
          </div>
          
          {/* Upstream dependencies */}
          <div className="mt-2 pt-2 border-t border-surface-400">
            <div className="text-text-500 mb-1">Calls from:</div>
            {dependencies.edges
              .filter(e => e.target === selectedService)
              .map(e => (
                <div key={e.source} className="text-text-500">
                  ← {e.source} ({e.callCount} calls)
                </div>
              ))}
          </div>
          
          {/* Downstream dependencies */}
          <div className="mt-2 pt-2 border-t border-surface-400">
            <div className="text-text-500 mb-1">Calls to:</div>
            {dependencies.edges
              .filter(e => e.source === selectedService)
              .map(e => (
                <div key={e.target} className="text-text-500">
                  → {e.target} ({e.callCount} calls)
                </div>
              ))}
          </div>
        </div>
      )}
    </div>
  );
};

export default ServiceDependencyGraph;