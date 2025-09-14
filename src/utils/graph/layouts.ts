import { ServiceNode } from '../../hooks/useDependencyDiscovery';

export type LayoutMode = 'force' | 'hierarchical' | 'circular';

export function applyForceLayout(
  nodes: Map<string, ServiceNode>,
  edges: Array<{ source: string; target: string; strength: number }>,
  width: number,
  height: number
): void {
  const nodeArray = Array.from(nodes.values());
  const iterations = 50;
  const alpha = 0.1;
  const repulsion = 50000;
  const attraction = 0.001;
  
  for (let iter = 0; iter < iterations; iter++) {
    // Apply repulsive forces between all nodes
    for (let i = 0; i < nodeArray.length; i++) {
      for (let j = i + 1; j < nodeArray.length; j++) {
        const dx = nodeArray[j].x - nodeArray[i].x;
        const dy = nodeArray[j].y - nodeArray[i].y;
        const dist = Math.max(Math.sqrt(dx * dx + dy * dy), 1);
        const force = repulsion / (dist * dist);
        
        const fx = (dx / dist) * force * alpha;
        const fy = (dy / dist) * force * alpha;
        
        if (!nodeArray[i].pinned) {
          nodeArray[i].vx -= fx;
          nodeArray[i].vy -= fy;
        }
        if (!nodeArray[j].pinned) {
          nodeArray[j].vx += fx;
          nodeArray[j].vy += fy;
        }
      }
    }
    
    // Apply attractive forces along edges
    edges.forEach(edge => {
      const source = nodes.get(edge.source);
      const target = nodes.get(edge.target);
      if (!source || !target) return;
      
      const dx = target.x - source.x;
      const dy = target.y - source.y;
      const dist = Math.sqrt(dx * dx + dy * dy);
      const force = dist * attraction * edge.strength;
      
      const fx = (dx / dist) * force * alpha;
      const fy = (dy / dist) * force * alpha;
      
      if (!source.pinned) {
        source.vx += fx;
        source.vy += fy;
      }
      if (!target.pinned) {
        target.vx -= fx;
        target.vy -= fy;
      }
    });
    
    // Apply velocities and damping
    nodeArray.forEach(node => {
      if (!node.pinned) {
        node.x += node.vx;
        node.y += node.vy;
        node.vx *= 0.9; // damping
        node.vy *= 0.9;
        
        // Keep within bounds
        node.x = Math.max(50, Math.min(width - 50, node.x));
        node.y = Math.max(50, Math.min(height - 50, node.y));
      }
    });
  }
}

export function applyCircularLayout(
  nodes: Map<string, ServiceNode>,
  width: number,
  height: number
): void {
  const nodeArray = Array.from(nodes.values());
  const centerX = width / 2;
  const centerY = height / 2;
  const radius = Math.min(width, height) * 0.35;
  
  nodeArray.forEach((node, i) => {
    const angle = (i / nodeArray.length) * Math.PI * 2;
    node.x = centerX + Math.cos(angle) * radius;
    node.y = centerY + Math.sin(angle) * radius;
    node.vx = 0;
    node.vy = 0;
  });
}

export function applyHierarchicalLayout(
  nodes: Map<string, ServiceNode>,
  edges: Array<{ source: string; target: string }>,
  width: number,
  height: number
): void {
  // Find root nodes (no incoming edges)
  const hasIncoming = new Set<string>();
  edges.forEach(edge => hasIncoming.add(edge.target));
  
  const roots = Array.from(nodes.keys()).filter(id => !hasIncoming.has(id));
  const levels = new Map<string, number>();
  const visited = new Set<string>();
  
  // BFS to assign levels
  const queue = roots.map(id => ({ id, level: 0 }));
  while (queue.length > 0) {
    const { id, level } = queue.shift()!;
    if (visited.has(id)) continue;
    
    visited.add(id);
    levels.set(id, level);
    
    edges
      .filter(e => e.source === id)
      .forEach(e => queue.push({ id: e.target, level: level + 1 }));
  }
  
  // Group nodes by level
  const levelGroups = new Map<number, string[]>();
  levels.forEach((level, id) => {
    if (!levelGroups.has(level)) {
      levelGroups.set(level, []);
    }
    levelGroups.get(level)!.push(id);
  });
  
  // Position nodes
  const maxLevel = Math.max(...Array.from(levelGroups.keys()));
  levelGroups.forEach((nodeIds, level) => {
    const y = 100 + (level / maxLevel) * (height - 200);
    const spacing = width / (nodeIds.length + 1);
    
    nodeIds.forEach((id, i) => {
      const node = nodes.get(id);
      if (node) {
        node.x = spacing * (i + 1);
        node.y = y;
        node.vx = 0;
        node.vy = 0;
      }
    });
  });
}