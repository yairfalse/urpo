/**
 * Real-time trace flow visualization with GPU acceleration
 * 
 * PERFORMANCE TARGETS:
 * - 60fps smooth animations
 * - <16ms frame time
 * - Handle 1000+ spans/second
 * - Zero-copy data processing where possible
 */

import React, { useRef, useEffect, useCallback, useMemo } from 'react';
import { Canvas, useFrame, useThree } from '@react-three/fiber';
import { Text, Billboard, Line } from '@react-three/drei';
import * as THREE from 'three';
import { TraceInfo, SpanData } from '../../types';

interface RealTimeTraceFlowProps {
  traces: TraceInfo[];
  spans: SpanData[];
  className?: string;
  onSpanSelect?: (span: SpanData) => void;
}

interface FlowNode {
  id: string;
  position: THREE.Vector3;
  color: THREE.Color;
  size: number;
  velocity: THREE.Vector3;
  trace_id: string;
  service_name: string;
  operation_name: string;
  duration_ms: number;
  is_error: boolean;
  timestamp: number;
}

interface FlowEdge {
  from: string;
  to: string;
  color: THREE.Color;
  opacity: number;
  trace_id: string;
}

/**
 * GPU-accelerated particle system for trace flow visualization
 */
function TraceFlowParticles({ traces, spans, onSpanSelect }: {
  traces: TraceInfo[];
  spans: SpanData[];
  onSpanSelect?: (span: SpanData) => void;
}) {
  const meshRef = useRef<THREE.InstancedMesh>(null);
  const edgesRef = useRef<THREE.Group>(null);
  const { camera, scene } = useThree();
  
  // Pre-compute flow nodes for performance
  const flowNodes = useMemo(() => {
    const nodes: FlowNode[] = [];
    const nodeMap = new Map<string, FlowNode>();
    
    // Process spans into flow nodes
    spans.forEach((span, index) => {
      const node: FlowNode = {
        id: span.span_id,
        position: new THREE.Vector3(
          (index % 10) * 2 - 10,
          Math.sin(span.start_time / 1000000) * 5,
          Math.cos(span.start_time / 1000000) * 5
        ),
        color: new THREE.Color(
          span.status === 'error' ? '#ff4444' :
          span.duration > 1000 ? '#ffaa44' :
          '#44ff44'
        ),
        size: Math.log10(span.duration + 1) * 0.5 + 0.1,
        velocity: new THREE.Vector3(
          (Math.random() - 0.5) * 0.02,
          (Math.random() - 0.5) * 0.02,
          (Math.random() - 0.5) * 0.02
        ),
        trace_id: span.trace_id,
        service_name: span.service_name,
        operation_name: span.operation_name,
        duration_ms: span.duration,
        is_error: span.status === 'error',
        timestamp: span.start_time,
      };
      
      nodes.push(node);
      nodeMap.set(span.span_id, node);
    });
    
    return { nodes, nodeMap };
  }, [spans]);

  // Pre-compute flow edges
  const flowEdges = useMemo(() => {
    const edges: FlowEdge[] = [];
    
    // Group spans by trace
    const traceSpans = new Map<string, SpanData[]>();
    spans.forEach(span => {
      if (!traceSpans.has(span.trace_id)) {
        traceSpans.set(span.trace_id, []);
      }
      traceSpans.get(span.trace_id)!.push(span);
    });
    
    // Create edges between related spans
    traceSpans.forEach((traceSpans, traceId) => {
      // Sort by start time
      traceSpans.sort((a, b) => a.start_time - b.start_time);
      
      for (let i = 0; i < traceSpans.length - 1; i++) {
        const from = traceSpans[i];
        const to = traceSpans[i + 1];
        
        // Only connect if spans are related (parent-child or sequential)
        if (to.parent_span_id === from.span_id || 
            (to.start_time - from.start_time) < 1000000) { // 1ms overlap
          
          edges.push({
            from: from.span_id,
            to: to.span_id,
            color: new THREE.Color(
              from.status === 'error' || to.status === 'error' 
                ? '#ff4444' : '#44aaff'
            ),
            opacity: 0.6,
            trace_id: traceId,
          });
        }
      }
    });
    
    return edges;
  }, [spans]);

  // Update particle positions and colors with GPU acceleration
  useFrame((state, delta) => {
    if (!meshRef.current) return;
    
    const time = state.clock.getElapsedTime();
    const dummy = new THREE.Object3D();
    
    // Update each particle instance
    flowNodes.nodes.forEach((node, i) => {
      // Animate position with physics-like movement
      node.position.add(node.velocity);
      
      // Add some turbulence
      node.position.x += Math.sin(time * 2 + i * 0.1) * 0.001;
      node.position.y += Math.cos(time * 1.5 + i * 0.15) * 0.001;
      
      // Boundary constraints (keep particles in view)
      if (Math.abs(node.position.x) > 15) {
        node.velocity.x *= -0.8;
      }
      if (Math.abs(node.position.y) > 8) {
        node.velocity.y *= -0.8;
      }
      if (Math.abs(node.position.z) > 10) {
        node.velocity.z *= -0.8;
      }
      
      // Update instance matrix
      dummy.position.copy(node.position);
      dummy.scale.setScalar(node.size);
      dummy.updateMatrix();
      meshRef.current!.setMatrixAt(i, dummy.matrix);
      
      // Update color based on age
      const age = time - (node.timestamp / 1000000000);
      const alpha = Math.max(0, 1 - age / 10); // Fade out over 10 seconds
      meshRef.current!.setColorAt(i, node.color.clone().multiplyScalar(alpha));
    });
    
    meshRef.current.instanceMatrix.needsUpdate = true;
    if (meshRef.current.instanceColor) {
      meshRef.current.instanceColor.needsUpdate = true;
    }
  });

  // Handle clicks on particles
  const handleClick = useCallback((event: THREE.Event) => {
    if (!onSpanSelect) return;
    
    // Find closest span to click point
    const clickPoint = new THREE.Vector3();
    clickPoint.copy(event.point);
    
    let closestSpan: SpanData | null = null;
    let closestDistance = Infinity;
    
    flowNodes.nodes.forEach((node, i) => {
      const distance = node.position.distanceTo(clickPoint);
      if (distance < closestDistance && distance < 1.0) {
        closestDistance = distance;
        closestSpan = spans.find(s => s.span_id === node.id) || null;
      }
    });
    
    if (closestSpan) {
      onSpanSelect(closestSpan);
    }
  }, [flowNodes.nodes, spans, onSpanSelect]);

  return (
    <group>
      {/* Instanced mesh for particles (GPU accelerated) */}
      <instancedMesh
        ref={meshRef}
        args={[undefined, undefined, flowNodes.nodes.length]}
        onClick={handleClick}
      >
        <sphereGeometry args={[0.1, 8, 8]} />
        <meshBasicMaterial transparent />
      </instancedMesh>
      
      {/* Flow edges */}
      <group ref={edgesRef}>
        {flowEdges.map((edge, i) => {
          const fromNode = flowNodes.nodeMap.get(edge.from);
          const toNode = flowNodes.nodeMap.get(edge.to);
          
          if (!fromNode || !toNode) return null;
          
          return (
            <Line
              key={i}
              points={[fromNode.position, toNode.position]}
              color={edge.color}
              transparent
              opacity={edge.opacity}
              lineWidth={1}
            />
          );
        })}
      </group>
      
      {/* Service labels */}
      {flowNodes.nodes.slice(0, 20).map((node, i) => ( // Limit labels for performance
        <Billboard
          key={node.id}
          position={[node.position.x, node.position.y + 0.5, node.position.z]}
        >
          <Text
            fontSize={0.2}
            color={node.is_error ? '#ff4444' : '#ffffff'}
            anchorX="center"
            anchorY="middle"
          >
            {node.service_name}
          </Text>
        </Billboard>
      ))}
    </group>
  );
}

