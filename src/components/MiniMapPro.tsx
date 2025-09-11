// MINIMAP PRO - MAXIMUM INFORMATION DENSITY
import React, { useRef, useEffect, useCallback, useState, useMemo } from 'react';

interface MiniMapProProps {
  traces: any[];
  spans: any[];
  currentView: { start: number; end: number };
  onNavigate: (time: number) => void;
  width?: number;
  height?: number;
}

const MiniMapPro: React.FC<MiniMapProProps> = ({
  traces,
  spans,
  currentView,
  onNavigate,
  width = 80,
  height = 600
}) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [hoveredSection, setHoveredSection] = useState<number>(-1);
  
  // Compute data buckets
  const bucketData = useMemo(() => {
    if (!traces.length) return null;
    
    const minTime = Math.min(...traces.map(t => t.start_time));
    const maxTime = Math.max(...traces.map(t => t.start_time + t.duration));
    const timeRange = maxTime - minTime;
    
    const NUM_BUCKETS = Math.floor(height / 3); // 3 pixels per bucket minimum
    const buckets = Array(NUM_BUCKETS).fill(null).map(() => ({
      traces: 0,
      errors: 0,
      slow: 0,
      services: new Set<string>(),
      maxLatency: 0,
      minLatency: Infinity,
      avgLatency: 0,
      totalLatency: 0
    }));
    
    // Fill buckets
    traces.forEach(trace => {
      const bucketIdx = Math.floor(((trace.start_time - minTime) / timeRange) * NUM_BUCKETS);
      if (bucketIdx >= 0 && bucketIdx < NUM_BUCKETS) {
        const bucket = buckets[bucketIdx];
        bucket.traces++;
        bucket.totalLatency += trace.duration;
        bucket.maxLatency = Math.max(bucket.maxLatency, trace.duration);
        bucket.minLatency = Math.min(bucket.minLatency, trace.duration);
        
        if (trace.has_error) bucket.errors++;
        if (trace.duration > 1000000) bucket.slow++; // > 1s
        
        trace.services?.forEach((s: string) => bucket.services.add(s));
      }
    });
    
    // Calculate averages
    buckets.forEach(bucket => {
      if (bucket.traces > 0) {
        bucket.avgLatency = bucket.totalLatency / bucket.traces;
      }
    });
    
    return { buckets, minTime, maxTime, timeRange };
  }, [traces, height]);

  // Render the minimap
  const render = useCallback(() => {
    const canvas = canvasRef.current;
    const ctx = canvas?.getContext('2d');
    if (!ctx || !bucketData) return;
    
    // Clear
    ctx.fillStyle = '#000';
    ctx.fillRect(0, 0, width, height);
    
    const { buckets, minTime, maxTime } = bucketData;
    const bucketHeight = height / buckets.length;
    
    // Find max values for normalization
    const maxTraces = Math.max(...buckets.map(b => b.traces));
    const maxErrors = Math.max(...buckets.map(b => b.errors));
    
    // Render each bucket
    buckets.forEach((bucket, i) => {
      const y = i * bucketHeight;
      
      if (bucket.traces === 0) {
        // Empty bucket - draw faint line
        ctx.fillStyle = '#111';
        ctx.fillRect(0, y, width, 1);
        return;
      }
      
      // MULTI-COLUMN VISUALIZATION
      // Column 1: Traffic density (0-25% width)
      const densityWidth = width * 0.25;
      const densityIntensity = bucket.traces / maxTraces;
      const densityColor = Math.floor(50 + densityIntensity * 205);
      ctx.fillStyle = `rgb(0, ${densityColor}, ${Math.floor(densityColor * 0.7)})`;
      ctx.fillRect(0, y, densityWidth * densityIntensity, bucketHeight);
      
      // Column 2: Error rate (25-40% width)
      if (bucket.errors > 0) {
        const errorRate = bucket.errors / bucket.traces;
        const errorIntensity = bucket.errors / maxErrors;
        ctx.fillStyle = `rgba(255, 51, 102, ${0.5 + errorIntensity * 0.5})`;
        ctx.fillRect(densityWidth, y, width * 0.15 * errorRate, bucketHeight);
      }
      
      // Column 3: Latency indicator (40-60% width)
      const latencyX = width * 0.4;
      const latencyWidth = width * 0.2;
      
      // Color based on latency
      let latencyColor;
      if (bucket.avgLatency < 100000) { // < 100ms
        latencyColor = '#6B7280';
      } else if (bucket.avgLatency < 500000) { // < 500ms
        latencyColor = '#ffaa00';
      } else { // > 500ms
        latencyColor = '#ff3366';
      }
      
      const latencyNormalized = Math.min(bucket.avgLatency / 1000000, 1); // Cap at 1s
      ctx.fillStyle = latencyColor;
      ctx.globalAlpha = 0.3 + latencyNormalized * 0.7;
      ctx.fillRect(latencyX, y, latencyWidth * latencyNormalized, bucketHeight);
      ctx.globalAlpha = 1;
      
      // Column 4: Service count (60-75% width)
      const serviceX = width * 0.6;
      const serviceWidth = width * 0.15;
      const serviceCount = bucket.services.size;
      const serviceIntensity = Math.min(serviceCount / 10, 1); // Cap at 10 services
      ctx.fillStyle = `rgb(${100 + serviceIntensity * 155}, ${100}, ${255})`;
      ctx.fillRect(serviceX, y, serviceWidth * serviceIntensity, bucketHeight);
      
      // Column 5: ASCII visualization (75-100% width)
      const asciiX = width * 0.75;
      ctx.font = '6px monospace';
      ctx.textAlign = 'left';
      
      // Choose character based on characteristics
      let char = '░';
      let color = '#444';
      
      if (bucket.errors > bucket.traces * 0.5) {
        char = '█';
        color = '#ff3366';
      } else if (bucket.slow > bucket.traces * 0.3) {
        char = '▓';
        color = '#ffaa00';
      } else if (densityIntensity > 0.8) {
        char = '▓';
        color = '#6B7280';
      } else if (densityIntensity > 0.4) {
        char = '▒';
        color = '#00aa88';
      }
      
      ctx.fillStyle = color;
      for (let x = asciiX; x < width; x += 6) {
        ctx.fillText(char, x, y + bucketHeight/2);
      }
    });
    
    // Draw current viewport
    const viewStartY = ((currentView.start - minTime) / bucketData.timeRange) * height;
    const viewEndY = ((currentView.end - minTime) / bucketData.timeRange) * height;
    const viewHeight = Math.max(viewEndY - viewStartY, 3);
    
    // Viewport indicator with glow effect
    ctx.strokeStyle = '#6B7280';
    ctx.lineWidth = 2;
    ctx.shadowBlur = 10;
    ctx.shadowColor = '#6B7280';
    ctx.strokeRect(0, viewStartY, width, viewHeight);
    ctx.shadowBlur = 0;
    
    // "YOU ARE HERE" text
    ctx.fillStyle = '#6B7280';
    ctx.font = 'bold 8px monospace';
    ctx.save();
    ctx.translate(width/2, viewStartY + viewHeight/2);
    ctx.rotate(-Math.PI/2);
    ctx.textAlign = 'center';
    ctx.fillText('◄ HERE ►', 0, 0);
    ctx.restore();
    
    // Time markers
    ctx.fillStyle = '#666';
    ctx.font = '7px monospace';
    ctx.textAlign = 'right';
    
    // Draw time scale on left edge
    for (let i = 0; i <= 10; i++) {
      const y = (height / 10) * i;
      const time = new Date((minTime + (bucketData.timeRange / 10) * i) * 1000);
      const timeStr = time.toLocaleTimeString().split(' ')[0];
      
      ctx.save();
      ctx.translate(10, y);
      ctx.rotate(-Math.PI/2);
      ctx.fillText(timeStr, 0, 0);
      ctx.restore();
      
      // Tick mark
      ctx.strokeStyle = '#333';
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(5, y);
      ctx.stroke();
    }
    
    // Legend/header
    ctx.fillStyle = '#666';
    ctx.font = '6px monospace';
    ctx.textAlign = 'center';
    
    // Column headers (rotated)
    const headers = [
      { x: width * 0.125, text: 'LOAD' },
      { x: width * 0.325, text: 'ERR' },
      { x: width * 0.5, text: 'LAT' },
      { x: width * 0.675, text: 'SVC' },
      { x: width * 0.875, text: 'MAP' }
    ];
    
    headers.forEach(header => {
      ctx.save();
      ctx.translate(header.x, 10);
      ctx.fillText(header.text, 0, 0);
      ctx.restore();
    });
    
    // Highlight hovered section
    if (hoveredSection >= 0 && hoveredSection < buckets.length) {
      const y = hoveredSection * bucketHeight;
      ctx.strokeStyle = '#fff';
      ctx.lineWidth = 1;
      ctx.strokeRect(0, y, width, bucketHeight);
      
      // Show details
      const bucket = buckets[hoveredSection];
      if (bucket.traces > 0) {
        ctx.fillStyle = 'rgba(0, 0, 0, 0.9)';
        ctx.fillRect(width, y - 20, 150, 40);
        
        ctx.fillStyle = '#fff';
        ctx.font = '9px monospace';
        ctx.textAlign = 'left';
        ctx.fillText(`Traces: ${bucket.traces}`, width + 5, y - 10);
        ctx.fillText(`Errors: ${bucket.errors} (${(bucket.errors/bucket.traces*100).toFixed(1)}%)`, width + 5, y);
        ctx.fillText(`Avg: ${(bucket.avgLatency/1000).toFixed(1)}ms`, width + 5, y + 10);
        ctx.fillText(`Services: ${bucket.services.size}`, width + 5, y + 20);
      }
    }
    
    // Performance indicator
    const totalErrors = buckets.reduce((sum, b) => sum + b.errors, 0);
    const totalTraces = buckets.reduce((sum, b) => sum + b.traces, 0);
    const errorRate = totalTraces > 0 ? (totalErrors / totalTraces * 100) : 0;
    
    // Status bar at bottom
    ctx.fillStyle = '#000';
    ctx.fillRect(0, height - 15, width, 15);
    ctx.fillStyle = errorRate > 5 ? '#ff3366' : errorRate > 1 ? '#ffaa00' : '#6B7280';
    ctx.font = 'bold 8px monospace';
    ctx.textAlign = 'center';
    ctx.fillText(`${errorRate.toFixed(1)}% ERR`, width/2, height - 4);
    
  }, [bucketData, currentView, hoveredSection, width, height]);

  // Handle mouse events
  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    if (!bucketData) return;
    
    const canvas = canvasRef.current;
    if (!canvas) return;
    
    const rect = canvas.getBoundingClientRect();
    const y = e.clientY - rect.top;
    const bucketHeight = height / bucketData.buckets.length;
    const bucketIdx = Math.floor(y / bucketHeight);
    
    setHoveredSection(bucketIdx);
  }, [bucketData, height]);

  const handleClick = useCallback((e: React.MouseEvent) => {
    if (!bucketData) return;
    
    const canvas = canvasRef.current;
    if (!canvas) return;
    
    const rect = canvas.getBoundingClientRect();
    const y = e.clientY - rect.top;
    const position = y / height;
    
    const targetTime = bucketData.minTime + position * bucketData.timeRange;
    onNavigate(targetTime);
  }, [bucketData, height, onNavigate]);

  // Re-render on data change
  useEffect(() => {
    render();
  }, [render]);

  return (
    <div className="relative inline-block" style={{ width, height }}>
      <canvas
        ref={canvasRef}
        width={width}
        height={height}
        className="cursor-pointer"
        onMouseMove={handleMouseMove}
        onMouseLeave={() => setHoveredSection(-1)}
        onClick={handleClick}
        style={{ 
          imageRendering: 'pixelated',
          border: '1px solid #222'
        }}
      />
      
      {/* Title */}
      <div className="absolute -top-5 left-0 text-xs font-mono text-gray-500">
        MINIMAP
      </div>
    </div>
  );
};

export default MiniMapPro;