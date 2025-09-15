/**
 * Advanced trace timeline with microsecond precision
 * 
 * PERFORMANCE TARGETS:
 * - Handle 10,000+ spans per trace
 * - Smooth 60fps zoom/pan
 * - <1ms hover response
 * - Virtual scrolling for large traces
 */

import React, { useMemo, useRef, useCallback, useState, useEffect } from 'react';
import { FixedSizeList as List } from 'react-window';
import { SpanData, TraceInfo } from '../../types';

interface AdvancedTraceTimelineProps {
  trace: TraceInfo;
  spans: SpanData[];
  className?: string;
  onSpanSelect?: (span: SpanData) => void;
  onSpanHover?: (span: SpanData | null) => void;
}

interface TimelineSpan {
  span: SpanData;
  depth: number;
  x: number;
  width: number;
  color: string;
  textColor: string;
  children: TimelineSpan[];
  parent?: TimelineSpan;
}

interface ViewportState {
  startTime: number;
  endTime: number;
  pixelsPerMicrosecond: number;
  offsetX: number;
}

/**
 * Calculate optimal timeline layout with span hierarchy
 */
function calculateTimelineLayout(spans: SpanData[], viewportWidth: number): {
  timelineSpans: TimelineSpan[];
  totalDuration: number;
  minStartTime: number;
  maxDepth: number;
} {
  if (spans.length === 0) {
    return { timelineSpans: [], totalDuration: 0, minStartTime: 0, maxDepth: 0 };
  }

  // Sort spans by start time for processing
  const sortedSpans = [...spans].sort((a, b) => a.start_time - b.start_time);
  
  const minStartTime = sortedSpans[0].start_time;
  const maxEndTime = Math.max(...sortedSpans.map(s => s.start_time + s.duration_ms * 1000));
  const totalDuration = maxEndTime - minStartTime;
  
  // Build span hierarchy (parent-child relationships)
  const spanMap = new Map<string, SpanData>();
  sortedSpans.forEach(span => spanMap.set(span.span_id, span));
  
  const rootSpans: SpanData[] = [];
  const childrenMap = new Map<string, SpanData[]>();
  
  sortedSpans.forEach(span => {
    if (span.parent_span_id && spanMap.has(span.parent_span_id)) {
      if (!childrenMap.has(span.parent_span_id)) {
        childrenMap.set(span.parent_span_id, []);
      }
      childrenMap.get(span.parent_span_id)!.push(span);
    } else {
      rootSpans.push(span);
    }
  });

  // Calculate pixel positions
  const pixelsPerMicrosecond = viewportWidth / totalDuration;
  
  function buildTimelineSpan(span: SpanData, depth: number, parent?: TimelineSpan): TimelineSpan {
    const relativeStart = span.start_time - minStartTime;
    const x = relativeStart * pixelsPerMicrosecond;
    const width = Math.max(2, span.duration_ms * 1000 * pixelsPerMicrosecond); // Min 2px width
    
    // Color based on span characteristics
    let color: string;
    let textColor: string;
    
    if (span.status?.code === 'ERROR') {
      color = '#dc2626'; // red-600
      textColor = '#ffffff';
    } else if (span.duration_ms > 1000) {
      color = '#d97706'; // amber-600
      textColor = '#ffffff';
    } else if (span.duration_ms > 100) {
      color = '#eab308'; // yellow-500
      textColor = '#000000';
    } else {
      color = '#059669'; // emerald-600
      textColor = '#ffffff';
    }
    
    const timelineSpan: TimelineSpan = {
      span,
      depth,
      x,
      width,
      color,
      textColor,
      children: [],
      parent,
    };
    
    // Recursively build children
    const children = childrenMap.get(span.span_id) || [];
    timelineSpan.children = children
      .sort((a, b) => a.start_time - b.start_time)
      .map(child => buildTimelineSpan(child, depth + 1, timelineSpan));
    
    return timelineSpan;
  }
  
  const timelineSpans = rootSpans.map(span => buildTimelineSpan(span, 0));
  const maxDepth = Math.max(...getAllSpans(timelineSpans).map(s => s.depth));
  
  return { timelineSpans, totalDuration, minStartTime, maxDepth };
}

/**
 * Flatten timeline spans for virtual list
 */
function getAllSpans(timelineSpans: TimelineSpan[]): TimelineSpan[] {
  const result: TimelineSpan[] = [];
  
  function traverse(spans: TimelineSpan[]) {
    spans.forEach(span => {
      result.push(span);
      traverse(span.children);
    });
  }
  
  traverse(timelineSpans);
  return result.sort((a, b) => a.span.start_time - b.span.start_time);
}

/**
 * Format duration for display
 */
function formatDuration(durationMs: number): string {
  if (durationMs < 1) {
    return `${(durationMs * 1000).toFixed(0)}μs`;
  } else if (durationMs < 1000) {
    return `${durationMs.toFixed(2)}ms`;
  } else {
    return `${(durationMs / 1000).toFixed(2)}s`;
  }
}

