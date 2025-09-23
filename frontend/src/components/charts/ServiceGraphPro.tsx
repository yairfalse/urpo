import { useEffect, useRef, useState, useCallback, useMemo, memo } from 'react';
import * as d3 from 'd3';
import { Activity, AlertCircle, CheckCircle, Clock, Database, Cloud, Server, Layers } from 'lucide-react';
import { ServiceMetrics, TraceInfo } from '../../types';

interface ServiceNode extends d3.SimulationNodeDatum {
  id: string;
  name: string;
  requestRate: number;
  errorRate: number;
  latencyP95: number;
  health: 'healthy' | 'degraded' | 'critical';
  type: 'service' | 'database' | 'cache' | 'external' | 'gateway';
  group: number;
}

interface ServiceLink extends d3.SimulationLinkDatum<ServiceNode> {
  source: string | ServiceNode;
  target: string | ServiceNode;
  requestRate: number;
  errorRate: number;
  latency: number;
  strength: number;
}

interface ServiceGraphProProps {
  services: ServiceMetrics[];
  traces: TraceInfo[];
}

// Professional color scheme inspired by Hubble UI
const COLORS = {
  node: {
    healthy: '#5AD8A6',
    degraded: '#FF9845',
    critical: '#F6465D',
    default: '#5B8FF9',
  },
  link: {
    normal: 'rgba(91, 143, 249, 0.3)',
    hover: 'rgba(91, 143, 249, 0.6)',
    error: 'rgba(246, 70, 93, 0.4)',
  },
  bg: {
    grid: '#1A2332',
    tooltip: '#111923',
  },
  text: {
    primary: '#F3F4F6',
    secondary: '#9CA3AF',
  },
};

