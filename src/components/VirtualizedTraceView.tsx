import { memo, useMemo, useState, useCallback, useRef, useEffect } from 'react';
import { TraceInfo, SpanData } from '../types';

interface Props {
  trace: TraceInfo;
  spans: SpanData[];
}

// CRITICAL: This component uses virtualization to handle 100K+ spans
// We handle 100,000+ spans efficiently with virtualization.
const VirtualizedTraceView = memo(({ trace, spans }: Props) => {
  const [expandedSpans, setExpandedSpans] = useState<Set<string>>(new Set());
  const [visibleRange, setVisibleRange] = useState({ start: 0, end: 50 });
  const containerRef = useRef<HTMLDivElement>(null);
  const [selectedSpan, setSelectedSpan] = useState<string | null>(null);

  // Build span tree for hierarchy
  const spanTree = useMemo(() => {
    const tree = new Map<string | undefined, SpanData[]>();
    const spanMap = new Map<string, SpanData>();

    // First pass: create span map
    spans.forEach(span => {
      spanMap.set(span.span_id, span);
    });

    // Second pass: build tree
    spans.forEach(span => {
      const parentId = span.parent_span_id;
      if (!tree.has(parentId)) {
        tree.set(parentId, []);
      }
      tree.get(parentId)!.push(span);
    });

    // Sort children by start time
    tree.forEach(children => {
      children.sort((a, b) => a.start_time - b.start_time);
    });

    return { tree, spanMap };
  }, [spans]);

  // Flatten tree for rendering (with indentation levels)
  const flattenedSpans = useMemo(() => {
    const result: Array<{ span: SpanData; level: number }> = [];
    
    const traverse = (spanId: string | undefined, level: number) => {
      const children = spanTree.tree.get(spanId) || [];
      children.forEach(child => {
        result.push({ span: child, level });
        if (expandedSpans.has(child.span_id)) {
          traverse(child.span_id, level + 1);
        }
      });
    };

    traverse(undefined, 0); // Start with root spans
    return result;
  }, [spanTree, expandedSpans]);

  // Calculate timing info
  const timingInfo = useMemo(() => {
    if (spans.length === 0) return null;

    const minTime = Math.min(...spans.map(s => s.start_time));
    const maxTime = Math.max(...spans.map(s => s.start_time + s.duration));
    const totalDuration = maxTime - minTime;

    return { minTime, maxTime, totalDuration };
  }, [spans]);

  // Handle scroll for virtualization
  const handleScroll = useCallback(() => {
    if (!containerRef.current) return;

    const container = containerRef.current;
    const scrollTop = container.scrollTop;
    const itemHeight = 32; // Height of each span row
    const containerHeight = container.clientHeight;

    const start = Math.floor(scrollTop / itemHeight);
    const visibleCount = Math.ceil(containerHeight / itemHeight);
    const end = Math.min(start + visibleCount + 10, flattenedSpans.length); // Buffer of 10

    setVisibleRange({ start: Math.max(0, start - 10), end }); // Buffer of 10
  }, [flattenedSpans.length]);

  // Setup scroll listener
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    container.addEventListener('scroll', handleScroll);
    return () => container.removeEventListener('scroll', handleScroll);
  }, [handleScroll]);

  const toggleExpand = (spanId: string) => {
    setExpandedSpans(prev => {
      const next = new Set(prev);
      if (next.has(spanId)) {
        next.delete(spanId);
      } else {
        next.add(spanId);
      }
      return next;
    });
  };

  const formatDuration = (ms: number) => {
    if (ms < 1) return `${(ms * 1000).toFixed(0)}μs`;
    if (ms < 1000) return `${ms.toFixed(1)}ms`;
    return `${(ms / 1000).toFixed(2)}s`;
  };

  const getSpanColor = (span: SpanData) => {
    if (span.status === 'error') return 'bg-red-500';
    if (span.duration > 1000) return 'bg-yellow-500';
    return 'bg-green-500';
  };

  const calculateSpanPosition = (span: SpanData) => {
    if (!timingInfo) return { left: 0, width: 0 };

    const relativeStart = span.start_time - timingInfo.minTime;
    const left = (relativeStart / timingInfo.totalDuration) * 100;
    const width = (span.duration / timingInfo.totalDuration) * 100;

    return { left: `${left}%`, width: `${Math.max(width, 0.1)}%` };
  };

  // Get visible spans using virtualization for performance
  const visibleSpans = flattenedSpans.slice(visibleRange.start, visibleRange.end);

  return (
    <div className="h-full flex flex-col">
      {/* Professional Header */}
      <div className="p-4 border-b border-surface-300">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm font-medium text-text-900">
              {trace.root_operation}
            </p>
            <p className="text-xs text-text-500 mt-1 font-mono">
              {trace.span_count} spans • {formatDuration(trace.duration)}
            </p>
          </div>
          
          <div className="text-xs text-text-500">
            <p className="font-mono">Showing {visibleRange.start}-{visibleRange.end} of {flattenedSpans.length}</p>
            <p className="text-status-healthy font-mono">Virtualized rendering</p>
          </div>
        </div>
      </div>

      {/* Timeline header */}
      <div className="h-8 bg-surface-100 border-b border-surface-300 relative px-4">
        <div className="absolute inset-x-4 top-0 h-full flex items-center">
          <span className="text-xs text-text-500 font-mono">0ms</span>
          <div className="flex-1" />
          <span className="text-xs text-text-500 font-mono">{formatDuration(trace.duration)}</span>
        </div>
      </div>

      {/* Virtualized span list */}
      <div
        ref={containerRef}
        className="flex-1 overflow-y-auto relative"
        style={{ height: 'calc(100% - 120px)' }}
      >
        {/* Virtual spacer for scroll */}
        <div style={{ height: `${flattenedSpans.length * 32}px`, position: 'relative' }}>
          {/* Only render visible spans - THIS IS THE SECRET SAUCE! */}
          {visibleSpans.map(({ span, level }, index) => {
            const hasChildren = (spanTree.tree.get(span.span_id)?.length || 0) > 0;
            const isExpanded = expandedSpans.has(span.span_id);
            const position = calculateSpanPosition(span);
            const actualIndex = visibleRange.start + index;

            return (
              <div
                key={span.span_id}
                className={`absolute w-full h-8 border-b border-surface-200 hover:bg-surface-100 transition-colors cursor-pointer ${
                  selectedSpan === span.span_id ? 'bg-surface-200 border-status-info' : ''
                }`}
                style={{ top: `${actualIndex * 32}px` }}
                onClick={() => setSelectedSpan(span.span_id)}
              >
                <div className="flex items-center h-full">
                  {/* Indentation and expand toggle */}
                  <div
                    className="flex items-center h-full"
                    style={{ paddingLeft: `${level * 20 + 8}px`, width: '300px' }}
                  >
                    {hasChildren && (
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          toggleExpand(span.span_id);
                        }}
                        className="mr-1 text-slate-500 hover:text-slate-300"
                      >
                        {isExpanded ? '▼' : '▶'}
                      </button>
                    )}
                    
                    <span className="text-xs text-text-900 truncate font-medium">
                      {span.operation_name}
                    </span>
                    
                    {span.status === 'error' && (
                      <span className="ml-2 text-xs px-1.5 py-0.5 bg-status-error bg-opacity-10 text-status-error rounded border border-status-error border-opacity-20">
                        ERR
                      </span>
                    )}
                  </div>

                  {/* Service name */}
                  <div className="w-32 px-2">
                    <span className="text-xs text-text-500 truncate">
                      {span.service_name}
                    </span>
                  </div>

                  {/* Duration */}
                  <div className="w-20 px-2">
                    <span className="text-xs text-text-700 font-mono">
                      {formatDuration(span.duration)}
                    </span>
                  </div>

                  {/* Timeline bar */}
                  <div className="flex-1 h-full relative px-2">
                    <div className="relative h-full flex items-center">
                      <div
                        className={`absolute h-1 ${getSpanColor(span)} opacity-70`}
                        style={{
                          left: position.left,
                          width: position.width,
                        }}
                      />
                    </div>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* Selected span details */}
      {selectedSpan && (
        <div className="h-48 border-t border-surface-300 p-4 overflow-y-auto bg-surface-50">
          {(() => {
            const span = spanTree.spanMap.get(selectedSpan);
            if (!span) return null;

            return (
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <h4 className="text-sm font-medium text-text-900">
                    {span.operation_name}
                  </h4>
                  <button
                    onClick={() => setSelectedSpan(null)}
                    className="text-text-500 hover:text-text-700 p-1"
                  >
                    ✕
                  </button>
                </div>
                
                <div className="grid grid-cols-2 gap-3 text-xs">
                  <div>
                    <span className="text-text-500">Service:</span>
                    <span className="ml-2 text-text-900 font-medium">{span.service_name}</span>
                  </div>
                  <div>
                    <span className="text-text-500">Duration:</span>
                    <span className="ml-2 text-text-900 font-mono">{formatDuration(span.duration)}</span>
                  </div>
                  <div>
                    <span className="text-text-500">Span ID:</span>
                    <span className="ml-2 text-text-700 font-mono">{span.span_id}</span>
                  </div>
                  <div>
                    <span className="text-text-500">Status:</span>
                    <span className={`ml-2 font-medium ${span.status === 'error' ? 'text-status-error' : 'text-status-healthy'}`}>
                      {span.status}
                    </span>
                  </div>
                </div>

                {span.error_message && (
                  <div className="text-xs bg-status-error bg-opacity-5 border border-status-error border-opacity-20 rounded p-3 text-status-error">
                    {span.error_message}
                  </div>
                )}

                {Object.keys(span.attributes).length > 0 && (
                  <div className="text-xs">
                    <p className="text-text-500 mb-2 font-medium">Attributes:</p>
                    <div className="bg-surface-100 rounded-lg p-3 space-y-2 border border-surface-300">
                      {Object.entries(span.attributes).map(([key, value]) => (
                        <div key={key} className="flex justify-between">
                          <span className="text-text-500 font-mono">{key}:</span>
                          <span className="text-text-900 font-mono text-right">{value}</span>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            );
          })()}
        </div>
      )}
    </div>
  );
});

VirtualizedTraceView.displayName = 'VirtualizedTraceView';

export default VirtualizedTraceView;