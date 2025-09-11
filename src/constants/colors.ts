// Single source of truth for all colors
// Used by both Tailwind classes and Canvas components

export const COLORS = {
  // Backgrounds
  background: {
    primary: '#FFFFFF',
    secondary: '#F9FAFB',
    elevated: '#F3F4F6',
    dark: '#111827', // Dark mode background
  },
  
  // Surface colors for cards/panels
  surface: {
    50: '#FFFFFF',
    100: '#F9FAFB', 
    200: '#F3F4F6',
    300: '#E5E7EB',
    400: '#D1D5DB',
  },
  
  // Text colors
  text: {
    900: '#111827', // Primary text
    700: '#374151', // Secondary text  
    500: '#6B7280', // Muted text
    300: '#9CA3AF', // Placeholder
    100: '#F3F4F6', // Light (on dark)
  },
  
  // Status colors
  status: {
    healthy: '#6B7280', // Neutral gray (not green)
    warning: '#F59E0B', // Amber
    error: '#EF4444',   // Red
    info: '#6B7280',    // Neutral gray (not blue)
  },
  
  // Canvas specific (for graphs/visualizations)
  canvas: {
    background: '#FAFAFA',
    grid: '#E5E7EB',
    node: {
      default: '#F3F4F6',
      hover: '#E5E7EB',
      selected: '#374151',
      border: '#D1D5DB',
    },
    edge: {
      default: '#D1D5DB',
      hover: '#9CA3AF',
      selected: '#6B7280',
      arrow: '#9CA3AF',
    },
    text: {
      primary: '#111827',
      secondary: '#6B7280',
      label: '#374151',
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

// Tailwind class mappings for consistency
export const tw = {
  // Backgrounds
  bgPrimary: 'bg-surface-50',
  bgSecondary: 'bg-surface-100',
  bgElevated: 'bg-surface-200',
  
  // Text
  textPrimary: 'text-text-900',
  textSecondary: 'text-text-700',
  textMuted: 'text-text-500',
  textPlaceholder: 'text-text-300',
  
  // Borders
  border: 'border-surface-300',
  borderHover: 'border-surface-400',
  
  // Status
  statusHealthy: 'text-text-500',
  statusWarning: 'text-status-warning',
  statusError: 'text-status-error',
  
  // Common patterns
  card: 'bg-surface-50 border border-surface-300 rounded-lg',
  cardHover: 'hover:border-surface-400 hover:shadow-md',
  button: 'bg-surface-50 border border-surface-300 text-text-700 hover:bg-surface-100',
  buttonActive: 'bg-text-900 text-surface-50',
} as const;