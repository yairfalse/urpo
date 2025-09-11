import { useEffect, useRef, useState } from 'react';
import * as d3 from 'd3';
import { motion } from 'framer-motion';
import { Activity, AlertCircle, CheckCircle, Clock, Network } from 'lucide-react';

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
  services: any[];
  traces: any[];
}

export default function ServiceGraph({ services, traces }: ServiceGraphProps) {
  const svgRef = useRef<SVGSVGElement>(null);
  const [selectedNode, setSelectedNode] = useState<string | null>(null);
  const [hoveredNode, setHoveredNode] = useState<string | null>(null);
  const [graphData, setGraphData] = useState<{ nodes: ServiceNode[], links: ServiceLink[] }>({ 
    nodes: [], 
    links: [] 
  });

  // Build graph data from services and traces
  useEffect(() => {
    const nodes: ServiceNode[] = services.map(service => ({
      id: service.name,
      name: service.name,
      requestRate: service.request_rate,
      errorRate: service.error_rate,
      latencyP95: service.latency_p95,
      health: service.error_rate > 5 ? 'critical' : service.error_rate > 1 ? 'degraded' : 'healthy',
      type: inferServiceType(service.name)
    }));

    // Extract service relationships from traces
    const linkMap = new Map<string, ServiceLink>();
    
    traces.forEach(trace => {
      if (trace.services && trace.services.length > 1) {
        for (let i = 0; i < trace.services.length - 1; i++) {
          const key = `${trace.services[i]}->${trace.services[i + 1]}`;
          const existing = linkMap.get(key);
          
          if (existing) {
            existing.requestRate += 1;
            if (trace.has_error) existing.errorRate += 1;
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
    });

    setGraphData({
      nodes,
      links: Array.from(linkMap.values())
    });
  }, [services, traces]);

  // D3 Force Simulation
  useEffect(() => {
    if (!svgRef.current || graphData.nodes.length === 0) return;

    const width = svgRef.current.clientWidth;
    const height = svgRef.current.clientHeight;

    // Clear previous graph
    d3.select(svgRef.current).selectAll('*').remove();

    const svg = d3.select(svgRef.current);
    
    // Create container groups
    const g = svg.append('g');
    
    // Add zoom behavior
    const zoom = (d3 as any).zoom()
      .scaleExtent([0.5, 3])
      .on('zoom', (event: any) => {
        g.attr('transform', event.transform);
      });

    svg.call(zoom as any);

    // Create force simulation
    const simulation = (d3 as any).forceSimulation(graphData.nodes as any)
      .force('link', (d3 as any).forceLink(graphData.links)
        .id((d: any) => d.id)
        .distance(150))
      .force('charge', (d3 as any).forceManyBody().strength(-500))
      .force('center', (d3 as any).forceCenter(width / 2, height / 2))
      .force('collision', (d3 as any).forceCollide().radius(40));

    // Create arrow markers for directed edges
    svg.append('defs').selectAll('marker')
      .data(['healthy', 'degraded', 'critical'])
      .enter().append('marker')
      .attr('id', d => `arrow-${d}`)
      .attr('viewBox', '0 -5 10 10')
      .attr('refX', 25)
      .attr('refY', 0)
      .attr('markerWidth', 6)
      .attr('markerHeight', 6)
      .attr('orient', 'auto')
      .append('path')
      .attr('d', 'M0,-5L10,0L0,5')
      .attr('fill', d => 
        d === 'critical' ? '#ef4444' : 
        d === 'degraded' ? '#f59e0b' : 
        '#6B7280'
      );

    // Create links
    const link = g.append('g')
      .selectAll('line')
      .data(graphData.links)
      .enter().append('line')
      .attr('stroke', d => 
        d.errorRate > 0.5 ? '#ef4444' : 
        d.errorRate > 0.1 ? '#f59e0b' : 
        '#6B7280'
      )
      .attr('stroke-opacity', 0.6)
      .attr('stroke-width', d => Math.max(1, Math.min(5, d.requestRate / 10)))
      .attr('marker-end', d => 
        `url(#arrow-${d.errorRate > 0.5 ? 'critical' : d.errorRate > 0.1 ? 'degraded' : 'healthy'})`
      );

    // Create node groups
    const node = g.append('g')
      .selectAll('g')
      .data(graphData.nodes)
      .enter().append('g')
      .attr('cursor', 'pointer')
      .on('click', (_event: any, d: any) => setSelectedNode(d.id))
      .on('mouseenter', (_event: any, d: any) => setHoveredNode(d.id))
      .on('mouseleave', () => setHoveredNode(null))
      .call((d3 as any).drag()
        .on('start', dragstarted)
        .on('drag', dragged)
        .on('end', dragended) as any);

    // Add circles for nodes
    node.append('circle')
      .attr('r', d => 20 + Math.min(20, d.requestRate / 100))
      .attr('fill', d => {
        if (d.health === 'critical') return '#dc2626';
        if (d.health === 'degraded') return '#f97316';
        return '#6B7280';
      })
      .attr('stroke', '#fff')
      .attr('stroke-width', 2);

    // Add icons for service types
    node.append('text')
      .attr('font-family', 'lucide')
      .attr('font-size', '16px')
      .attr('fill', 'white')
      .attr('text-anchor', 'middle')
      .attr('dy', '0.3em')
      .text(d => getServiceIcon(d.type));

    // Add labels
    node.append('text')
      .attr('x', 0)
      .attr('y', d => 30 + Math.min(20, d.requestRate / 100))
      .attr('text-anchor', 'middle')
      .attr('fill', '#e5e7eb')
      .attr('font-size', '12px')
      .text(d => d.name);

    // Add metrics on hover
    node.append('text')
      .attr('x', 0)
      .attr('y', d => 45 + Math.min(20, d.requestRate / 100))
      .attr('text-anchor', 'middle')
      .attr('fill', '#9ca3af')
      .attr('font-size', '10px')
      .attr('opacity', 0)
      .text(d => `${d.requestRate.toFixed(0)} req/s`)
      .transition()
      .duration(200)
      .attr('opacity', d => hoveredNode === d.id ? 1 : 0);

    // Update positions on simulation tick
    simulation.on('tick', () => {
      link
        .attr('x1', (d: any) => d.source.x)
        .attr('y1', (d: any) => d.source.y)
        .attr('x2', (d: any) => d.target.x)
        .attr('y2', (d: any) => d.target.y);

      node.attr('transform', (d: any) => `translate(${d.x},${d.y})`);
    });

    // Drag functions
    function dragstarted(event: any, d: any) {
      if (!event.active) simulation.alphaTarget(0.3).restart();
      d.fx = d.x;
      d.fy = d.y;
    }

    function dragged(event: any, d: any) {
      d.fx = event.x;
      d.fy = event.y;
    }

    function dragended(event: any, d: any) {
      if (!event.active) simulation.alphaTarget(0);
      d.fx = null;
      d.fy = null;
    }

    return () => {
      simulation.stop();
    };
  }, [graphData, hoveredNode]);

  const inferServiceType = (name: string): ServiceNode['type'] => {
    if (name.includes('db') || name.includes('postgres') || name.includes('mysql')) return 'database';
    if (name.includes('redis') || name.includes('cache')) return 'cache';
    if (name.includes('api') || name.includes('external')) return 'external';
    return 'service';
  };

  const getServiceIcon = (type: ServiceNode['type']) => {
    switch (type) {
      case 'database': return 'DB';
      case 'cache': return 'CACHE';
      case 'external': return 'EXT';
      default: return 'SVC';
    }
  };

  const selectedService = graphData.nodes.find(n => n.id === selectedNode);

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
                <div className="w-3 h-3 rounded-full bg-gray-500"></div>
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
        <motion.div
          initial={{ opacity: 0, x: 20 }}
          animate={{ opacity: 1, x: 0 }}
          className="absolute top-20 right-4 w-80 bg-surface-50 border border-surface-300 rounded-lg p-4 shadow-xl"
        >
          <div className="flex items-center justify-between mb-3">
            <h3 className="text-text-900 font-semibold flex items-center gap-2">
              {selectedService.health === 'healthy' ? (
                <CheckCircle className="w-4 h-4 text-gray-500" />
              ) : selectedService.health === 'degraded' ? (
                <Clock className="w-4 h-4 text-amber-500" />
              ) : (
                <AlertCircle className="w-4 h-4 text-red-500" />
              )}
              {selectedService.name}
            </h3>
            <button
              onClick={() => setSelectedNode(null)}
              className="text-slate-500 hover:text-white"
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
        </motion.div>
      )}

      {/* Trace Animation Overlay */}
      <div className="absolute bottom-4 left-4 flex items-center gap-2">
        <Activity className="w-4 h-4 text-text-700 animate-pulse" />
        <span className="text-xs text-text-500">Live Trace Visualization</span>
      </div>
    </div>
  );
}