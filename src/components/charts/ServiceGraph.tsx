import { useEffect, useRef, useState, useCallback, useMemo } from 'react';
import * as d3 from 'd3';
import { Activity, AlertCircle, CheckCircle, Clock, Network } from 'lucide-react';
import { ServiceMetrics, TraceInfo } from '../../types';

interface ServiceNode {
  id: string;
  name: string;
  requestRate: number;
  errorRate: number;
  latencyP95: number;
  health: 'healthy' | 'degraded' | 'critical';
  type: 'service' | 'database' | 'cache' | 'external';
}

interface ServiceLink {
  source: string;
  target: string;
  requestRate: number;
  errorRate: number;
  latency: number;
}

interface ServiceGraphProps {
  services: ServiceMetrics[];
  traces: TraceInfo[];
}

// REMOVED DUPLICATE TYPE DEFINITIONS - Now using centralized types from '../../types'

// PERFORMANCE: Utility functions outside component to prevent recreating
const inferServiceType = (name: string): ServiceNode['type'] => {
  if (name.includes('db') || name.includes('postgres') || name.includes('mysql')) return 'database';
  if (name.includes('redis') || name.includes('cache')) return 'cache';
  if (name.includes('api') || name.includes('external')) return 'external';
  return 'service';
};

const getServiceIcon = (type: ServiceNode['type']): string => {
  switch (type) {
    case 'database': return 'DB';
    case 'cache': return 'C';
    case 'external': return 'E';
    default: return 'S';
  }
};