/**
 * Format timestamp for display
 */
function formatTimestamp(timestampUs: number, baseTime: number): string {
  const relativeMs = (timestampUs - baseTime) / 1000;
  return `+${formatDuration(relativeMs)}`;
}

/**
 * Individual span row component
 */
const SpanRow: React.FC<{
  index: number;
  style: React.CSSProperties;
  data: {
    spans: TimelineSpan[];
    viewport: ViewportState;
    onSpanSelect?: (span: SpanData) => void;
    onSpanHover?: (span: SpanData | null) => void;
    hoveredSpan: string | null;
  };
}> = ({ index, style, data }) => {
  const { spans, viewport, onSpanSelect, onSpanHover, hoveredSpan } = data;
  const timelineSpan = spans[index];
  
  if (!timelineSpan) return null;
  
  const isHovered = hoveredSpan === timelineSpan.span.span_id;
  const isVisible = timelineSpan.x + timelineSpan.width >= viewport.offsetX && 
                   timelineSpan.x <= viewport.offsetX + window.innerWidth;
  
  if (!isVisible) {
    return <div style={style}></div>;
  }
  
  const handleClick = useCallback(() => {
    onSpanSelect?.(timelineSpan.span);
  }, [onSpanSelect, timelineSpan.span]);
  
  const handleMouseEnter = useCallback(() => {
    onSpanHover?.(timelineSpan.span);
  }, [onSpanHover, timelineSpan.span]);
  
  const handleMouseLeave = useCallback(() => {
    onSpanHover?.(null);
  }, [onSpanHover]);
  
  return (
    <div
      style={{
        ...style,
        paddingLeft: `${timelineSpan.depth * 20}px`,
      }}
      className="flex items-center border-b border-surface-200 hover:bg-surface-50 transition-colors duration-75"
    >
      {/* Service name and operation */}
      <div className="w-64 px-2 py-1 text-xs font-mono truncate">
        <div className="text-text-900 font-medium">{timelineSpan.span.service_name}</div>
        <div className="text-text-500">{timelineSpan.span.operation_name}</div>
      </div>
      
      {/* Timeline bar */}
      <div className="flex-1 relative h-8 px-2">
        <div
          className={`absolute h-6 top-1 rounded cursor-pointer transition-all duration-75 ${
            isHovered ? 'ring-2 ring-blue-400 z-10' : ''
          }`}
          style={{
            left: `${Math.max(0, timelineSpan.x - viewport.offsetX)}px`,
            width: `${timelineSpan.width}px`,
            backgroundColor: timelineSpan.color,
            minWidth: '2px',
          }}
          onClick={handleClick}
          onMouseEnter={handleMouseEnter}
          onMouseLeave={handleMouseLeave}
        >
          {/* Span label (only show if wide enough) */}
          {timelineSpan.width > 50 && (
            <div
              className="absolute inset-0 flex items-center px-1 text-xs font-mono truncate"
              style={{ color: timelineSpan.textColor }}
            >
              {timelineSpan.span.operation_name}
            </div>
          )}
        </div>
      </div>
      
      {/* Duration */}
      <div className="w-20 px-2 text-xs font-mono text-right text-text-600">
        {formatDuration(timelineSpan.span.duration_ms)}
      </div>
      
      {/* Start time */}
      <div className="w-24 px-2 text-xs font-mono text-right text-text-500">
        {formatTimestamp(timelineSpan.span.start_time, timelineSpan.span.start_time)}
      </div>
    </div>
  );
};

/**
 * Timeline ruler component
 */
const TimelineRuler: React.FC<{
  viewport: ViewportState;
  totalDuration: number;
  minStartTime: number;
}> = ({ viewport, totalDuration, minStartTime }) => {
  const ticks = useMemo(() => {
    const tickInterval = totalDuration / 10;
    const ticks: Array<{ position: number; label: string }> = [];
    
    for (let i = 0; i <= 10; i++) {
      const time = i * tickInterval;
      const position = time * viewport.pixelsPerMicrosecond - viewport.offsetX;
      
      if (position >= -50 && position <= window.innerWidth + 50) {
        ticks.push({
          position,
          label: formatTimestamp(minStartTime + time, minStartTime),
        });
      }
    }
    
    return ticks;
  }, [viewport, totalDuration, minStartTime]);
  
  return (
    <div className="h-8 border-b border-surface-300 bg-surface-100 relative overflow-hidden">
      {ticks.map((tick, i) => (
        <div
          key={i}
          className="absolute top-0 h-full border-l border-surface-400"
          style={{ left: `${tick.position}px` }}
        >
          <div className="absolute top-1 left-1 text-xs font-mono text-text-600">
            {tick.label}
          </div>
        </div>
      ))}
    </div>
  );
};

/**
 * Main advanced trace timeline component
 */
