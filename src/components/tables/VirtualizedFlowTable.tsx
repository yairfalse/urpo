import { useState, useMemo, memo, useRef, useCallback } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { 
  ArrowRight, 
  AlertTriangle, 
  CheckCircle,
  XCircle,
  Filter,
  Download,
  RefreshCw,
  TrendingUp,
  TrendingDown,
  Minus
} from 'lucide-react';
import { format } from 'date-fns';
import { TraceInfo } from '../../types';

interface Flow {
  id: string;
  timestamp: number;
  source: {
    service: string;
    namespace?: string;
    pod?: string;
  };
  destination: {
    service: string;
    namespace?: string;
    pod?: string;
  };
  protocol: string;
  method?: string;
  path?: string;
  statusCode?: number;
  latency: number;
  bytes: number;
  verdict: 'FORWARDED' | 'DROPPED' | 'ERROR';
  tags?: string[];
}

interface VirtualizedFlowTableProps {
  traces: TraceInfo[];
  onRefresh?: () => void;
}

const ROW_HEIGHT = 64; // Height of each row in pixels
const HEADER_HEIGHT = 40; // Height of header in pixels
const OVERSCAN = 5; // Number of rows to render outside visible area

const VirtualizedFlowTableImpl = ({ traces, onRefresh }: VirtualizedFlowTableProps) => {
  const [selectedFlow, setSelectedFlow] = useState<string | null>(null);
  const [filter, setFilter] = useState('');
  const [verdictFilter, setVerdictFilter] = useState<string>('all');
  const [protocolFilter, setProtocolFilter] = useState<string>('all');
  
  const parentRef = useRef<HTMLDivElement>(null);
  const scrollingRef = useRef<HTMLDivElement>(null);

  // Convert traces to flows (Hubble-style)
  const flows: Flow[] = useMemo(() => {
    return traces.map(trace => ({
      id: trace.trace_id,
      timestamp: trace.start_time * 1000,
      source: {
        service: trace.root_service,
        namespace: 'default',
        pod: `${trace.root_service}-${Math.random().toString(36).substr(2, 5)}`
      },
      destination: {
        service: trace.services?.[1] || 'unknown',
        namespace: 'default',
        pod: trace.services?.[1] ? `${trace.services[1]}-${Math.random().toString(36).substr(2, 5)}` : 'unknown'
      },
      protocol: Math.random() > 0.7 ? 'GRPC' : 'HTTP',
      method: ['GET', 'POST', 'PUT', 'DELETE'][Math.floor(Math.random() * 4)],
      path: trace.root_operation || '/api/v1/data',
      statusCode: trace.has_error ? 500 : 200,
      latency: trace.duration,
      bytes: Math.floor(Math.random() * 10000),
      verdict: trace.has_error ? 'ERROR' : Math.random() > 0.95 ? 'DROPPED' : 'FORWARDED',
      tags: trace.has_error ? ['error', 'alert'] : []
    }));
  }, [traces]);

  // Apply filters
  const filteredFlows = useMemo(() => {
    return flows.filter(flow => {
      if (filter && !flow.source.service.includes(filter) && !flow.destination.service.includes(filter)) {
        return false;
      }
      if (verdictFilter !== 'all' && flow.verdict !== verdictFilter) {
        return false;
      }
      if (protocolFilter !== 'all' && flow.protocol !== protocolFilter) {
        return false;
      }
      return true;
    });
  }, [flows, filter, verdictFilter, protocolFilter]);

  // Initialize virtualizer
  const virtualizer = useVirtualizer({
    count: filteredFlows.length,
    getScrollElement: () => scrollingRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: OVERSCAN,
    getItemKey: (index) => filteredFlows[index]?.id || index,
  });

  const virtualItems = virtualizer.getVirtualItems();

  const getVerdictColor = useCallback((verdict: Flow['verdict']) => {
    switch (verdict) {
      case 'FORWARDED': return 'text-text-500';
      case 'DROPPED': return 'text-amber-400';
      case 'ERROR': return 'text-red-400';
      default: return 'text-text-500';
    }
  }, []);

  const getVerdictIcon = useCallback((verdict: Flow['verdict']) => {
    switch (verdict) {
      case 'FORWARDED': return <CheckCircle className="w-4 h-4" />;
      case 'DROPPED': return <AlertTriangle className="w-4 h-4" />;
      case 'ERROR': return <XCircle className="w-4 h-4" />;
      default: return null;
    }
  }, []);

  const getLatencyTrend = useCallback((latency: number) => {
    if (latency < 50) return <TrendingDown className="w-3 h-3 text-text-500" />;
    if (latency > 200) return <TrendingUp className="w-3 h-3 text-red-400" />;
    return <Minus className="w-3 h-3 text-text-500" />;
  }, []);

  const formatBytes = useCallback((bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }, []);

  return (
    <div className="h-full flex flex-col bg-surface-50" ref={parentRef}>
      {/* Header */}
      <div className="clean-card border-b border-surface-300 p-4 rounded-none">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-2">
              <div className="w-2 h-2 bg-text-700 rounded-full"></div>
              <h2 className="text-lg font-semibold text-text-900">Trace Flows (Virtualized)</h2>
            </div>
            <span className="text-xs text-text-500">
              {filteredFlows.length} of {flows.length} flows
            </span>
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={onRefresh}
              className="p-2 text-text-500 hover:text-text-900 hover:bg-surface-200 rounded"
            >
              <RefreshCw className="w-4 h-4" />
            </button>
            <button className="p-2 text-text-500 hover:text-text-900 hover:bg-surface-200 rounded">
              <Download className="w-4 h-4" />
            </button>
          </div>
        </div>

        {/* Filters */}
        <div className="flex items-center gap-3">
          <div className="relative flex-1 max-w-xs">
            <Filter className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-text-500" />
            <input
              type="text"
              placeholder="Filter by service..."
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              className="w-full pl-10 pr-3 py-2 bg-surface-100 text-text-900 rounded border border-surface-400 focus:border-text-700 focus:outline-none text-sm"
            />
          </div>

          <select
            value={verdictFilter}
            onChange={(e) => setVerdictFilter(e.target.value)}
            className="px-3 py-2 bg-surface-100 text-text-900 rounded border border-surface-400 focus:border-text-700 focus:outline-none text-sm"
          >
            <option value="all">All Status</option>
            <option value="FORWARDED">Success</option>
            <option value="DROPPED">Dropped</option>
            <option value="ERROR">Error</option>
          </select>

          <select
            value={protocolFilter}
            onChange={(e) => setProtocolFilter(e.target.value)}
            className="px-3 py-2 bg-surface-100 text-text-900 rounded border border-surface-400 focus:border-text-700 focus:outline-none text-sm"
          >
            <option value="all">All Protocols</option>
            <option value="HTTP">HTTP</option>
            <option value="GRPC">GRPC</option>
          </select>

          {/* Statistics */}
          <div className="ml-auto flex items-center gap-4 text-xs">
            <div className="flex items-center gap-2">
              <CheckCircle className="w-3 h-3 text-text-500" />
              <span className="text-text-500">
                {flows.filter(f => f.verdict === 'FORWARDED').length}
              </span>
            </div>
            <div className="flex items-center gap-2">
              <AlertTriangle className="w-3 h-3 text-amber-400" />
              <span className="text-text-500">
                {flows.filter(f => f.verdict === 'DROPPED').length}
              </span>
            </div>
            <div className="flex items-center gap-2">
              <XCircle className="w-3 h-3 text-red-400" />
              <span className="text-text-500">
                {flows.filter(f => f.verdict === 'ERROR').length}
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* Virtualized Table */}
      <div className="flex-1 overflow-hidden">
        {/* Table Header */}
        <div className="sticky top-0 bg-surface-100 border-b border-surface-300 z-10">
          <div className="flex text-xs text-text-500 font-medium" style={{ height: HEADER_HEIGHT }}>
            <div className="flex-none w-32 p-3">Time</div>
            <div className="flex-1 min-w-0 p-3">Source</div>
            <div className="flex-none w-12 p-3 text-center">→</div>
            <div className="flex-1 min-w-0 p-3">Destination</div>
            <div className="flex-none w-28 p-3">Protocol</div>
            <div className="flex-1 min-w-0 p-3">Path</div>
            <div className="flex-none w-32 p-3 text-center">Status</div>
            <div className="flex-none w-24 p-3 text-right">Latency</div>
            <div className="flex-none w-24 p-3 text-right">Size</div>
          </div>
        </div>

        {/* Virtualized Rows */}
        <div
          ref={scrollingRef}
          className="overflow-auto"
          style={{ height: `calc(100% - ${HEADER_HEIGHT}px)` }}
        >
          <div
            style={{
              height: `${virtualizer.getTotalSize()}px`,
              width: '100%',
              position: 'relative',
            }}
          >
            {virtualItems.map((virtualRow) => {
              const flow = filteredFlows[virtualRow.index];
              if (!flow) return null;

              return (
                <div
                  key={virtualRow.key}
                  style={{
                    position: 'absolute',
                    top: 0,
                    left: 0,
                    width: '100%',
                    height: `${virtualRow.size}px`,
                    transform: `translateY(${virtualRow.start}px)`,
                  }}
                  className={`
                    flex items-center border-b border-surface-300 hover:bg-surface-100 cursor-pointer
                    ${selectedFlow === flow.id ? 'bg-surface-200' : ''}
                  `}
                  onClick={() => setSelectedFlow(flow.id === selectedFlow ? null : flow.id)}
                >
                  <div className="flex-none w-32 p-3 text-xs text-text-500">
                    {format(new Date(flow.timestamp), 'HH:mm:ss.SSS')}
                  </div>
                  <div className="flex-1 min-w-0 p-3">
                    <div className="text-xs">
                      <div className="text-text-900 font-medium truncate">{flow.source.service}</div>
                      <div className="text-text-500 truncate">{flow.source.pod}</div>
                    </div>
                  </div>
                  <div className="flex-none w-12 p-3 text-center">
                    <ArrowRight className="w-4 h-4 text-text-500 inline" />
                  </div>
                  <div className="flex-1 min-w-0 p-3">
                    <div className="text-xs">
                      <div className="text-text-900 font-medium truncate">{flow.destination.service}</div>
                      <div className="text-text-500 truncate">{flow.destination.pod}</div>
                    </div>
                  </div>
                  <div className="flex-none w-28 p-3">
                    <div className="flex items-center gap-2">
                      <span className="text-xs text-text-900">{flow.protocol}</span>
                      {flow.method && (
                        <span className="text-xs px-1.5 py-0.5 bg-surface-200 text-text-500 rounded">
                          {flow.method}
                        </span>
                      )}
                    </div>
                  </div>
                  <div className="flex-1 min-w-0 p-3 text-xs text-text-500 font-mono truncate">
                    {flow.path}
                  </div>
                  <div className="flex-none w-32 p-3">
                    <div className={`flex items-center justify-center gap-1 ${getVerdictColor(flow.verdict)}`}>
                      {getVerdictIcon(flow.verdict)}
                      <span className="text-xs">{flow.verdict}</span>
                    </div>
                  </div>
                  <div className="flex-none w-24 p-3 text-right">
                    <div className="flex items-center justify-end gap-1">
                      {getLatencyTrend(flow.latency)}
                      <span className="text-xs text-text-900">{flow.latency}ms</span>
                    </div>
                  </div>
                  <div className="flex-none w-24 p-3 text-right text-xs text-text-500">
                    {formatBytes(flow.bytes)}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      </div>

      {/* Performance Stats */}
      <div className="border-t border-surface-300 bg-surface-100 px-4 py-2">
        <div className="flex items-center justify-between text-xs text-text-500">
          <span>
            Rendering {virtualItems.length} of {filteredFlows.length} rows (virtualized)
          </span>
          <span>
            Scroll position: {Math.round((scrollingRef.current?.scrollTop || 0) / ROW_HEIGHT)} / {filteredFlows.length}
          </span>
        </div>
      </div>

      {/* Selected Flow Details */}
      {selectedFlow && (
        <div className="border-t border-surface-300 bg-surface-100 p-4">
          <div className="flex items-center justify-between mb-3">
            <h3 className="text-sm font-medium text-text-900">Flow Details</h3>
            <button
              onClick={() => setSelectedFlow(null)}
              className="text-text-500 hover:text-text-900"
            >
              ✕
            </button>
          </div>
          <div className="grid grid-cols-3 gap-4 text-xs">
            <div>
              <span className="text-text-500">Trace ID:</span>
              <div className="font-mono text-text-700 mt-1">{selectedFlow}</div>
            </div>
            <div>
              <span className="text-text-500">Duration:</span>
              <div className="text-text-900 mt-1">
                {flows.find(f => f.id === selectedFlow)?.latency}ms
              </div>
            </div>
            <div>
              <span className="text-text-500">Status:</span>
              <div className="text-text-900 mt-1">
                {flows.find(f => f.id === selectedFlow)?.statusCode || 'N/A'}
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export const VirtualizedFlowTable = memo(VirtualizedFlowTableImpl);
VirtualizedFlowTable.displayName = 'VirtualizedFlowTable';