export default function ServiceGraph({ services, traces }: ServiceGraphProps) {
  const svgRef = useRef<SVGSVGElement>(null);
  const [selectedNode, setSelectedNode] = useState<string | null>(null);
  const [hoveredNode, setHoveredNode] = useState<string | null>(null);

  // PERFORMANCE: Memoize expensive data transformations
  const graphData = useMemo(() => {
    // Fast path: early return if no data
    if (!services.length) return { nodes: [], links: [] };

    const nodes: ServiceNode[] = services.map(service => ({
      id: service.name,
      name: service.name,
      requestRate: service.request_rate,
      errorRate: service.error_rate,
      latencyP95: service.latency_p95,
      health: service.error_rate > 5 ? 'critical' : service.error_rate > 1 ? 'degraded' : 'healthy',
      type: inferServiceType(service.name)
    }));

    // Extract service relationships from traces - optimized for performance
    const linkMap = new Map<string, ServiceLink>();
    
    for (const trace of traces) {
      if (trace.services?.length > 1) {
        for (let i = 0; i < trace.services.length - 1; i++) {
          const key = `${trace.services[i]}->${trace.services[i + 1]}`;
          const existing = linkMap.get(key);
          
          if (existing) {
            existing.requestRate++;
            if (trace.has_error) existing.errorRate++;
          } else {
            linkMap.set(key, {
              source: trace.services[i],
              target: trace.services[i + 1],
              requestRate: 1,
              errorRate: trace.has_error ? 1 : 0,
              latency: trace.duration / trace.services.length
            });
          }
        }
      }
    }

    return {
      nodes,
      links: Array.from(linkMap.values())
    };
  }, [services, traces]);

  // PERFORMANCE: Stable drag handlers with proper typing
  const dragHandlers = useMemo(() => {
    const dragstarted = (event: d3.D3DragEvent<SVGGElement, ServiceNode, unknown>, d: ServiceNode & d3.SimulationNodeDatum, simulation: d3.Simulation<ServiceNode, undefined>) => {
      if (!event.active) simulation.alphaTarget(0.3).restart();
      d.fx = d.x;
      d.fy = d.y;
    };

    const dragged = (event: d3.D3DragEvent<SVGGElement, ServiceNode, unknown>, d: ServiceNode & d3.SimulationNodeDatum) => {
      d.fx = event.x;
      d.fy = event.y;
    };

    const dragended = (event: d3.D3DragEvent<SVGGElement, ServiceNode, unknown>, d: ServiceNode & d3.SimulationNodeDatum, simulation: d3.Simulation<ServiceNode, undefined>) => {
      if (!event.active) simulation.alphaTarget(0);
      d.fx = null;
      d.fy = null;
    };

    return { dragstarted, dragged, dragended };
  }, []);

  // PERFORMANCE: Memoized color functions
  const getNodeColor = useCallback((health: ServiceNode['health']) => {
    switch (health) {
      case 'critical': return '#dc2626';
      case 'degraded': return '#f97316';
      default: return '#6B7280';
    }
  }, []);

  const getLinkColor = useCallback((errorRate: number) => {
    if (errorRate > 0.5) return '#ef4444';
    if (errorRate > 0.1) return '#f59e0b';
    return '#6B7280';
  }, []);

  // PERFORMANCE: Extract D3 setup into separate effects for better control
  useEffect(() => {
    if (!svgRef.current || graphData.nodes.length === 0) return;

    const svg = d3.select(svgRef.current);
    const width = svgRef.current.clientWidth;
    const height = svgRef.current.clientHeight;

    // Clear previous graph - efficient cleanup
    svg.selectAll('*').remove();

    // Create container with zoom
    const g = svg.append('g');
    const zoom = d3.zoom()
      .scaleExtent([0.5, 3])
      .on('zoom', (event) => {
        g.attr('transform', event.transform);
      });

    svg.call(zoom as any);

    // Create optimized force simulation with proper types
    const simulation = d3.forceSimulation<ServiceNode>(graphData.nodes)
      .force('link', d3.forceLink<ServiceNode, ServiceLink>(graphData.links)
        .id((d) => d.id)
        .distance(150)
        .strength(0.5))
      .force('charge', d3.forceManyBody<ServiceNode>().strength(-500))
      .force('center', d3.forceCenter(width / 2, height / 2))
      .force('collision', d3.forceCollide<ServiceNode>().radius(45));

    // Arrow markers
    const defs = svg.append('defs');
    ['healthy', 'degraded', 'critical'].forEach(status => {
      defs.append('marker')
        .attr('id', `arrow-${status}`)
        .attr('viewBox', '0 -5 10 10')
        .attr('refX', 25)
        .attr('refY', 0)
        .attr('markerWidth', 6)
        .attr('markerHeight', 6)
        .attr('orient', 'auto')
        .append('path')
        .attr('d', 'M0,-5L10,0L0,5')
        .attr('fill', 
          status === 'critical' ? '#ef4444' : 
          status === 'degraded' ? '#f59e0b' : '#6B7280'
        );
    });

    // Links with optimized styling
    const link = g.append('g')
      .attr('class', 'links')
      .selectAll('line')
      .data(graphData.links)
      .enter().append('line')
      .attr('stroke', d => getLinkColor(d.errorRate))
      .attr('stroke-opacity', 0.7)
      .attr('stroke-width', d => Math.max(1, Math.min(5, d.requestRate / 10)))
      .attr('marker-end', d => 
        `url(#arrow-${d.errorRate > 0.5 ? 'critical' : d.errorRate > 0.1 ? 'degraded' : 'healthy'})`
      );

    // Node groups with event handlers
    const node = g.append('g')
      .attr('class', 'nodes')
      .selectAll('g')
      .data(graphData.nodes)
      .enter().append('g')
      .attr('cursor', 'pointer')
      .on('click', (event, d) => setSelectedNode(d.id))
      .on('mouseenter', (event, d) => setHoveredNode(d.id))
      .on('mouseleave', () => setHoveredNode(null))
      .call(d3.drag()
        .on('start', (event, d) => dragHandlers.dragstarted(event, d, simulation))
        .on('drag', dragHandlers.dragged)
        .on('end', (event, d) => dragHandlers.dragended(event, d, simulation)) as any);

    // Node circles
    node.append('circle')
      .attr('r', d => 20 + Math.min(20, d.requestRate / 100))
      .attr('fill', d => getNodeColor(d.health))
      .attr('stroke', '#ffffff')
      .attr('stroke-width', 2);

    // Service type icons
    node.append('text')
      .attr('font-family', 'monospace')
      .attr('font-size', '12px')
      .attr('fill', 'white')
      .attr('text-anchor', 'middle')
      .attr('dominant-baseline', 'middle')
      .text(d => getServiceIcon(d.type));

    // Node labels
    node.append('text')
      .attr('x', 0)
      .attr('y', d => 35 + Math.min(20, d.requestRate / 100))
      .attr('text-anchor', 'middle')
      .attr('fill', '#e5e7eb')
      .attr('font-size', '12px')
      .attr('font-weight', '500')
      .text(d => d.name);

    // Performance metrics (show on hover)
    const hoverText = node.append('text')
      .attr('x', 0)
      .attr('y', d => 50 + Math.min(20, d.requestRate / 100))
      .attr('text-anchor', 'middle')
      .attr('fill', '#9ca3af')
      .attr('font-size', '10px')
      .attr('opacity', 0)
      .text(d => `${d.requestRate.toFixed(0)} req/s`);

    // Simulation tick handler with proper types
    simulation.on('tick', () => {
      link
        .attr('x1', (d) => (d.source as ServiceNode & d3.SimulationNodeDatum).x || 0)
        .attr('y1', (d) => (d.source as ServiceNode & d3.SimulationNodeDatum).y || 0)
        .attr('x2', (d) => (d.target as ServiceNode & d3.SimulationNodeDatum).x || 0)
        .attr('y2', (d) => (d.target as ServiceNode & d3.SimulationNodeDatum).y || 0);

      node.attr('transform', (d) => `translate(${d.x || 0},${d.y || 0})`);
    });

    // CRITICAL: Proper cleanup to prevent memory leaks
    return () => {
      simulation.stop();
      simulation.on('tick', null);
    };
  }, [graphData, dragHandlers, getNodeColor, getLinkColor]);

  // PERFORMANCE: Separate effect for hover state updates
  useEffect(() => {
    if (!svgRef.current) return;

    d3.select(svgRef.current)
      .selectAll('.nodes text:last-child')
      .transition()
      .duration(150)
      .attr('opacity', (d: ServiceNode) => hoveredNode === d.id ? 1 : 0);
  }, [hoveredNode]);

  // PERFORMANCE: Memoize selected service lookup
  const selectedService = useMemo(
    () => graphData.nodes.find(n => n.id === selectedNode),
    [graphData.nodes, selectedNode]
  );

  return (
    <div className="relative h-full bg-surface-50 border border-surface-300 rounded-lg overflow-hidden">
      {/* Header */}
      <div className="absolute top-0 left-0 right-0 z-10 bg-gradient-to-b from-surface-50 to-transparent p-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Network className="w-5 h-5 text-text-700" />
            <h2 className="text-lg font-semibold text-text-900">Service Dependency Map</h2>
            <span className="text-xs text-text-500">
              {graphData.nodes.length} services, {graphData.links.length} trace paths
            </span>
          </div>
          
          <div className="flex items-center gap-4">
            {/* Legend */}
            <div className="flex items-center gap-3 text-xs">
              <div className="flex items-center gap-1">
                <div className="w-3 h-3 rounded-full bg-surface-400"></div>
                <span className="text-text-500">Healthy</span>
              </div>
              <div className="flex items-center gap-1">
                <div className="w-3 h-3 rounded-full bg-amber-500"></div>
                <span className="text-text-500">Degraded</span>
              </div>
              <div className="flex items-center gap-1">
                <div className="w-3 h-3 rounded-full bg-red-500"></div>
                <span className="text-text-500">Critical</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Graph SVG */}
      <svg 
        ref={svgRef} 
        className="w-full h-full"
        style={{ background: '#FAFAFA' }}
      />

      {/* Selected Service Details */}
      {selectedService && (
        <div className="absolute top-20 right-4 w-80 bg-surface-50 border border-surface-300 rounded-lg p-4 shadow-xl"
        >
          <div className="flex items-center justify-between mb-3">
            <h3 className="text-text-900 font-semibold flex items-center gap-2">
              {selectedService.health === 'healthy' ? (
                <CheckCircle className="w-4 h-4 text-text-500" />
              ) : selectedService.health === 'degraded' ? (
                <Clock className="w-4 h-4 text-amber-500" />
              ) : (
                <AlertCircle className="w-4 h-4 text-red-500" />
              )}
              {selectedService.name}
            </h3>
            <button
              onClick={() => setSelectedNode(null)}
              className="text-text-500 hover:text-text-900"
            >
              ✕
            </button>
          </div>

          <div className="space-y-2">
            <div className="flex justify-between text-sm">
              <span className="text-text-500">Request Rate</span>
              <span className="text-text-900">{selectedService.requestRate.toFixed(2)} req/s</span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-text-500">Error Rate</span>
              <span className={`${selectedService.errorRate > 5 ? 'text-red-400' : 'text-white'}`}>
                {selectedService.errorRate.toFixed(2)}%
              </span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-text-500">P95 Latency</span>
              <span className="text-text-900">{selectedService.latencyP95}ms</span>
            </div>
          </div>

          <div className="mt-4 pt-4 border-t border-surface-300">
            <h4 className="text-sm font-medium text-text-500 mb-2">Connected Services</h4>
            <div className="space-y-1">
              {graphData.links
                .filter(l => l.source === selectedService.id || l.target === selectedService.id)
                .map((link, idx) => (
                  <div key={idx} className="text-xs text-text-500">
                    {link.source === selectedService.id ? '→' : '←'} {
                      link.source === selectedService.id ? link.target : link.source
                    } ({link.requestRate} req/s)
                  </div>
                ))}
            </div>
          </div>
        </div>
      )}

      {/* Trace Animation Overlay */}
      <div className="absolute bottom-4 left-4 flex items-center gap-2">
        <Activity className="w-4 h-4 text-text-700" />
        <span className="text-xs text-text-500">Live Trace Visualization</span>
      </div>
    </div>
  );
}