export const AdvancedTraceTimeline: React.FC<AdvancedTraceTimelineProps> = ({
  trace,
  spans,
  className = '',
  onSpanSelect,
  onSpanHover,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const [viewport, setViewport] = useState<ViewportState>({
    startTime: 0,
    endTime: 0,
    pixelsPerMicrosecond: 0,
    offsetX: 0,
  });
  const [hoveredSpan, setHoveredSpan] = useState<string | null>(null);
  
  // Calculate timeline layout
  const { timelineSpans, totalDuration, minStartTime, maxDepth } = useMemo(() => {
    return calculateTimelineLayout(spans, 1000); // Initial width estimate
  }, [spans]);
  
  const flatSpans = useMemo(() => getAllSpans(timelineSpans), [timelineSpans]);
  
  // Initialize viewport
  useEffect(() => {
    if (containerRef.current && totalDuration > 0) {
      const containerWidth = containerRef.current.clientWidth - 320; // Account for fixed columns
      const pixelsPerMicrosecond = containerWidth / totalDuration;
      
      setViewport({
        startTime: minStartTime,
        endTime: minStartTime + totalDuration,
        pixelsPerMicrosecond,
        offsetX: 0,
      });
    }
  }, [totalDuration, minStartTime]);
  
  // Handle hover events
  const handleSpanHover = useCallback((span: SpanData | null) => {
    setHoveredSpan(span?.span_id || null);
    onSpanHover?.(span);
  }, [onSpanHover]);
  
  // Zoom and pan handlers
  const handleWheel = useCallback((e: React.WheelEvent) => {
    e.preventDefault();
    
    if (e.ctrlKey || e.metaKey) {
      // Zoom
      const zoomFactor = e.deltaY > 0 ? 0.9 : 1.1;
      setViewport(prev => ({
        ...prev,
        pixelsPerMicrosecond: prev.pixelsPerMicrosecond * zoomFactor,
      }));
    } else {
      // Pan
      setViewport(prev => ({
        ...prev,
        offsetX: Math.max(0, prev.offsetX + e.deltaX),
      }));
    }
  }, []);
  
  if (spans.length === 0) {
    return (
      <div className={`flex items-center justify-center h-64 bg-surface-50 rounded-lg ${className}`}>
        <div className="text-center">
          <div className="text-text-500">No spans found for this trace</div>
          <div className="text-xs text-text-400 mt-1">Trace ID: {trace.trace_id}</div>
        </div>
      </div>
    );
  }
  
  return (
    <div
      ref={containerRef}
      className={`bg-white rounded-lg border border-surface-300 overflow-hidden ${className}`}
      onWheel={handleWheel}
    >
      {/* Header */}
      <div className="p-4 border-b border-surface-200">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-lg font-semibold text-text-900">Trace Timeline</h3>
            <div className="text-sm text-text-500 font-mono">
              {trace.trace_id} • {spans.length} spans • {formatDuration(trace.duration)}
            </div>
          </div>
          <div className="flex items-center gap-4 text-xs text-text-500">
            <div>🖱️ Scroll to pan</div>
            <div>⌘ + Scroll to zoom</div>
            <div>Click spans to inspect</div>
          </div>
        </div>
      </div>
      
      {/* Column headers */}
      <div className="flex items-center border-b border-surface-300 bg-surface-50 h-8">
        <div className="w-64 px-2 text-xs font-medium text-text-700">Service / Operation</div>
        <div className="flex-1 px-2 text-xs font-medium text-text-700">Timeline</div>
        <div className="w-20 px-2 text-xs font-medium text-text-700 text-right">Duration</div>
        <div className="w-24 px-2 text-xs font-medium text-text-700 text-right">Start</div>
      </div>
      
      {/* Timeline ruler */}
      <TimelineRuler
        viewport={viewport}
        totalDuration={totalDuration}
        minStartTime={minStartTime}
      />
      
      {/* Virtual list of spans */}
      <div className="h-96 overflow-hidden">
        <List
          height={384}
          itemCount={flatSpans.length}
          itemSize={32}
          itemData={{
            spans: flatSpans,
            viewport,
            onSpanSelect,
            onSpanHover: handleSpanHover,
            hoveredSpan,
          }}
        >
          {SpanRow}
        </List>
      </div>
      
      {/* Statistics footer */}
      <div className="p-3 border-t border-surface-200 bg-surface-50">
        <div className="flex items-center justify-between text-xs">
          <div className="flex items-center gap-4 text-text-600">
            <span>📊 {spans.length} spans</span>
            <span>🏗️ {maxDepth + 1} levels deep</span>
            <span>⚡ {formatDuration(trace.duration)} total</span>
          </div>
          <div className="flex items-center gap-2">
            <div className="w-3 h-3 bg-green-600 rounded"></div>
            <span className="text-text-500">Fast (&lt;100ms)</span>
            <div className="w-3 h-3 bg-yellow-500 rounded ml-2"></div>
            <span className="text-text-500">Slow (&gt;100ms)</span>
            <div className="w-3 h-3 bg-red-600 rounded ml-2"></div>
            <span className="text-text-500">Error</span>
          </div>
        </div>
      </div>
    </div>
  );
};

export default AdvancedTraceTimeline;