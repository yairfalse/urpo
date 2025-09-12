// UI Constants - All magic numbers extracted for maintainability
// Following CLAUDE.md performance guidelines

// ============================================
// PERFORMANCE TARGETS (from CLAUDE.md)
// ============================================
export const PERFORMANCE = {
  STARTUP_TIME_MS: 200,          // Maximum startup time
  FRAME_TIME_MS: 16,             // 60fps target
  MEMORY_LIMIT_MB: 100,          // Maximum memory for 1M spans
  SPANS_PER_SECOND: 10000,       // Processing target
  SEARCH_TIME_MS: 1,             // Search across 100K traces
} as const;

// ============================================
// D3 VISUALIZATION CONSTANTS
// ============================================
export const D3_GRAPH = {
  // Force simulation
  FORCE_LINK_DISTANCE: 150,
  FORCE_CHARGE_STRENGTH: -500,
  FORCE_COLLISION_RADIUS: 40,
  
  // Zoom
  ZOOM_MIN_SCALE: 0.5,
  ZOOM_MAX_SCALE: 3,
  
  // Node sizing
  NODE_BASE_RADIUS: 20,
  NODE_MAX_BONUS_RADIUS: 20,
  NODE_SCALE_FACTOR: 100,
  
  // Animation
  DRAG_ALPHA_TARGET: 0.3,
  DRAG_ALPHA_MIN: 0,
  
  // Stroke widths
  LINK_MIN_WIDTH: 1,
  LINK_MAX_WIDTH: 5,
  LINK_SCALE_FACTOR: 10,
  NODE_STROKE_WIDTH: 2,
} as const;

// ============================================
// VIRTUALIZATION CONSTANTS
// ============================================
export const VIRTUALIZATION = {
  INITIAL_VISIBLE_ITEMS: 50,
  SPAN_ROW_HEIGHT: 32,           // Height of each span row in pixels
  BUFFER_ITEMS: 10,              // Extra items to render for smooth scrolling
  MAX_VISIBLE_SPANS: 50,         // Maximum spans to show initially
} as const;

// ============================================
// POLLING & UPDATES
// ============================================
export const POLLING = {
  METRICS_INTERVAL_MS: 1000,     // Poll metrics every second
  DEBOUNCE_DELAY_MS: 16,         // Debounce for 60fps
  BATCH_SIZE: 512,               // Batch processing size
} as const;

// ============================================
// THRESHOLDS & LIMITS
// ============================================
export const THRESHOLDS = {
  // Error rate thresholds
  ERROR_RATE_CRITICAL: 5,        // % error rate for critical status
  ERROR_RATE_DEGRADED: 1,        // % error rate for degraded status
  
  // Latency thresholds
  LATENCY_SLOW_MS: 1000,         // Mark as slow above this
  LATENCY_WARNING_MS: 200,       // Warning threshold
  LATENCY_FAST_MS: 50,           // Considered fast below this
  
  // Memory thresholds
  MEMORY_WARNING_MB: 50,         // Warning threshold
  MEMORY_CRITICAL_MB: 100,       // Critical threshold
  
  // CPU thresholds
  CPU_WARNING_PERCENT: 20,       // Warning threshold
  CPU_CRITICAL_PERCENT: 50,      // Critical threshold
} as const;

// ============================================
// LAYOUT & SIZING
// ============================================
export const LAYOUT = {
  // Header
  HEADER_HEIGHT: 64,             // Main header height in pixels
  
  // Panels
  SPAN_DETAILS_HEIGHT: 192,      // Selected span details panel (h-48)
  SERVICE_DETAILS_WIDTH: 320,    // Service details panel width (w-80)
  
  // Spacing
  PANEL_PADDING: 24,             // Standard panel padding (p-6)
  CARD_PADDING: 16,              // Card padding (p-4)
  
  // Icons
  ICON_SIZE_SM: 12,              // Small icons (w-3 h-3)
  ICON_SIZE_MD: 16,              // Medium icons (w-4 h-4)
  ICON_SIZE_LG: 20,              // Large icons (w-5 h-5)
  ICON_SIZE_XL: 32,              // Extra large icons (w-8 h-8)
} as const;

// ============================================
// ANIMATION DURATIONS
// ============================================
export const ANIMATION = {
  TRANSITION_FAST: 150,          // Fast transitions
  TRANSITION_NORMAL: 300,        // Normal transitions
  TRANSITION_SLOW: 500,          // Slow transitions
  PULSE_DURATION: 2000,          // Pulse animation cycle
} as const;

// ============================================
// DATA LIMITS
// ============================================
export const DATA_LIMITS = {
  MAX_TRACES_DISPLAY: 100,       // Maximum traces to display
  MAX_SPANS_PER_TRACE: 1000,     // Maximum spans per trace
  SPAN_POOL_SIZE: 10000,         // Object pool size for spans
  TRACE_HISTORY_LIMIT: 1000,     // Keep last N traces in memory
} as const;

// ============================================
// FORMAT CONSTANTS
// ============================================
export const FORMATS = {
  TIME_FORMAT: 'HH:mm:ss.SSS',
  DATE_FORMAT: 'yyyy-MM-dd',
  TRACE_ID_LENGTH: 32,
  SPAN_ID_LENGTH: 16,
} as const;

// ============================================
// Z-INDEX LAYERS
// ============================================
export const Z_INDEX = {
  BACKGROUND: 0,
  CONTENT: 1,
  OVERLAY: 10,
  MODAL: 100,
  TOOLTIP: 1000,
  NOTIFICATION: 10000,
} as const;

// Type exports for TypeScript
export type PerformanceConfig = typeof PERFORMANCE;
export type D3GraphConfig = typeof D3_GRAPH;
export type VirtualizationConfig = typeof VIRTUALIZATION;
export type ThresholdsConfig = typeof THRESHOLDS;