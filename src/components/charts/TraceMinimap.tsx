// TRACE MINIMAP - SEE EVERYTHING AT A GLANCE
import React, { useRef, useEffect, useCallback, useState } from 'react';

interface MinimapProps {
  traces: any[];
  currentView: { start: number; end: number };
  onViewChange: (start: number, end: number) => void;
  height?: number;
  width?: number;
}

const TraceMinimap: React.FC<MinimapProps> = ({ 
  traces, 
  currentView, 
  onViewChange,
  height = 400,
  width = 60 
}) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [isDragging, setIsDragging] = useState(false);
  const [hoverInfo, setHoverInfo] = useState<{ y: number; text: string } | null>(null);

  // Render the minimap
  const renderMinimap = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Clear with dark background
    ctx.fillStyle = '#0a0a0a';
    ctx.fillRect(0, 0, width, height);
    
    // Draw border
    ctx.strokeStyle = '#1a1a1a';
    ctx.lineWidth = 1;
    ctx.strokeRect(0, 0, width, height);

    if (traces.length === 0) {
      // Empty state
      ctx.fillStyle = '#333';
      ctx.font = '10px monospace';
      ctx.textAlign = 'center';
      ctx.save();
      ctx.translate(width / 2, height / 2);
      ctx.rotate(-Math.PI / 2);
      ctx.fillText('NO TRACES', 0, 0);
      ctx.restore();
      return;
    }

    // Calculate time range
    const minTime = Math.min(...traces.map(t => t.start_time));
    const maxTime = Math.max(...traces.map(t => t.start_time + t.duration));
    const timeRange = maxTime - minTime;

    // Create density map
    const buckets = 100;
    const densityMap = new Array(buckets).fill(0);
    const errorMap = new Array(buckets).fill(0);
    const slowMap = new Array(buckets).fill(0);

    traces.forEach(trace => {
      const bucketIndex = Math.floor(((trace.start_time - minTime) / timeRange) * buckets);
      if (bucketIndex >= 0 && bucketIndex < buckets) {
        densityMap[bucketIndex]++;
        if (trace.has_error) {
          errorMap[bucketIndex]++;
        }
        if (trace.duration > 1000000) { // Slow if > 1s
          slowMap[bucketIndex]++;
        }
      }
    });

    const maxDensity = Math.max(...densityMap);

    // Draw density visualization
    const bucketHeight = height / buckets;
    
    densityMap.forEach((density, i) => {
      const y = i * bucketHeight;
      const intensity = density / maxDensity;
      
      // Base density - shades of gray
      if (density > 0) {
        const grayLevel = Math.floor(100 + intensity * 155);
        ctx.fillStyle = `rgb(${grayLevel}, ${grayLevel}, ${grayLevel})`;
        ctx.fillRect(0, y, width, bucketHeight);
      }
      
      // Error overlay - red
      if (errorMap[i] > 0) {
        const errorIntensity = errorMap[i] / density;
        ctx.fillStyle = `rgba(255, 51, 102, ${errorIntensity * 0.8})`;
        ctx.fillRect(0, y, width, bucketHeight);
      }
      
      // Slow traces overlay - yellow
      if (slowMap[i] > 0) {
        const slowIntensity = slowMap[i] / density;
        ctx.fillStyle = `rgba(255, 170, 0, ${slowIntensity * 0.5})`;
        ctx.fillRect(width * 0.7, y, width * 0.3, bucketHeight);
      }
    });

    // Draw ASCII-style characters for visual texture
    ctx.font = '8px monospace';
    densityMap.forEach((density, i) => {
      const y = i * bucketHeight + bucketHeight / 2;
      let char = '░';
      
      if (errorMap[i] > density * 0.5) {
        char = '█';
        ctx.fillStyle = '#ff3366';
      } else if (density > maxDensity * 0.8) {
        char = '▓';
        ctx.fillStyle = '#6B7280';
      } else if (density > maxDensity * 0.5) {
        char = '▒';
        ctx.fillStyle = '#888';
      } else if (density > 0) {
        char = '░';
        ctx.fillStyle = '#444';
      }
      
      if (density > 0) {
        ctx.textAlign = 'center';
        for (let x = 10; x < width; x += 10) {
          ctx.fillText(char, x, y);
        }
      }
    });

    // Draw current viewport indicator
    const viewStartY = ((currentView.start - minTime) / timeRange) * height;
    const viewEndY = ((currentView.end - minTime) / timeRange) * height;
    const viewHeight = Math.max(viewEndY - viewStartY, 2);

    // Viewport background
    ctx.fillStyle = 'rgba(0, 255, 170, 0.1)';
    ctx.fillRect(0, viewStartY, width, viewHeight);

    // Viewport border
    ctx.strokeStyle = '#6B7280';
    ctx.lineWidth = 2;
    ctx.strokeRect(0, viewStartY, width, viewHeight);

    // "You are here" indicator
    ctx.fillStyle = '#6B7280';
    ctx.font = 'bold 10px monospace';
    ctx.save();
    ctx.translate(width / 2, viewStartY + viewHeight / 2);
    ctx.rotate(-Math.PI / 2);
    ctx.textAlign = 'center';
    ctx.fillText('◄ HERE', 0, 0);
    ctx.restore();

    // Draw time labels
    ctx.fillStyle = '#666';
    ctx.font = '8px monospace';
    ctx.textAlign = 'right';
    
    // Top time
    const topTime = new Date(minTime * 1000);
    ctx.save();
    ctx.translate(width - 2, 10);
    ctx.rotate(-Math.PI / 2);
    ctx.fillText(topTime.toLocaleTimeString(), 0, 0);
    ctx.restore();

    // Bottom time
    const bottomTime = new Date(maxTime * 1000);
    ctx.save();
    ctx.translate(width - 2, height - 10);
    ctx.rotate(-Math.PI / 2);
    ctx.fillText(bottomTime.toLocaleTimeString(), 0, 0);
    ctx.restore();

    // Draw annotations for interesting regions
    const annotations: Array<{ y: number; label: string; color: string }> = [];
    
    // Find error clusters
    for (let i = 0; i < buckets - 5; i++) {
      const errorSum = errorMap.slice(i, i + 5).reduce((a, b) => a + b, 0);
      if (errorSum > 10) {
        annotations.push({
          y: (i + 2.5) * bucketHeight,
          label: 'ERR',
          color: '#ff3366'
        });
        i += 5; // Skip ahead
      }
    }

    // Find traffic spikes
    for (let i = 1; i < buckets - 1; i++) {
      if (densityMap[i] > densityMap[i-1] * 2 && densityMap[i] > densityMap[i+1] * 2) {
        annotations.push({
          y: i * bucketHeight,
          label: 'SPIKE',
          color: '#ffaa00'
        });
      }
    }

    // Draw annotations
    annotations.forEach(ann => {
      ctx.strokeStyle = ann.color;
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(0, ann.y);
      ctx.lineTo(5, ann.y);
      ctx.stroke();
      
      ctx.fillStyle = ann.color;
      ctx.font = '7px monospace';
      ctx.fillText(ann.label, 8, ann.y + 2);
    });

  }, [traces, currentView, width, height]);

  // Handle click/drag to navigate
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    setIsDragging(true);
    const canvas = canvasRef.current;
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    const y = e.clientY - rect.top;
    const position = y / height;

    if (traces.length > 0) {
      const minTime = Math.min(...traces.map(t => t.start_time));
      const maxTime = Math.max(...traces.map(t => t.start_time + t.duration));
      const timeRange = maxTime - minTime;
      const clickTime = minTime + position * timeRange;
      
      // Center view on clicked position
      const viewSize = (currentView.end - currentView.start);
      onViewChange(clickTime - viewSize / 2, clickTime + viewSize / 2);
    }
  }, [traces, height, currentView, onViewChange]);

  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    const y = e.clientY - rect.top;

    if (isDragging && traces.length > 0) {
      const position = y / height;
      const minTime = Math.min(...traces.map(t => t.start_time));
      const maxTime = Math.max(...traces.map(t => t.start_time + t.duration));
      const timeRange = maxTime - minTime;
      const clickTime = minTime + position * timeRange;
      
      const viewSize = (currentView.end - currentView.start);
      onViewChange(clickTime - viewSize / 2, clickTime + viewSize / 2);
    } else {
      // Show hover info
      const position = y / height;
      if (traces.length > 0) {
        const minTime = Math.min(...traces.map(t => t.start_time));
        const maxTime = Math.max(...traces.map(t => t.start_time + t.duration));
        const timeRange = maxTime - minTime;
        const hoverTime = minTime + position * timeRange;
        const time = new Date(hoverTime * 1000);
        
        setHoverInfo({
          y: y,
          text: time.toLocaleTimeString()
        });
      }
    }
  }, [isDragging, traces, height, currentView, onViewChange]);

  const handleMouseUp = useCallback(() => {
    setIsDragging(false);
  }, []);

  const handleMouseLeave = useCallback(() => {
    setIsDragging(false);
    setHoverInfo(null);
  }, []);

  // Render on data change
  useEffect(() => {
    renderMinimap();
  }, [renderMinimap]);

  return (
    <div className="relative inline-block">
      <canvas
        ref={canvasRef}
        width={width}
        height={height}
        className="border border-gray-800 cursor-ns-resize"
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseLeave}
        style={{ imageRendering: 'pixelated' }}
      />
      
      {/* Hover tooltip */}
      {hoverInfo && (
        <div 
          className="absolute left-full ml-2 px-2 py-1 bg-surface-50 border border-gray-500 text-xs font-mono text-gray-500 pointer-events-none"
          style={{ top: hoverInfo.y - 10 }}
        >
          {hoverInfo.text}
        </div>
      )}
      
      {/* Legend */}
      <div className="absolute -top-6 left-0 text-xs font-mono text-gray-500">
        MAP
      </div>
      
      <div className="absolute -bottom-6 left-0 text-xs font-mono space-x-2">
        <span className="text-gray-500">░ Normal</span>
        <span className="text-red-500">█ Errors</span>
        <span className="text-yellow-500">▓ Slow</span>
      </div>
    </div>
  );
};

export default TraceMinimap;