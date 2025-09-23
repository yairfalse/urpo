import { useMemo, useState, memo } from 'react';
import { Clock, AlertCircle, CheckCircle, ChevronRight, ChevronDown, Server } from 'lucide-react';

interface SpanData {
  span_id: string;
  trace_id: string;
  parent_span_id?: string;
  service_name: string;
  operation_name: string;
  start_time: number;
  duration: number;
  status: 'ok' | 'error' | 'cancelled';
  attributes?: Record<string, any>;
  depth?: number;
  children?: SpanData[];
}

interface TraceWaterfallProps {
  spans: SpanData[];
  traceId: string;
}

// Color scheme for different services
const SERVICE_COLORS = [
  '#5B8FF9', '#5AD8A6', '#975FE4', '#FF9845', '#5DCFFF',
  '#FF6B9D', '#3BCBB0', '#FFC53D', '#F6465D', '#8B5CF6',
];

const getServiceColor = (serviceName: string, serviceMap: Map<string, number>): string => {
  if (!serviceMap.has(serviceName)) {
    serviceMap.set(serviceName, serviceMap.size);
  }
  const index = serviceMap.get(serviceName)!;
  return SERVICE_COLORS[index % SERVICE_COLORS.length];
};

const TraceWaterfall = memo(({ spans, traceId }: TraceWaterfallProps) => {
  const [expandedSpans, setExpandedSpans] = useState<Set<string>>(new Set());
  const [selectedSpan, setSelectedSpan] = useState<string | null>(null);

  // Build span tree and calculate timing
  const { spanTree, minTime, maxTime, serviceColorMap } = useMemo(() => {
    if (!spans.length) return { spanTree: [], minTime: 0, maxTime: 1, serviceColorMap: new Map() };

    const spanMap = new Map<string, SpanData>();
    const rootSpans: SpanData[] = [];
    const serviceMap = new Map<string, number>();

    // First pass: create span map
    spans.forEach(span => {
      spanMap.set(span.span_id, { ...span, children: [] });
    });

    // Second pass: build tree structure
    spans.forEach(span => {
      if (span.parent_span_id && spanMap.has(span.parent_span_id)) {
        const parent = spanMap.get(span.parent_span_id)!;
        parent.children = parent.children || [];
        parent.children.push(spanMap.get(span.span_id)!);
      } else {
        rootSpans.push(spanMap.get(span.span_id)!);
      }
    });

    // Calculate depth for each span
    const calculateDepth = (span: SpanData, depth: number = 0): void => {
      span.depth = depth;
      span.children?.forEach(child => calculateDepth(child, depth + 1));
    };

    rootSpans.forEach(span => calculateDepth(span));

    // Find time range
    const times = spans.flatMap(s => [s.start_time, s.start_time + s.duration]);
    const min = Math.min(...times);
    const max = Math.max(...times);

    // Generate service colors
    spans.forEach(span => getServiceColor(span.service_name, serviceMap));

    return {
      spanTree: rootSpans,
      minTime: min,
      maxTime: max,
      serviceColorMap: serviceMap,
    };
  }, [spans]);

  const totalDuration = maxTime - minTime;

  // Toggle span expansion
  const toggleSpan = (spanId: string) => {
    const newExpanded = new Set(expandedSpans);
    if (newExpanded.has(spanId)) {
      newExpanded.delete(spanId);
    } else {
      newExpanded.add(spanId);
    }
    setExpandedSpans(newExpanded);
  };

  // Render a single span row
  const renderSpan = (span: SpanData, level: number = 0): JSX.Element => {
    const hasChildren = span.children && span.children.length > 0;
    const isExpanded = expandedSpans.has(span.span_id);
    const isSelected = selectedSpan === span.span_id;

    const relativeStart = ((span.start_time - minTime) / totalDuration) * 100;
    const relativeWidth = (span.duration / totalDuration) * 100;

    const barColor = getServiceColor(span.service_name, serviceColorMap);

    return (
      <div key={span.span_id}>
        <div
          className={`
            group flex items-center h-8 border-b border-dark-300
            hover:bg-dark-100 transition-colors cursor-pointer
            ${isSelected ? 'bg-dark-150' : ''}
          `}
          onClick={() => setSelectedSpan(span.span_id)}
        >
          {/* Service and operation name */}
          <div className="flex-shrink-0 w-80 px-2 flex items-center">
            <div className="flex items-center" style={{ paddingLeft: `${level * 20}px` }}>
              {hasChildren && (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    toggleSpan(span.span_id);
                  }}
                  className="mr-1 text-light-400 hover:text-light-200"
                >
                  {isExpanded ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
                </button>
              )}
              {!hasChildren && <span className="w-5" />}

              <Server className="w-4 h-4 mr-2 text-light-500" />

              <div className="flex flex-col">
                <span className="text-xs text-light-200 font-medium">
                  {span.service_name}
                </span>
                <span className="text-[10px] text-light-500">
                  {span.operation_name}
                </span>
              </div>
            </div>
          </div>

          {/* Duration */}
          <div className="flex-shrink-0 w-20 px-2 text-xs text-light-400 text-right">
            {span.duration.toFixed(2)}ms
          </div>

          {/* Waterfall bar */}
          <div className="flex-1 relative h-full py-2">
            <div className="relative h-full">
              {/* Timeline background grid */}
              <div className="absolute inset-0 flex">
                {[...Array(10)].map((_, i) => (
                  <div
                    key={i}
                    className="flex-1 border-r border-dark-300 border-opacity-20"
                  />
                ))}
              </div>

              {/* Span bar */}
              <div
                className="absolute top-1 bottom-1 rounded-sm transition-opacity hover:opacity-80"
                style={{
                  left: `${relativeStart}%`,
                  width: `${Math.max(0.5, relativeWidth)}%`,
                  backgroundColor: barColor,
                  boxShadow: `0 0 0 1px ${barColor}40`,
                }}
              >
                <div className="h-full relative">
                  {/* Status indicator */}
                  {span.status === 'error' && (
                    <AlertCircle className="absolute right-1 top-1/2 -translate-y-1/2 w-3 h-3 text-white" />
                  )}
                </div>
              </div>
            </div>
          </div>

          {/* Status */}
          <div className="flex-shrink-0 w-16 px-2">
            {span.status === 'ok' ? (
              <CheckCircle className="w-4 h-4 text-semantic-success" />
            ) : span.status === 'error' ? (
              <AlertCircle className="w-4 h-4 text-semantic-error" />
            ) : (
              <Clock className="w-4 h-4 text-light-500" />
            )}
          </div>
        </div>

        {/* Render children if expanded */}
        {isExpanded && hasChildren && (
          <div>
            {span.children!.map(child => renderSpan(child, level + 1))}
          </div>
        )}
      </div>
    );
  };

  // Calculate timeline markers
  const timeMarkers = useMemo(() => {
    const markers: number[] = [];
    const step = totalDuration / 10;
    for (let i = 0; i <= 10; i++) {
      markers.push(minTime + i * step);
    }
    return markers;
  }, [minTime, totalDuration]);

  return (
    <div className="h-full bg-dark-50 rounded-lg border border-dark-300 flex flex-col">
      {/* Header */}
      <div className="p-4 border-b border-dark-300 bg-dark-100">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-lg font-semibold text-light-50">Trace Waterfall</h2>
            <p className="text-sm text-light-400 mt-1">
              Trace ID: <code className="text-xs bg-dark-200 px-2 py-0.5 rounded">{traceId}</code>
            </p>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-xs text-light-500">Total Duration:</span>
            <span className="text-sm font-medium text-data-cyan">{totalDuration.toFixed(2)}ms</span>
          </div>
        </div>
      </div>

      {/* Column headers */}
      <div className="flex items-center h-10 bg-dark-150 border-b border-dark-300">
        <div className="flex-shrink-0 w-80 px-2 text-xs font-medium text-light-400">
          Service / Operation
        </div>
        <div className="flex-shrink-0 w-20 px-2 text-xs font-medium text-light-400 text-right">
          Duration
        </div>
        <div className="flex-1 px-2 relative">
          {/* Time markers */}
          <div className="flex justify-between text-[10px] text-light-500">
            {timeMarkers.map((time, i) => (
              <span key={i}>{(time - minTime).toFixed(0)}ms</span>
            ))}
          </div>
        </div>
        <div className="flex-shrink-0 w-16 px-2 text-xs font-medium text-light-400">
          Status
        </div>
      </div>

      {/* Span list */}
      <div className="flex-1 overflow-auto">
        {spanTree.length > 0 ? (
          spanTree.map(span => renderSpan(span))
        ) : (
          <div className="p-8 text-center text-light-500">
            No spans available for this trace
          </div>
        )}
      </div>

      {/* Selected span details */}
      {selectedSpan && (
        <div className="p-4 border-t border-dark-300 bg-dark-100">
          <h3 className="text-sm font-semibold text-light-50 mb-2">Span Details</h3>
          {(() => {
            const span = spans.find(s => s.span_id === selectedSpan);
            if (!span) return null;
            return (
              <div className="grid grid-cols-2 gap-4 text-xs">
                <div>
                  <span className="text-light-500">Span ID:</span>
                  <code className="ml-2 text-light-300">{span.span_id}</code>
                </div>
                <div>
                  <span className="text-light-500">Service:</span>
                  <span className="ml-2 text-light-300">{span.service_name}</span>
                </div>
                <div>
                  <span className="text-light-500">Operation:</span>
                  <span className="ml-2 text-light-300">{span.operation_name}</span>
                </div>
                <div>
                  <span className="text-light-500">Duration:</span>
                  <span className="ml-2 text-light-300">{span.duration.toFixed(3)}ms</span>
                </div>
                {span.attributes && Object.keys(span.attributes).length > 0 && (
                  <div className="col-span-2">
                    <span className="text-light-500">Attributes:</span>
                    <pre className="mt-1 text-[10px] text-light-400 bg-dark-200 p-2 rounded overflow-auto">
                      {JSON.stringify(span.attributes, null, 2)}
                    </pre>
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

TraceWaterfall.displayName = 'TraceWaterfall';

export { TraceWaterfall };