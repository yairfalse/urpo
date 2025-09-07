// LIVE SERVICE MAP - WATCH YOUR SYSTEM BREATHE
import React, { useRef, useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/tauri';

interface ServiceNode {
  id: string;
  name: string;
  x: number;
  y: number;
  rps: number;  // requests per second
  errorRate: number;
  p95Latency: number;
  healthy: boolean;
  connections: string[];  // connected service IDs
  activeTraces: number;
}

interface TraceFlow {
  from: string;
  to: string;
  intensity: number;
  offset: number;
}

const LiveServiceMap = () => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animationRef = useRef<number>();
  const [services, setServices] = useState<Map<string, ServiceNode>>(new Map());
  const [flows, setFlows] = useState<TraceFlow[]>([]);
  const [hoveredService, setHoveredService] = useState<string | null>(null);
  
  // Auto-layout services in a force-directed graph
  const layoutServices = useCallback((serviceData: any[]) => {
    const nodes = new Map<string, ServiceNode>();
    const centerX = 800;
    const centerY = 400;
    const radius = 250;
    
    serviceData.forEach((service, index) => {
      const angle = (index / serviceData.length) * Math.PI * 2;
      nodes.set(service.name, {
        id: service.name,
        name: service.name,
        x: centerX + Math.cos(angle) * radius,
        y: centerY + Math.sin(angle) * radius,
        rps: service.request_rate || 0,
        errorRate: service.error_rate || 0,
        p95Latency: service.latency_p95 || 0,
        healthy: service.error_rate < 0.01,
        connections: service.dependencies || [],
        activeTraces: Math.floor(service.request_rate * service.latency_p95 / 1000),
      });
    });
    
    return nodes;
  }, []);

  // Fetch live metrics
  const updateMetrics = useCallback(async () => {
    try {
      const metrics = await invoke('get_service_metrics');
      const newServices = layoutServices(metrics as any[]);
      setServices(newServices);
      
      // Create flows between services
      const newFlows: TraceFlow[] = [];
      newServices.forEach((service) => {
        service.connections.forEach(targetId => {
          if (newServices.has(targetId)) {
            newFlows.push({
              from: service.id,
              to: targetId,
              intensity: service.rps / 10,
              offset: 0,
            });
          }
        });
      });
      setFlows(newFlows);
    } catch (error) {
      console.error('Failed to fetch metrics:', error);
    }
  }, [layoutServices]);

  // ANIMATION LOOP - 60FPS SMOOTH
  const animate = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    
    // Clear with dark background
    ctx.fillStyle = '#0a0a0a';
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    
    // Update flow animations
    flows.forEach(flow => {
      flow.offset += 0.02 * flow.intensity;
      if (flow.offset > 1) flow.offset = 0;
    });
    
    // Draw connections with flowing particles
    ctx.strokeStyle = '#1a1a1a';
    ctx.lineWidth = 2;
    
    flows.forEach(flow => {
      const from = services.get(flow.from);
      const to = services.get(flow.to);
      if (!from || !to) return;
      
      // Draw the connection line
      ctx.beginPath();
      ctx.moveTo(from.x, from.y);
      ctx.lineTo(to.x, to.y);
      ctx.stroke();
      
      // Draw flowing particles
      const particleX = from.x + (to.x - from.x) * flow.offset;
      const particleY = from.y + (to.y - from.y) * flow.offset;
      
      ctx.fillStyle = from.healthy ? '#00ffaa' : '#ff3366';
      ctx.globalAlpha = 0.8;
      ctx.beginPath();
      ctx.arc(particleX, particleY, 3, 0, Math.PI * 2);
      ctx.fill();
      ctx.globalAlpha = 1;
    });
    
    // Draw service nodes - BREATHING WITH TRAFFIC
    const time = Date.now() * 0.001;
    
    services.forEach(service => {
      const pulse = Math.sin(time * service.rps * 0.5) * 3;
      const size = 30 + pulse;
      
      // Shadow effect for active services
      if (service.rps > 0) {
        ctx.shadowBlur = 10 + pulse;
        ctx.shadowColor = service.healthy ? '#00ffaa' : '#ff3366';
      }
      
      // Node background
      ctx.fillStyle = service.healthy ? '#001122' : '#220011';
      ctx.fillRect(service.x - size, service.y - size/2, size * 2, size);
      
      // Node border
      ctx.strokeStyle = service.healthy ? '#00ffaa' : '#ff3366';
      ctx.lineWidth = service === hoveredService ? 3 : 1;
      ctx.strokeRect(service.x - size, service.y - size/2, size * 2, size);
      
      // Reset shadow
      ctx.shadowBlur = 0;
      
      // Service name
      ctx.fillStyle = '#e0e0e0';
      ctx.font = 'bold 11px JetBrains Mono';
      ctx.textAlign = 'center';
      ctx.fillText(service.name, service.x, service.y - 2);
      
      // Metrics
      ctx.font = '9px JetBrains Mono';
      ctx.fillStyle = '#888';
      ctx.fillText(`${service.rps.toFixed(1)} rps`, service.x, service.y + 10);
      
      // Error indicator
      if (service.errorRate > 0) {
        ctx.fillStyle = '#ff3366';
        ctx.fillText(`${(service.errorRate * 100).toFixed(1)}% err`, service.x, service.y + 20);
      }
      
      // Active traces indicator (breathing dots)
      const dotRadius = 2 + Math.sin(time * 3 + service.id.length) * 1;
      for (let i = 0; i < Math.min(service.activeTraces, 5); i++) {
        ctx.fillStyle = '#00ffaa';
        ctx.globalAlpha = 0.6;
        ctx.beginPath();
        ctx.arc(
          service.x - 25 + i * 12,
          service.y - size/2 - 10,
          dotRadius,
          0,
          Math.PI * 2
        );
        ctx.fill();
      }
      ctx.globalAlpha = 1;
    });
    
    // Stats overlay
    ctx.fillStyle = '#00ffaa';
    ctx.font = 'bold 12px JetBrains Mono';
    ctx.textAlign = 'left';
    ctx.fillText('LIVE', 20, 30);
    
    // Continue animation
    animationRef.current = requestAnimationFrame(animate);
  }, [services, flows, hoveredService]);

  // Handle mouse hover
  const handleMouseMove = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    
    let hovered: string | null = null;
    services.forEach(service => {
      const dx = x - service.x;
      const dy = y - service.y;
      if (Math.abs(dx) < 40 && Math.abs(dy) < 20) {
        hovered = service.id;
      }
    });
    
    setHoveredService(hovered);
  }, [services]);

  // Start animation and polling
  useEffect(() => {
    updateMetrics();
    const interval = setInterval(updateMetrics, 1000); // Update every second
    
    animationRef.current = requestAnimationFrame(animate);
    
    return () => {
      clearInterval(interval);
      if (animationRef.current) {
        cancelAnimationFrame(animationRef.current);
      }
    };
  }, [updateMetrics, animate]);

  return (
    <div className="live-service-map">
      <div className="relative">
        <canvas
          ref={canvasRef}
          width={1600}
          height={800}
          className="w-full h-full bg-black border border-gray-900"
          onMouseMove={handleMouseMove}
          style={{ imageRendering: 'crisp-edges' }}
        />
        
        {/* Overlay stats */}
        <div className="absolute top-4 right-4 bg-black/80 p-3 border border-gray-800 text-xs font-mono">
          <div className="text-green-500 mb-2">SERVICE HEALTH</div>
          <div className="space-y-1 text-gray-400">
            <div>Services: {services.size}</div>
            <div>Total RPS: {Array.from(services.values()).reduce((sum, s) => sum + s.rps, 0).toFixed(1)}</div>
            <div>Errors: {Array.from(services.values()).filter(s => !s.healthy).length}</div>
          </div>
        </div>
        
        {/* Hover tooltip */}
        {hoveredService && services.get(hoveredService) && (
          <div className="absolute bg-black/95 border border-green-500 p-2 text-xs font-mono"
               style={{
                 left: services.get(hoveredService)!.x + 50,
                 top: services.get(hoveredService)!.y - 30,
               }}>
            <div className="text-green-500 font-bold mb-1">{hoveredService}</div>
            <div className="text-gray-300 space-y-0.5">
              <div>RPS: {services.get(hoveredService)!.rps.toFixed(2)}</div>
              <div>P95: {services.get(hoveredService)!.p95Latency}ms</div>
              <div>Errors: {(services.get(hoveredService)!.errorRate * 100).toFixed(2)}%</div>
              <div>Active: {services.get(hoveredService)!.activeTraces} traces</div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default LiveServiceMap;