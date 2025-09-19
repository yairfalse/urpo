// Enterprise-grade professional color system
// Used by both Tailwind classes and Canvas components

export const COLORS = {
  // Professional backgrounds
  background: {
    primary: '#FFFFFF',
    secondary: '#F8FAFC',
    elevated: '#F1F5F9',
    dark: '#0F172A', // Dark mode background
  },

  // Professional surface colors for cards/panels
  surface: {
    50: '#FFFFFF',
    100: '#F8FAFC',
    200: '#F1F5F9',
    300: '#E2E8F0',
    400: '#CBD5E1',
  },

  // Professional text hierarchy
  text: {
    950: '#0F172A', // Deepest text
    900: '#1E293B', // Primary text
    700: '#475569', // Secondary text
    500: '#64748B', // Muted text
    300: '#94A3B8', // Placeholder
    100: '#F1F5F9', // Light (on dark)
  },

  // Enterprise status colors
  status: {
    healthy: '#059669', // Professional green
    warning: '#D97706', // Amber orange
    error: '#DC2626',   // Clean red
    info: '#0284C7',    // Professional blue
  },
  
  // Professional canvas colors (for graphs/visualizations)
  canvas: {
    background: '#F8FAFC',
    grid: '#E2E8F0',
    node: {
      default: '#F1F5F9',
      hover: '#E2E8F0',
      selected: '#475569',
      border: '#CBD5E1',
    },
    edge: {
      default: '#CBD5E1',
      hover: '#94A3B8',
      selected: '#64748B',
      arrow: '#94A3B8',
    },
    text: {
      primary: '#0F172A',
      secondary: '#64748B',
      label: '#475569',
    }
  }
} as const;

// Export for use in Canvas/D3 components
export const getCanvasColors = () => ({
  bg: COLORS.canvas.background,
  grid: COLORS.canvas.grid,
  node: COLORS.canvas.node.default,
  nodeBorder: COLORS.canvas.node.border,
  nodeSelected: COLORS.canvas.node.selected,
  edge: COLORS.canvas.edge.default,
  edgeSelected: COLORS.canvas.edge.selected,
  text: COLORS.canvas.text.primary,
  textSecondary: COLORS.canvas.text.secondary,
  error: COLORS.status.error,
  warning: COLORS.status.warning,
  healthy: COLORS.status.healthy,
});

// Professional Tailwind class mappings
export const tw = {
  // Backgrounds
  bgPrimary: 'bg-surface-50',
  bgSecondary: 'bg-surface-100',
  bgElevated: 'bg-surface-200',

  // Text
  textPrimary: 'text-text-950',
  textSecondary: 'text-text-700',
  textMuted: 'text-text-500',
  textPlaceholder: 'text-text-300',

  // Borders
  border: 'border-surface-300',
  borderHover: 'border-surface-400',

  // Status
  statusHealthy: 'text-status-healthy',
  statusWarning: 'text-status-warning',
  statusError: 'text-status-error',
  statusInfo: 'text-status-info',

  // Professional patterns
  card: 'bg-surface-50 border border-surface-300 rounded-md shadow-sm',
  cardHover: 'hover:border-surface-400 hover:shadow-md',
  button: 'bg-surface-50 border border-surface-300 text-text-700 hover:bg-surface-100',
  buttonActive: 'bg-text-950 text-surface-50',
} as const;