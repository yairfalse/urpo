import { useState, useMemo } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { 
  ArrowRight, 
  Clock, 
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

interface FlowTableProps {
  traces: any[];
  onRefresh?: () => void;
}

export default function FlowTable({ traces, onRefresh }: FlowTableProps) {
  const [selectedFlow, setSelectedFlow] = useState<string | null>(null);
  const [filter, setFilter] = useState('');
  const [verdictFilter, setVerdictFilter] = useState<string>('all');
  const [protocolFilter, setProtocolFilter] = useState<string>('all');

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

  const getVerdictColor = (verdict: Flow['verdict']) => {
    switch (verdict) {
      case 'FORWARDED': return 'text-green-400';
      case 'DROPPED': return 'text-amber-400';
      case 'ERROR': return 'text-red-400';
      default: return 'text-slate-400';
    }
  };

  const getVerdictIcon = (verdict: Flow['verdict']) => {
    switch (verdict) {
      case 'FORWARDED': return <CheckCircle className="w-4 h-4" />;
      case 'DROPPED': return <AlertTriangle className="w-4 h-4" />;
      case 'ERROR': return <XCircle className="w-4 h-4" />;
      default: return null;
    }
  };

  const getLatencyTrend = (latency: number) => {
    if (latency < 50) return <TrendingDown className="w-3 h-3 text-green-400" />;
    if (latency > 200) return <TrendingUp className="w-3 h-3 text-red-400" />;
    return <Minus className="w-3 h-3 text-slate-400" />;
  };

  const formatBytes = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  return (
    <div className="h-full flex flex-col bg-slate-950">
      {/* Hubble-style Header */}
      <div className="bg-slate-900 border-b border-slate-800 p-4">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-2">
              <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
              <h2 className="text-lg font-semibold text-white">Trace Flows</h2>
            </div>
            <span className="text-xs text-slate-500">
              {filteredFlows.length} trace flows
            </span>
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={onRefresh}
              className="p-2 text-slate-400 hover:text-white hover:bg-slate-800 rounded transition-colors"
            >
              <RefreshCw className="w-4 h-4" />
            </button>
            <button className="p-2 text-slate-400 hover:text-white hover:bg-slate-800 rounded transition-colors">
              <Download className="w-4 h-4" />
            </button>
          </div>
        </div>

        {/* Filters */}
        <div className="flex items-center gap-3">
          <div className="relative flex-1 max-w-xs">
            <Filter className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-500" />
            <input
              type="text"
              placeholder="Filter by service..."
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              className="w-full pl-10 pr-3 py-2 bg-slate-800 text-white rounded border border-slate-700 focus:border-green-500 focus:outline-none text-sm"
            />
          </div>

          <select
            value={verdictFilter}
            onChange={(e) => setVerdictFilter(e.target.value)}
            className="px-3 py-2 bg-slate-800 text-white rounded border border-slate-700 focus:border-green-500 focus:outline-none text-sm"
          >
            <option value="all">All Status</option>
            <option value="FORWARDED">Success</option>
            <option value="DROPPED">Dropped</option>
            <option value="ERROR">Error</option>
          </select>

          <select
            value={protocolFilter}
            onChange={(e) => setProtocolFilter(e.target.value)}
            className="px-3 py-2 bg-slate-800 text-white rounded border border-slate-700 focus:border-green-500 focus:outline-none text-sm"
          >
            <option value="all">All Protocols</option>
            <option value="HTTP">HTTP</option>
            <option value="GRPC">GRPC</option>
          </select>

          {/* Statistics */}
          <div className="ml-auto flex items-center gap-4 text-xs">
            <div className="flex items-center gap-2">
              <CheckCircle className="w-3 h-3 text-green-400" />
              <span className="text-slate-400">
                {flows.filter(f => f.verdict === 'FORWARDED').length}
              </span>
            </div>
            <div className="flex items-center gap-2">
              <AlertTriangle className="w-3 h-3 text-amber-400" />
              <span className="text-slate-400">
                {flows.filter(f => f.verdict === 'DROPPED').length}
              </span>
            </div>
            <div className="flex items-center gap-2">
              <XCircle className="w-3 h-3 text-red-400" />
              <span className="text-slate-400">
                {flows.filter(f => f.verdict === 'ERROR').length}
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* Flow Table */}
      <div className="flex-1 overflow-auto">
        <table className="w-full">
          <thead className="sticky top-0 bg-slate-900 border-b border-slate-800">
            <tr className="text-xs text-slate-400">
              <th className="text-left p-3 font-medium">Time</th>
              <th className="text-left p-3 font-medium">Source</th>
              <th className="text-center p-3 font-medium">→</th>
              <th className="text-left p-3 font-medium">Destination</th>
              <th className="text-left p-3 font-medium">Protocol</th>
              <th className="text-left p-3 font-medium">Path</th>
              <th className="text-center p-3 font-medium">Status</th>
              <th className="text-right p-3 font-medium">Latency</th>
              <th className="text-right p-3 font-medium">Size</th>
            </tr>
          </thead>
          <tbody>
            <AnimatePresence mode="popLayout">
              {filteredFlows.map((flow, index) => (
                <motion.tr
                  key={flow.id}
                  initial={{ opacity: 0, y: -10 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: 10 }}
                  transition={{ delay: index * 0.01 }}
                  className={`
                    border-b border-slate-800 hover:bg-slate-900 cursor-pointer transition-colors
                    ${selectedFlow === flow.id ? 'bg-slate-900' : ''}
                  `}
                  onClick={() => setSelectedFlow(flow.id === selectedFlow ? null : flow.id)}
                >
                  <td className="p-3 text-xs text-slate-500">
                    {format(new Date(flow.timestamp), 'HH:mm:ss.SSS')}
                  </td>
                  <td className="p-3">
                    <div className="text-xs">
                      <div className="text-white font-medium">{flow.source.service}</div>
                      <div className="text-slate-500">{flow.source.pod}</div>
                    </div>
                  </td>
                  <td className="p-3 text-center">
                    <ArrowRight className="w-4 h-4 text-slate-600 inline" />
                  </td>
                  <td className="p-3">
                    <div className="text-xs">
                      <div className="text-white font-medium">{flow.destination.service}</div>
                      <div className="text-slate-500">{flow.destination.pod}</div>
                    </div>
                  </td>
                  <td className="p-3">
                    <div className="flex items-center gap-2">
                      <span className="text-xs text-white">{flow.protocol}</span>
                      {flow.method && (
                        <span className="text-xs px-1.5 py-0.5 bg-slate-800 text-slate-400 rounded">
                          {flow.method}
                        </span>
                      )}
                    </div>
                  </td>
                  <td className="p-3 text-xs text-slate-400 font-mono">
                    {flow.path}
                  </td>
                  <td className="p-3">
                    <div className={`flex items-center justify-center gap-1 ${getVerdictColor(flow.verdict)}`}>
                      {getVerdictIcon(flow.verdict)}
                      <span className="text-xs">{flow.verdict}</span>
                    </div>
                  </td>
                  <td className="p-3 text-right">
                    <div className="flex items-center justify-end gap-1">
                      {getLatencyTrend(flow.latency)}
                      <span className="text-xs text-white">{flow.latency}ms</span>
                    </div>
                  </td>
                  <td className="p-3 text-right text-xs text-slate-400">
                    {formatBytes(flow.bytes)}
                  </td>
                </motion.tr>
              ))}
            </AnimatePresence>
          </tbody>
        </table>
      </div>

      {/* Selected Flow Details */}
      <AnimatePresence>
        {selectedFlow && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: 'auto', opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            className="border-t border-slate-800 bg-slate-900 p-4"
          >
            <div className="flex items-center justify-between mb-3">
              <h3 className="text-sm font-medium text-white">Flow Details</h3>
              <button
                onClick={() => setSelectedFlow(null)}
                className="text-slate-500 hover:text-white"
              >
                ✕
              </button>
            </div>
            <div className="grid grid-cols-3 gap-4 text-xs">
              <div>
                <span className="text-slate-500">Trace ID:</span>
                <div className="font-mono text-slate-300 mt-1">{selectedFlow}</div>
              </div>
              <div>
                <span className="text-slate-500">Duration:</span>
                <div className="text-white mt-1">
                  {flows.find(f => f.id === selectedFlow)?.latency}ms
                </div>
              </div>
              <div>
                <span className="text-slate-500">Status:</span>
                <div className="text-white mt-1">
                  {flows.find(f => f.id === selectedFlow)?.statusCode || 'N/A'}
                </div>
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}