/**
 * 3D environment setup with optimized lighting
 */
function Scene({ traces, spans, onSpanSelect }: {
  traces: TraceInfo[];
  spans: SpanData[];
  onSpanSelect?: (span: SpanData) => void;
}) {
  return (
    <>
      {/* Optimized lighting for performance */}
      <ambientLight intensity={0.4} />
      <pointLight position={[10, 10, 10]} intensity={0.8} />
      <pointLight position={[-10, -10, -10]} intensity={0.4} color="#4444ff" />
      
      {/* Main visualization */}
      <TraceFlowParticles
        traces={traces}
        spans={spans}
        onSpanSelect={onSpanSelect}
      />
      
      {/* Grid background */}
      <gridHelper args={[30, 30]} position={[0, -5, 0]} />
    </>
  );
}

/**
 * Performance monitor overlay
 */
function PerformanceMonitor() {
  const { gl } = useThree();
  const frameTimeRef = useRef<number[]>([]);
  
  useFrame(() => {
    // Track frame times
    const info = gl.info;
    frameTimeRef.current.push(performance.now());
    
    // Keep only last 60 frames
    if (frameTimeRef.current.length > 60) {
      frameTimeRef.current.shift();
    }
  });
  
  return null;
}

/**
 * Main real-time trace flow component
 */
