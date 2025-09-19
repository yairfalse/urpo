import { ServiceNode, ServiceEdge } from '../../hooks/useDependencyDiscovery';

export interface RenderOptions {
  showMetrics: boolean;
  selectedService: string | null;
  hoveredService: string | null;
  theme: {
    nodeRadius: number;
    fontSize: number;
    colors: {
      node: string;
      nodeSelected: string;
      nodeHovered: string;
      edge: string;
      text: string;
      background: string;
    };
  };
}

export function renderGraph(
  ctx: CanvasRenderingContext2D,
  nodes: Map<string, ServiceNode>,
  edges: ServiceEdge[],
  options: RenderOptions
): void {
  const { width, height } = ctx.canvas;
  
  // Clear canvas
  ctx.fillStyle = options.theme.colors.background;
  ctx.fillRect(0, 0, width, height);
  
  // Draw edges
  ctx.strokeStyle = options.theme.colors.edge;
  ctx.lineWidth = 1;
  
  edges.forEach(edge => {
    const source = nodes.get(edge.source);
    const target = nodes.get(edge.target);
    if (!source || !target) return;
    
    ctx.globalAlpha = 0.3 + edge.strength * 0.4;
    ctx.beginPath();
    ctx.moveTo(source.x, source.y);
    ctx.lineTo(target.x, target.y);
    ctx.stroke();
    
    // Draw arrow
    const angle = Math.atan2(target.y - source.y, target.x - source.x);
    const arrowLength = 10;
    const arrowAngle = Math.PI / 6;
    
    ctx.beginPath();
    ctx.moveTo(target.x - options.theme.nodeRadius * Math.cos(angle), 
               target.y - options.theme.nodeRadius * Math.sin(angle));
    ctx.lineTo(
      target.x - options.theme.nodeRadius * Math.cos(angle) - arrowLength * Math.cos(angle - arrowAngle),
      target.y - options.theme.nodeRadius * Math.sin(angle) - arrowLength * Math.sin(angle - arrowAngle)
    );
    ctx.moveTo(target.x - options.theme.nodeRadius * Math.cos(angle),
               target.y - options.theme.nodeRadius * Math.sin(angle));
    ctx.lineTo(
      target.x - options.theme.nodeRadius * Math.cos(angle) - arrowLength * Math.cos(angle + arrowAngle),
      target.y - options.theme.nodeRadius * Math.sin(angle) - arrowLength * Math.sin(angle + arrowAngle)
    );
    ctx.stroke();
  });
  
  ctx.globalAlpha = 1;
  
  // Draw nodes
  nodes.forEach(node => {
    const isSelected = node.id === options.selectedService;
    const isHovered = node.id === options.hoveredService;
    
    // Node circle
    ctx.fillStyle = isSelected ? options.theme.colors.nodeSelected :
                   isHovered ? options.theme.colors.nodeHovered :
                   options.theme.colors.node;
    
    ctx.beginPath();
    ctx.arc(node.x, node.y, options.theme.nodeRadius, 0, Math.PI * 2);
    ctx.fill();
    
    if (isSelected || isHovered) {
      ctx.strokeStyle = '#111827';
      ctx.lineWidth = 2;
      ctx.stroke();
    }
    
    // Node label
    ctx.fillStyle = options.theme.colors.text;
    ctx.font = `${options.theme.fontSize}px Inter, sans-serif`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(node.name, node.x, node.y);
    
    // Metrics
    if (options.showMetrics) {
      ctx.font = `${options.theme.fontSize - 2}px Inter, sans-serif`;
      ctx.fillStyle = '#6B7280';
      ctx.fillText(
        `${node.metrics.requestRate.toFixed(0)} req/s`,
        node.x,
        node.y + options.theme.nodeRadius + 15
      );
      
      if (node.metrics.errorRate > 0) {
        ctx.fillStyle = '#EF4444';
        ctx.fillText(
          `${node.metrics.errorRate.toFixed(1)}% errors`,
          node.x,
          node.y + options.theme.nodeRadius + 30
        );
      }
    }
  });
}

export function getNodeAtPosition(
  nodes: Map<string, ServiceNode>,
  x: number,
  y: number,
  radius: number
): string | null {
  for (const [id, node] of nodes) {
    const dx = node.x - x;
    const dy = node.y - y;
    if (Math.sqrt(dx * dx + dy * dy) <= radius) {
      return id;
    }
  }
  return null;
}