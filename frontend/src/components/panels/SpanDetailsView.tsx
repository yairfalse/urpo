// SPAN DETAILS VIEW - SEE EVERYTHING ABOUT A SPAN
import { useState, useMemo, useEffect, memo } from 'react';
import { invoke } from '@tauri-apps/api/tauri';

interface SpanDetails {
  trace_id: string;
  span_id: string;
  parent_id?: string;
  service_name: string;
  operation_name: string;
  start_time: number;
  duration: number;
  status: string;
  attributes: Record<string, string>;
  events: Array<{
    time: number;
    name: string;
    attributes: Record<string, string>;
  }>;
  links: Array<{
    trace_id: string;
    span_id: string;
  }>;
}

interface SpanDetailsViewProps {
  traceId: string;
  spanId?: string;
  onClose?: () => void;
  onNavigateToSpan?: (spanId: string) => void;
}

const SpanDetailsViewImpl = ({
  traceId,
  spanId,
  onClose,
  onNavigateToSpan
}: SpanDetailsViewProps) => {
  const [selectedSpan, setSelectedSpan] = useState<SpanDetails | null>(null);
  const [spans, setSpans] = useState<SpanDetails[]>([]);
  const [loading, setLoading] = useState(true);
  const [viewMode, setViewMode] = useState<'tree' | 'timeline' | 'raw'>('tree');

  // Load trace spans
  useEffect(() => {
    const loadSpans = async () => {
      setLoading(true);
      try {
        const traceSpans = await invoke<SpanDetails[]>('get_trace_spans', { traceId });
        setSpans(traceSpans);
        
        // Auto-select span if provided
        if (spanId) {
          const span = traceSpans.find(s => s.span_id === spanId);
          setSelectedSpan(span || null);
        } else if (traceSpans.length > 0) {
          // Select root span by default
          const root = traceSpans.find(s => !s.parent_id) || traceSpans[0];
          setSelectedSpan(root);
        }
      } catch (error) {
        console.error('Failed to load spans:', error);
      } finally {
        setLoading(false);
      }
    };

    loadSpans();
  }, [traceId, spanId]);

  // Build span tree
  const spanTree = useMemo(() => {
    const tree: Record<string, SpanDetails[]> = {};
    const rootSpans: SpanDetails[] = [];

    spans.forEach(span => {
      if (!span.parent_id) {
        rootSpans.push(span);
      } else {
        if (!tree[span.parent_id]) {
          tree[span.parent_id] = [];
        }
        tree[span.parent_id].push(span);
      }
    });

    return { tree, rootSpans };
  }, [spans]);

  // Format duration
  const formatDuration = (duration: number) => {
    if (duration < 1000) return `${duration}μs`;
    if (duration < 1000000) return `${(duration / 1000).toFixed(2)}ms`;
    return `${(duration / 1000000).toFixed(2)}s`;
  };

  // Format timestamp
  const formatTime = (timestamp: number) => {
    return new Date(timestamp / 1000).toISOString();
  };

  // Render span tree node
  const renderSpanNode = (span: SpanDetails, depth: number = 0) => {
    const children = spanTree.tree[span.span_id] || [];
    const isSelected = selectedSpan?.span_id === span.span_id;
    const hasError = span.status === 'ERROR';

    return (
      <div key={span.span_id}>
        <div
          className={`flex items-center px-2 py-1 cursor-pointer font-mono text-xs border-l-2 ${
            isSelected 
              ? 'bg-gray-500/20 border-gray-500' 
              : 'hover:bg-gray-900 border-transparent'
          } ${hasError ? 'text-red-400' : ''}`}
          style={{ paddingLeft: `${depth * 20 + 8}px` }}
          onClick={() => setSelectedSpan(span)}
        >
          {/* Tree connector */}
          {depth > 0 && (
            <span className="text-gray-600 mr-2">
              {children.length > 0 ? '▼' : '─'}
            </span>
          )}
          
          {/* Service & Operation */}
          <span className={`${hasError ? 'text-red-400' : 'text-gray-400'}`}>
            {span.service_name}
          </span>
          <span className="text-gray-600 mx-1">/</span>
          <span className="text-white flex-1">
            {span.operation_name}
          </span>
          
          {/* Duration */}
          <span className={`ml-2 ${
            span.duration > 1000000 ? 'text-red-400' :
            span.duration > 100000 ? 'text-yellow-400' :
            'text-gray-400'
          }`}>
            {formatDuration(span.duration)}
          </span>
        </div>
        
        {/* Render children */}
        {children.map(child => renderSpanNode(child, depth + 1))}
      </div>
    );
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-gray-500">Loading spans...</div>
      </div>
    );
  }

  return (
    <div className="flex h-full bg-surface-50 text-white">
      {/* Left Panel - Span Tree */}
      <div className="w-1/2 border-r border-gray-800 overflow-y-auto">
        {/* View Mode Tabs */}
        <div className="flex border-b border-gray-800">
          <button
            onClick={() => setViewMode('tree')}
            className={`px-4 py-2 text-xs font-mono ${
              viewMode === 'tree' ? 'bg-gray-900 text-gray-400' : 'text-gray-500'
            }`}
          >
            TREE
          </button>
          <button
            onClick={() => setViewMode('timeline')}
            className={`px-4 py-2 text-xs font-mono ${
              viewMode === 'timeline' ? 'bg-gray-900 text-gray-400' : 'text-gray-500'
            }`}
          >
            TIMELINE
          </button>
          <button
            onClick={() => setViewMode('raw')}
            className={`px-4 py-2 text-xs font-mono ${
              viewMode === 'raw' ? 'bg-gray-900 text-gray-400' : 'text-gray-500'
            }`}
          >
            RAW
          </button>
        </div>

        {/* Span List */}
        <div className="p-2">
          {viewMode === 'tree' && (
            <div>
              {spanTree.rootSpans.map(span => renderSpanNode(span))}
            </div>
          )}
          
          {viewMode === 'timeline' && (
            <div className="space-y-1">
              {[...spans]
                .sort((a, b) => a.start_time - b.start_time)
                .map(span => (
                  <div
                    key={span.span_id}
                    className={`flex items-center px-2 py-1 cursor-pointer text-xs font-mono ${
                      selectedSpan?.span_id === span.span_id
                        ? 'bg-gray-500/20'
                        : 'hover:bg-gray-900'
                    }`}
                    onClick={() => setSelectedSpan(span)}
                  >
                    <span className="text-gray-500 w-20">
                      {formatTime(span.start_time).split('T')[1].split('.')[0]}
                    </span>
                    <span className="text-gray-400 w-32">{span.service_name}</span>
                    <span className="flex-1">{span.operation_name}</span>
                    <span className="text-yellow-400">{formatDuration(span.duration)}</span>
                  </div>
                ))}
            </div>
          )}
          
          {viewMode === 'raw' && (
            <pre className="text-xs text-gray-400">
              {JSON.stringify(spans, null, 2)}
            </pre>
          )}
        </div>
      </div>

      {/* Right Panel - Span Details */}
      <div className="w-1/2 overflow-y-auto">
        {selectedSpan ? (
          <div className="p-4 space-y-4">
            {/* Header */}
            <div className="border-b border-gray-800 pb-4">
              <h3 className="text-lg font-mono text-gray-400">
                {selectedSpan.operation_name}
              </h3>
              <div className="mt-2 space-y-1 text-xs text-gray-400">
                <div>Service: <span className="text-white">{selectedSpan.service_name}</span></div>
                <div>Span ID: <span className="text-white font-mono">{selectedSpan.span_id}</span></div>
                <div>Trace ID: <span className="text-white font-mono">{selectedSpan.trace_id}</span></div>
                {selectedSpan.parent_id && (
                  <div>
                    Parent: 
                    <button
                      onClick={() => {
                        const parent = spans.find(s => s.span_id === selectedSpan.parent_id);
                        if (parent) setSelectedSpan(parent);
                      }}
                      className="ml-2 text-text-700 hover:underline font-mono"
                    >
                      {selectedSpan.parent_id}
                    </button>
                  </div>
                )}
              </div>
            </div>

            {/* Timing */}
            <div>
              <h4 className="text-sm font-bold text-gray-400 mb-2">TIMING</h4>
              <div className="bg-gray-900 p-3 rounded-none space-y-1 text-xs font-mono">
                <div>Start: {formatTime(selectedSpan.start_time)}</div>
                <div>Duration: {formatDuration(selectedSpan.duration)}</div>
                <div>End: {formatTime(selectedSpan.start_time + selectedSpan.duration)}</div>
              </div>
            </div>

            {/* Status */}
            <div>
              <h4 className="text-sm font-bold text-gray-400 mb-2">STATUS</h4>
              <div className={`inline-block px-2 py-1 text-xs font-mono ${
                selectedSpan.status === 'ERROR' 
                  ? 'bg-red-500/20 text-red-400 border border-red-500' 
                  : 'bg-gray-500/20 text-gray-400 border border-gray-500'
              }`}>
                {selectedSpan.status || 'OK'}
              </div>
            </div>

            {/* Attributes */}
            {selectedSpan.attributes && Object.keys(selectedSpan.attributes).length > 0 && (
              <div>
                <h4 className="text-sm font-bold text-gray-400 mb-2">ATTRIBUTES</h4>
                <div className="bg-gray-900 p-3 rounded-none space-y-1 text-xs font-mono">
                  {Object.entries(selectedSpan.attributes).map(([key, value]) => (
                    <div key={key} className="flex">
                      <span className="text-gray-500 w-1/3">{key}:</span>
                      <span className="text-white flex-1 break-all">{value}</span>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Events */}
            {selectedSpan.events && selectedSpan.events.length > 0 && (
              <div>
                <h4 className="text-sm font-bold text-gray-400 mb-2">EVENTS</h4>
                <div className="space-y-2">
                  {selectedSpan.events.map((event, idx) => (
                    <div key={idx} className="bg-gray-900 p-2 rounded-none text-xs">
                      <div className="font-mono text-gray-400">{event.name}</div>
                      <div className="text-gray-500">
                        {formatTime(event.time)}
                      </div>
                      {event.attributes && (
                        <div className="mt-1 text-gray-400">
                          {JSON.stringify(event.attributes)}
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Links */}
            {selectedSpan.links && selectedSpan.links.length > 0 && (
              <div>
                <h4 className="text-sm font-bold text-gray-400 mb-2">LINKS</h4>
                <div className="space-y-1">
                  {selectedSpan.links.map((link, idx) => (
                    <div key={idx} className="text-xs font-mono">
                      <span className="text-gray-500">Trace:</span>
                      <span className="text-text-700 ml-2">{link.trace_id}</span>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        ) : (
          <div className="flex items-center justify-center h-full text-gray-500">
            Select a span to view details
          </div>
        )}
      </div>
    </div>
  );
};

export const SpanDetailsView = memo(SpanDetailsViewImpl);
SpanDetailsView.displayName = 'SpanDetailsView';