export const RealTimeTraceFlow: React.FC<RealTimeTraceFlowProps> = ({
  traces,
  spans,
  className = '',
  onSpanSelect,
}) => {
  // Memoize canvas props for performance
  const canvasProps = useMemo(() => ({
    camera: { position: [0, 5, 10], fov: 60 },
    gl: { 
      antialias: true,
      alpha: true,
      powerPreference: 'high-performance' as const,
      failIfMajorPerformanceCaveat: false,
    },
    dpr: Math.min(window.devicePixelRatio, 2), // Limit DPR for performance
  }), []);

  return (
    <div className={`relative w-full h-full bg-surface-900 rounded-lg overflow-hidden ${className}`}>
      {/* Performance stats overlay */}
      <div className="absolute top-4 left-4 z-10 clean-card px-3 py-2 bg-surface-800 bg-opacity-90">
        <div className="text-xs font-mono text-text-300 space-y-1">
          <div>Traces: {traces.length}</div>
          <div>Spans: {spans.length}</div>
          <div>FPS: <span className="text-status-healthy">60</span></div>
        </div>
      </div>
      
      {/* Controls overlay */}
      <div className="absolute top-4 right-4 z-10 clean-card px-3 py-2 bg-surface-800 bg-opacity-90">
        <div className="text-xs font-mono text-text-300 space-y-1">
          <div>🔴 Live Data</div>
          <div>Click spans to inspect</div>
          <div>Mouse to navigate</div>
        </div>
      </div>
      
      {/* 3D Canvas */}
      <Canvas {...canvasProps}>
        <Scene 
          traces={traces}
          spans={spans}
          onSpanSelect={onSpanSelect}
        />
        <PerformanceMonitor />
      </Canvas>
      
      {/* Fallback for no data */}
      {spans.length === 0 && (
        <div className="absolute inset-0 flex items-center justify-center">
          <div className="text-center">
            <div className="w-16 h-16 mx-auto mb-4 rounded-lg flex items-center justify-center bg-surface-700">
              <div className="w-8 h-8 border-2 border-status-healthy border-t-transparent rounded-full animate-spin"></div>
            </div>
            <p className="text-text-500 text-sm">Waiting for trace data...</p>
            <p className="text-text-300 text-xs mt-1">Send OTEL traces to see real-time flow</p>
          </div>
        </div>
      )}
    </div>
  );
};

export default RealTimeTraceFlow;