const ServiceGraphPro = memo(({ services, traces }: ServiceGraphProProps) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const svgRef = useRef<SVGSVGElement>(null);
  const [dimensions, setDimensions] = useState({ width: 0, height: 0 });
  const [selectedNode, setSelectedNode] = useState<string | null>(null);
  const [hoveredNode, setHoveredNode] = useState<string | null>(null);
  const simulationRef = useRef<d3.Simulation<ServiceNode, ServiceLink> | null>(null);

  // Auto-detect service type based on name patterns
  const inferServiceType = (name: string): ServiceNode['type'] => {
    const lowerName = name.toLowerCase();
    if (lowerName.includes('gateway') || lowerName.includes('proxy')) return 'gateway';
    if (lowerName.includes('db') || lowerName.includes('postgres') || lowerName.includes('mysql')) return 'database';
    if (lowerName.includes('redis') || lowerName.includes('cache')) return 'cache';
    if (lowerName.includes('external') || lowerName.includes('api')) return 'external';
    return 'service';
  };

  // Get icon for service type
  const getServiceIcon = (type: ServiceNode['type']) => {
    switch (type) {
      case 'gateway': return Cloud;
      case 'database': return Database;
      case 'cache': return Layers;
      case 'external': return Cloud;
      default: return Server;
    }
  };

  // Process graph data
  const graphData = useMemo(() => {
    if (!services.length) return { nodes: [], links: [] };

    // Create nodes with groups for better layout
    const nodes: ServiceNode[] = services.map((service, idx) => ({
      id: service.name,
      name: service.name,
      requestRate: service.request_rate,
      errorRate: service.error_rate,
      latencyP95: service.latency_p95,
      health: service.error_rate > 5 ? 'critical' : service.error_rate > 1 ? 'degraded' : 'healthy',
      type: inferServiceType(service.name),
      group: Math.floor(idx / 3), // Group nodes for better clustering
    }));

    // Extract relationships from traces
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
              latency: trace.duration / trace.services.length,
              strength: 1,
            });
          }
        }
      }
    }

    const links = Array.from(linkMap.values());

    // Normalize link strength for better visualization
    const maxRequests = Math.max(...links.map(l => l.requestRate), 1);
    links.forEach(link => {
      link.strength = (link.requestRate / maxRequests) * 10 + 1;
    });

    return { nodes, links };
  }, [services, traces]);

  // Handle resize
  useEffect(() => {
    const handleResize = () => {
      if (containerRef.current) {
        const { clientWidth, clientHeight } = containerRef.current;
        setDimensions({ width: clientWidth, height: clientHeight });
      }
    };

    handleResize();
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  // Main D3 visualization
  useEffect(() => {
    if (!svgRef.current || dimensions.width === 0) return;

    const { nodes, links } = graphData;
    if (!nodes.length) return;

    // Clear previous content
    d3.select(svgRef.current).selectAll('*').remove();

    const svg = d3.select(svgRef.current);
    const { width, height } = dimensions;

    // Create gradient definitions for links
    const defs = svg.append('defs');

    // Add glow filter for nodes
    const filter = defs.append('filter')
      .attr('id', 'glow')
      .attr('x', '-50%')
      .attr('y', '-50%')
      .attr('width', '200%')
      .attr('height', '200%');

    filter.append('feGaussianBlur')
      .attr('stdDeviation', '3')
      .attr('result', 'coloredBlur');

    const feMerge = filter.append('feMerge');
    feMerge.append('feMergeNode').attr('in', 'coloredBlur');
    feMerge.append('feMergeNode').attr('in', 'SourceGraphic');

    // Create container groups
    const g = svg.append('g');

    // Add zoom behavior
    const zoom = d3.zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.5, 3])
      .on('zoom', (event) => {
        g.attr('transform', event.transform);
      });

    svg.call(zoom);

    // Create force simulation
    const simulation = d3.forceSimulation<ServiceNode>(nodes)
      .force('link', d3.forceLink<ServiceNode, ServiceLink>(links)
        .id((d) => d.id)
        .distance((d) => 150 / (d.strength || 1))
        .strength((d) => Math.min(0.5, d.strength / 10)))
      .force('charge', d3.forceManyBody().strength(-500))
      .force('center', d3.forceCenter(width / 2, height / 2))
      .force('collision', d3.forceCollide().radius(40))
      .force('x', d3.forceX(width / 2).strength(0.05))
      .force('y', d3.forceY(height / 2).strength(0.05));

    simulationRef.current = simulation;

    // Create link elements with animated gradients
    const linkGroup = g.append('g').attr('class', 'links');

    const link = linkGroup.selectAll('line')
      .data(links)
      .enter()
      .append('g');

    // Draw links as paths for better curves
    const linkPath = link.append('path')
      .attr('class', 'link-path')
      .attr('stroke', (d) => d.errorRate > 0 ? COLORS.link.error : COLORS.link.normal)
      .attr('stroke-width', (d) => Math.max(1, Math.min(8, d.strength)))
      .attr('fill', 'none')
      .attr('opacity', 0.6);

    // Add animated particles on links for traffic visualization
    link.each(function(d: ServiceLink) {
      const group = d3.select(this);
      if (d.requestRate > 10) {
        group.append('circle')
          .attr('class', 'traffic-particle')
          .attr('r', 2)
          .attr('fill', '#5DCFFF')
          .append('animateMotion')
          .attr('dur', `${3 / (d.requestRate / 100)}s`)
          .attr('repeatCount', 'indefinite')
          .append('mpath')
          .attr('href', '#link-' + d.source + '-' + d.target);
      }
    });

    // Create node groups
    const nodeGroup = g.append('g').attr('class', 'nodes');

    const node = nodeGroup.selectAll('g')
      .data(nodes)
      .enter()
      .append('g')
      .attr('class', 'node-group')
      .style('cursor', 'pointer');

    // Add node backgrounds with health indicators
    node.append('circle')
      .attr('r', (d) => 25 + Math.sqrt(d.requestRate))
      .attr('fill', (d) => COLORS.node[d.health])
      .attr('stroke', (d) => d === hoveredNode ? '#fff' : 'transparent')
      .attr('stroke-width', 2)
      .attr('filter', 'url(#glow)')
      .attr('opacity', 0.9);

    // Add inner circle for service type
    node.append('circle')
      .attr('r', 20)
      .attr('fill', '#1A2332')
      .attr('opacity', 0.8);

    // Add service type icons
    node.append('text')
      .attr('text-anchor', 'middle')
      .attr('dominant-baseline', 'middle')
      .attr('fill', COLORS.text.primary)
      .attr('font-size', '16px')
      .attr('font-family', 'SF Mono, monospace')
      .text((d) => {
        const Icon = getServiceIcon(d.type);
        return d.type === 'database' ? '⚡' :
               d.type === 'cache' ? '💾' :
               d.type === 'gateway' ? '🌐' :
               d.type === 'external' ? '☁️' : '⚙️';
      });

    // Add service names
    node.append('text')
      .attr('y', 40)
      .attr('text-anchor', 'middle')
      .attr('fill', COLORS.text.primary)
      .attr('font-size', '12px')
      .attr('font-weight', '500')
      .text((d) => d.name);

    // Add metrics badges
    node.each(function(d: ServiceNode) {
      const group = d3.select(this);

      // Error rate badge
      if (d.errorRate > 0) {
        const errorBadge = group.append('g')
          .attr('transform', 'translate(20, -20)');

        errorBadge.append('circle')
          .attr('r', 8)
          .attr('fill', COLORS.node.critical);

        errorBadge.append('text')
          .attr('text-anchor', 'middle')
          .attr('dominant-baseline', 'middle')
          .attr('fill', 'white')
          .attr('font-size', '10px')
          .attr('font-weight', 'bold')
          .text(d.errorRate.toFixed(0) + '%');
      }

      // Request rate indicator
      if (d.requestRate > 100) {
        const rpsRing = group.append('circle')
          .attr('r', 30 + Math.sqrt(d.requestRate))
          .attr('fill', 'none')
          .attr('stroke', COLORS.node.default)
          .attr('stroke-width', 1)
          .attr('opacity', 0.3)
          .attr('stroke-dasharray', '2,2')
          .append('animate')
          .attr('attributeName', 'stroke-dashoffset')
          .attr('dur', '10s')
          .attr('repeatCount', 'indefinite')
          .attr('from', '0')
          .attr('to', '100');
      }
    });

    // Drag behavior
    const drag = d3.drag<SVGGElement, ServiceNode>()
      .on('start', (event, d) => {
        if (!event.active) simulation.alphaTarget(0.3).restart();
        d.fx = d.x;
        d.fy = d.y;
      })
      .on('drag', (event, d) => {
        d.fx = event.x;
        d.fy = event.y;
      })
      .on('end', (event, d) => {
        if (!event.active) simulation.alphaTarget(0);
        d.fx = null;
        d.fy = null;
      });

    node.call(drag);

    // Hover interactions
    node
      .on('mouseenter', function(event, d) {
        setHoveredNode(d.id);
        d3.select(this).select('circle').attr('stroke', '#fff');

        // Highlight connected links
        linkPath.attr('opacity', (l: any) => {
          if (typeof l.source === 'object' && typeof l.target === 'object') {
            return l.source.id === d.id || l.target.id === d.id ? 1 : 0.2;
          }
          return 0.2;
        });
      })
      .on('mouseleave', function() {
        setHoveredNode(null);
        d3.select(this).select('circle').attr('stroke', 'transparent');
        linkPath.attr('opacity', 0.6);
      })
      .on('click', (event, d) => {
        setSelectedNode(d.id === selectedNode ? null : d.id);
      });

    // Update positions on simulation tick
    simulation.on('tick', () => {
      // Update link positions with curves
      linkPath.attr('d', (d: any) => {
        const dx = d.target.x - d.source.x;
        const dy = d.target.y - d.source.y;
        const dr = Math.sqrt(dx * dx + dy * dy) * 0.5;
        return `M${d.source.x},${d.source.y}A${dr},${dr} 0 0,1 ${d.target.x},${d.target.y}`;
      });

      // Update node positions
      node.attr('transform', (d) => `translate(${d.x},${d.y})`);
    });

    // Cleanup
    return () => {
      simulation.stop();
    };
  }, [graphData, dimensions, hoveredNode, selectedNode]);

  return (
    <div className="relative h-full bg-dark-50 rounded-lg border border-dark-300" ref={containerRef}>
      {/* Header */}
      <div className="absolute top-0 left-0 right-0 z-10 p-4 bg-gradient-to-b from-dark-50 to-transparent">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-lg font-semibold text-light-50">Service Dependency Graph</h2>
            <p className="text-sm text-light-400 mt-1">
              {graphData.nodes.length} services, {graphData.links.length} dependencies
            </p>
          </div>

          {/* Legend */}
          <div className="flex items-center gap-4 bg-dark-100 rounded-lg px-3 py-2">
            <div className="flex items-center gap-2">
              <div className="w-3 h-3 rounded-full bg-semantic-success"></div>
              <span className="text-xs text-light-400">Healthy</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-3 h-3 rounded-full bg-semantic-warning"></div>
              <span className="text-xs text-light-400">Degraded</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-3 h-3 rounded-full bg-semantic-error"></div>
              <span className="text-xs text-light-400">Critical</span>
            </div>
          </div>
        </div>
      </div>

      {/* SVG Container */}
      <svg
        ref={svgRef}
        width={dimensions.width}
        height={dimensions.height}
        className="w-full h-full"
      />

      {/* Selected Node Details */}
      {selectedNode && (
        <div className="absolute bottom-4 left-4 bg-dark-100 border border-dark-300 rounded-lg p-4 max-w-sm">
          <h3 className="text-sm font-semibold text-light-50 mb-2">{selectedNode}</h3>
          <div className="space-y-1">
            {graphData.nodes
              .filter(n => n.id === selectedNode)
              .map(node => (
                <div key={node.id} className="text-xs text-light-400">
                  <div>RPS: {node.requestRate.toFixed(0)}</div>
                  <div>Error Rate: {node.errorRate.toFixed(2)}%</div>
                  <div>P95 Latency: {node.latencyP95}ms</div>
                </div>
              ))}
          </div>
        </div>
      )}
    </div>
  );
});

ServiceGraphPro.displayName = 'ServiceGraphPro';

export { ServiceGraphPro };