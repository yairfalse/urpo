import { useMemo, useState, useRef, useCallback, memo } from 'react';
import { Flame, Download } from 'lucide-react';

interface SpanData {
  span_id: string;
  trace_id: string;
  parent_span_id?: string;
  service_name: string;
  operation_name: string;
  start_time: number;
  duration: number;
  status: 'ok' | 'error' | 'cancelled';
  children?: SpanData[];
}

interface SpanFlamegraphProps {
  spans: SpanData[];
  traceId: string;
}

interface FlameNode {
  span: SpanData;
  x: number;
  y: number;
  width: number;
  height: number;
  depth: number;
  selfTime: number;
  totalTime: number;
}

const COLORS = {
  ok: ['#10b981', '#34d399', '#6ee7b7', '#a7f3d0'], // Green shades
  error: ['#ef4444', '#f87171', '#fca5a5', '#fecaca'], // Red shades
  cancelled: ['#f59e0b', '#fbbf24', '#fcd34d', '#fde68a'], // Yellow shades
};

const SERVICE_COLORS = [
  '#5B8FF9', '#5AD8A6', '#975FE4', '#FF9845', '#5DCFFF',
  '#FF6B9D', '#3BCBB0', '#FFC53D', '#F6465D', '#8B5CF6',
];

const SpanFlamegraphImpl = ({ spans, traceId }: SpanFlamegraphProps) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [hoveredNode, setHoveredNode] = useState<FlameNode | null>(null);
  const [selectedNode, setSelectedNode] = useState<FlameNode | null>(null);
  const [dimensions, setDimensions] = useState({ width: 1200, height: 600 });

  // Build span tree and calculate flame data
  const flameData = useMemo(() => {
    if (!spans.length) return { nodes: [], maxDepth: 0, minTime: 0, maxTime: 1 };

    // Build span tree
    const spanMap = new Map<string, SpanData>();
    const rootSpans: SpanData[] = [];

    spans.forEach(span => {
      spanMap.set(span.span_id, { ...span, children: [] });
    });

    spans.forEach(span => {
      const node = spanMap.get(span.span_id)!;
      if (span.parent_span_id) {
        const parent = spanMap.get(span.parent_span_id);
        if (parent) {
          parent.children!.push(node);
        } else {
          rootSpans.push(node);
        }
      } else {
        rootSpans.push(node);
      }
    });

    // Calculate timing
    const minTime = Math.min(...spans.map(s => s.start_time));
    const maxTime = spans.reduce((max, s) => {
      const endTime = s.start_time + s.duration;
      return endTime > max ? endTime : max;
    }, -Infinity);
    const totalDuration = maxTime - minTime;

    // Build flamegraph nodes (inverted - root at bottom)
    const nodes: FlameNode[] = [];
    let maxDepth = 0;

    const ROW_HEIGHT = 20;
    const PADDING = 2;

    function buildFlameNodes(span: SpanData, depth: number, availableWidth: number, offsetX: number) {
      maxDepth = Math.max(maxDepth, depth);

      // Calculate self time (time not spent in children)
      const childrenTime = (span.children || []).reduce((sum, child) => sum + child.duration, 0);
      const selfTime = span.duration - childrenTime;

      // Calculate width based on duration
      const width = (span.duration / totalDuration) * availableWidth;
      const x = offsetX + ((span.start_time - minTime) / totalDuration) * availableWidth;

      // Y position (inverted - root at bottom)
      const y = dimensions.height - (depth + 1) * (ROW_HEIGHT + PADDING);

      nodes.push({
        span,
        x,
        y,
        width,
        height: ROW_HEIGHT,
        depth,
        selfTime,
        totalTime: span.duration,
      });

      // Process children
      if (span.children && span.children.length > 0) {
        span.children.forEach(child => {
          buildFlameNodes(child, depth + 1, availableWidth, 0);
        });
      }
    }

    rootSpans.forEach(root => {
      buildFlameNodes(root, 0, dimensions.width, 0);
    });

    return { nodes, maxDepth, minTime, maxTime };
  }, [spans, dimensions.width, dimensions.height]);

  // Draw flamegraph
  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Clear canvas
    ctx.clearRect(0, 0, dimensions.width, dimensions.height);

    // Service color map
    const serviceColorMap = new Map<string, number>();

    // Draw nodes
    flameData.nodes.forEach(node => {
      const isHovered = hoveredNode?.span.span_id === node.span.span_id;
      const isSelected = selectedNode?.span.span_id === node.span.span_id;

      // Get color based on service and status
      let color: string;
      if (!serviceColorMap.has(node.span.service_name)) {
        serviceColorMap.set(node.span.service_name, serviceColorMap.size);
      }
      const serviceIdx = serviceColorMap.get(node.span.service_name)!;
      color = SERVICE_COLORS[serviceIdx % SERVICE_COLORS.length];

      // Apply status tint
      if (node.span.status === 'error') {
        color = COLORS.error[node.depth % COLORS.error.length];
      } else if (node.span.status === 'cancelled') {
        color = COLORS.cancelled[node.depth % COLORS.cancelled.length];
      }

      // Draw rectangle
      ctx.fillStyle = color;
      if (isHovered || isSelected) {
        ctx.fillStyle = adjustColor(color, 20); // Brighten
      }

      ctx.fillRect(node.x, node.y, node.width, node.height);

      // Draw border
      ctx.strokeStyle = isSelected ? '#000' : 'rgba(0,0,0,0.2)';
      ctx.lineWidth = isSelected ? 2 : 0.5;
      ctx.strokeRect(node.x, node.y, node.width, node.height);

      // Draw text (only if wide enough)
      if (node.width > 40) {
        ctx.fillStyle = '#000';
        ctx.font = '11px monospace';
        ctx.textBaseline = 'middle';

        const text = `${node.span.operation_name} (${formatDuration(node.totalTime)})`;
        const maxTextWidth = node.width - 8;

        ctx.save();
        ctx.beginPath();
        ctx.rect(node.x + 4, node.y, maxTextWidth, node.height);
        ctx.clip();

        ctx.fillText(text, node.x + 4, node.y + node.height / 2);
        ctx.restore();
      }
    });
  }, [flameData, hoveredNode, selectedNode, dimensions]);

  // Handle mouse move
  const handleMouseMove = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    const node = flameData.nodes.find(n =>
      x >= n.x && x <= n.x + n.width && y >= n.y && y <= n.y + n.height
    );

    setHoveredNode(node || null);
  }, [flameData]);

  // Handle click
  const handleClick = useCallback(() => {
    if (hoveredNode) {
      setSelectedNode(hoveredNode);
    }
  }, [hoveredNode]);

  // Export as image
  const exportImage = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const link = document.createElement('a');
    link.download = `flamegraph-${traceId}.png`;
    link.href = canvas.toDataURL();
    link.click();
  }, [traceId]);

  // Redraw when data changes
  useMemo(() => {
    draw();
  }, [draw]);

  return (
    <div ref={containerRef} className="relative w-full h-full bg-white rounded-lg border border-gray-200 overflow-hidden">
      {/* Header */}
      <div className="absolute top-0 left-0 right-0 z-10 bg-white border-b border-gray-200 p-3 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Flame className="w-5 h-5 text-orange-500" />
          <h3 className="font-semibold text-gray-900">Span Flamegraph</h3>
          <span className="text-sm text-gray-500">
            {flameData.nodes.length} spans • {flameData.maxDepth + 1} levels
          </span>
        </div>
        <button
          onClick={exportImage}
          className="flex items-center gap-2 px-3 py-1.5 text-sm bg-gray-100 hover:bg-gray-200 rounded"
        >
          <Download className="w-4 h-4" />
          Export PNG
        </button>
      </div>

      {/* Canvas */}
      <canvas
        ref={canvasRef}
        width={dimensions.width}
        height={dimensions.height}
        onMouseMove={handleMouseMove}
        onClick={handleClick}
        className="mt-14 cursor-pointer"
        style={{ display: 'block' }}
      />

      {/* Tooltip */}
      {hoveredNode && (
        <div className="absolute bottom-4 left-4 bg-black/90 text-white text-sm rounded-lg p-3 max-w-md">
          <div className="font-semibold mb-1">{hoveredNode.span.operation_name}</div>
          <div className="text-gray-300 text-xs space-y-0.5">
            <div>Service: {hoveredNode.span.service_name}</div>
            <div>Total Time: {formatDuration(hoveredNode.totalTime)}</div>
            <div>Self Time: {formatDuration(hoveredNode.selfTime)}</div>
            <div>Status: {hoveredNode.span.status}</div>
            <div>Depth: {hoveredNode.depth}</div>
          </div>
        </div>
      )}

      {/* Selected node details */}
      {selectedNode && (
        <div className="absolute top-16 right-4 bg-white border border-gray-200 rounded-lg p-4 max-w-sm shadow-lg">
          <div className="flex items-center justify-between mb-2">
            <h4 className="font-semibold text-gray-900">Selected Span</h4>
            <button
              onClick={() => setSelectedNode(null)}
              className="text-gray-400 hover:text-gray-600"
            >
              ✕
            </button>
          </div>
          <div className="space-y-2 text-sm">
            <div>
              <div className="text-gray-500">Operation</div>
              <div className="font-mono text-gray-900">{selectedNode.span.operation_name}</div>
            </div>
            <div>
              <div className="text-gray-500">Service</div>
              <div className="font-mono text-gray-900">{selectedNode.span.service_name}</div>
            </div>
            <div className="grid grid-cols-2 gap-2">
              <div>
                <div className="text-gray-500">Total</div>
                <div className="font-mono text-gray-900">{formatDuration(selectedNode.totalTime)}</div>
              </div>
              <div>
                <div className="text-gray-500">Self</div>
                <div className="font-mono text-gray-900">{formatDuration(selectedNode.selfTime)}</div>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

// Utility functions
function formatDuration(ns: number): string {
  if (ns < 1000) return `${ns}ns`;
  if (ns < 1000000) return `${(ns / 1000).toFixed(2)}μs`;
  if (ns < 1000000000) return `${(ns / 1000000).toFixed(2)}ms`;
  return `${(ns / 1000000000).toFixed(2)}s`;
}

function adjustColor(color: string, amount: number): string {
  const num = parseInt(color.replace('#', ''), 16);
  const r = Math.min(255, ((num >> 16) & 0xff) + amount);
  const g = Math.min(255, ((num >> 8) & 0xff) + amount);
  const b = Math.min(255, (num & 0xff) + amount);
  return `#${((r << 16) | (g << 8) | b).toString(16).padStart(6, '0')}`;
}

export const SpanFlamegraph = memo(SpanFlamegraphImpl);
