/**
 * Design System Tokens
 *
 * Professional design tokens inspired by Linear/Vercel aesthetics
 * These provide the foundation for a polished, modern UI
 */

// ============================================================================
// COLOR SYSTEM
// ============================================================================

export const colors = {
  // Base colors - minimal and elegant
  dark: {
    0: '#000000',      // Pure black
    50: '#0A0A0A',     // Background
    100: '#111111',    // Surface
    150: '#1A1A1A',    // Surface elevated
    200: '#262626',    // Border
    300: '#404040',    // Border hover
    400: '#525252',    // Muted
    500: '#737373',    // Subtle
    600: '#A3A3A3',    // Placeholder
  },

  light: {
    50: '#FFFFFF',     // Pure white
    100: '#FAFAFA',    // Text primary
    200: '#E5E5E5',    // Text secondary
    300: '#D4D4D4',    // Text tertiary
    400: '#A3A3A3',    // Text muted
    500: '#737373',    // Text disabled
    600: '#525252',    // Text subtle
  },

  // Accent colors - vibrant but professional
  brand: {
    blue: '#0096FF',      // Primary brand
    cyan: '#00D4FF',      // Secondary accent
    purple: '#8B5CF6',    // Tertiary
    pink: '#EC4899',      // Quaternary
  },

  // Data visualization colors
  data: {
    blue: '#3B82F6',      // Primary data
    cyan: '#06B6D4',      // Secondary data
    green: '#10B981',     // Success
    yellow: '#F59E0B',    // Warning
    orange: '#F97316',    // Alert
    red: '#EF4444',       // Error
    purple: '#8B5CF6',    // Special
    pink: '#EC4899',      // Highlight
  },

  // Semantic colors
  semantic: {
    success: '#10B981',
    warning: '#F59E0B',
    error: '#EF4444',
    info: '#3B82F6',
  },

  // Special effects
  glow: {
    blue: 'rgba(0, 150, 255, 0.5)',
    cyan: 'rgba(0, 212, 255, 0.5)',
    purple: 'rgba(139, 92, 246, 0.5)',
    green: 'rgba(16, 185, 129, 0.5)',
  },
} as const;

// ============================================================================
// TYPOGRAPHY
// ============================================================================

export const typography = {
  // Font families
  fonts: {
    sans: 'Inter, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
    mono: 'JetBrains Mono, Consolas, "Courier New", monospace',
  },

  // Font sizes with line heights
  sizes: {
    xs: { size: '0.75rem', height: '1rem' },      // 12px / 16px
    sm: { size: '0.875rem', height: '1.25rem' },   // 14px / 20px
    base: { size: '1rem', height: '1.5rem' },      // 16px / 24px
    lg: { size: '1.125rem', height: '1.75rem' },   // 18px / 28px
    xl: { size: '1.25rem', height: '1.75rem' },    // 20px / 28px
    '2xl': { size: '1.5rem', height: '2rem' },     // 24px / 32px
    '3xl': { size: '1.875rem', height: '2.25rem' }, // 30px / 36px
    '4xl': { size: '2.25rem', height: '2.5rem' },   // 36px / 40px
  },

  // Font weights
  weights: {
    normal: 400,
    medium: 500,
    semibold: 600,
    bold: 700,
  },
} as const;

// ============================================================================
// SPACING
// ============================================================================

export const spacing = {
  0: '0',
  px: '1px',
  0.5: '0.125rem',  // 2px
  1: '0.25rem',     // 4px
  1.5: '0.375rem',  // 6px
  2: '0.5rem',      // 8px
  2.5: '0.625rem',  // 10px
  3: '0.75rem',     // 12px
  4: '1rem',        // 16px
  5: '1.25rem',     // 20px
  6: '1.5rem',      // 24px
  8: '2rem',        // 32px
  10: '2.5rem',     // 40px
  12: '3rem',       // 48px
  16: '4rem',       // 64px
  20: '5rem',       // 80px
  24: '6rem',       // 96px
} as const;

// ============================================================================
// ANIMATION
// ============================================================================

export const animation = {
  // Duration
  duration: {
    instant: 0,
    fast: 150,
    normal: 250,
    slow: 350,
    slower: 500,
  },

  // Easing curves
  easing: {
    linear: 'linear',
    ease: 'ease',
    easeIn: 'ease-in',
    easeOut: 'ease-out',
    easeInOut: 'ease-in-out',
    spring: 'cubic-bezier(0.25, 0.46, 0.45, 0.94)',
    bounce: 'cubic-bezier(0.68, -0.55, 0.265, 1.55)',
  },

  // Spring configs for Framer Motion
  spring: {
    smooth: {
      type: 'spring',
      stiffness: 280,
      damping: 30,
    },
    bouncy: {
      type: 'spring',
      stiffness: 400,
      damping: 20,
    },
    stiff: {
      type: 'spring',
      stiffness: 500,
      damping: 35,
    },
  },
} as const;

// ============================================================================
// SHADOWS
// ============================================================================

export const shadows = {
  none: 'none',
  sm: '0 1px 2px 0 rgba(0, 0, 0, 0.5)',
  base: '0 1px 3px 0 rgba(0, 0, 0, 0.5), 0 1px 2px 0 rgba(0, 0, 0, 0.06)',
  md: '0 4px 6px -1px rgba(0, 0, 0, 0.5), 0 2px 4px -1px rgba(0, 0, 0, 0.06)',
  lg: '0 10px 15px -3px rgba(0, 0, 0, 0.5), 0 4px 6px -2px rgba(0, 0, 0, 0.05)',
  xl: '0 20px 25px -5px rgba(0, 0, 0, 0.5), 0 10px 10px -5px rgba(0, 0, 0, 0.04)',
  '2xl': '0 25px 50px -12px rgba(0, 0, 0, 0.75)',

  // Glow effects
  glow: {
    sm: `0 0 10px ${colors.glow.blue}`,
    md: `0 0 20px ${colors.glow.blue}`,
    lg: `0 0 30px ${colors.glow.blue}`,
  },

  // Inset shadows for depth
  inner: 'inset 0 2px 4px 0 rgba(0, 0, 0, 0.06)',
  innerLg: 'inset 0 4px 6px -1px rgba(0, 0, 0, 0.1)',
} as const;

// ============================================================================
// BORDERS
// ============================================================================

export const borders = {
  radius: {
    none: '0',
    sm: '0.125rem',   // 2px
    base: '0.25rem',  // 4px
    md: '0.375rem',   // 6px
    lg: '0.5rem',     // 8px
    xl: '0.75rem',    // 12px
    '2xl': '1rem',    // 16px
    full: '9999px',
  },

  width: {
    0: '0',
    1: '1px',
    2: '2px',
    4: '4px',
  },
} as const;

// ============================================================================
// Z-INDEX SCALE
// ============================================================================

export const zIndex = {
  hide: -1,
  base: 0,
  dropdown: 10,
  sticky: 20,
  overlay: 30,
  modal: 40,
  popover: 50,
  tooltip: 60,
  notification: 70,
} as const;

// ============================================================================
// BREAKPOINTS
// ============================================================================

export const breakpoints = {
  sm: '640px',
  md: '768px',
  lg: '1024px',
  xl: '1280px',
  '2xl': '1536px',
} as const;

// ============================================================================
// EFFECTS
// ============================================================================

export const effects = {
  // Glassmorphism
  glass: {
    light: 'backdrop-filter: blur(12px); background: rgba(255, 255, 255, 0.05);',
    dark: 'backdrop-filter: blur(12px); background: rgba(0, 0, 0, 0.3);',
  },

  // Gradients
  gradients: {
    brand: `linear-gradient(135deg, ${colors.brand.blue} 0%, ${colors.brand.cyan} 100%)`,
    dark: `linear-gradient(135deg, ${colors.dark[50]} 0%, ${colors.dark[100]} 100%)`,
    glow: `radial-gradient(circle at center, ${colors.glow.blue} 0%, transparent 70%)`,
  },

  // Text gradient
  textGradient: {
    brand: `background: linear-gradient(135deg, ${colors.brand.blue} 0%, ${colors.brand.cyan} 100%); -webkit-background-clip: text; -webkit-text-fill-color: transparent;`,
  },
} as const;

// ============================================================================
// COMPOSITE STYLES
// ============================================================================

export const components = {
  // Button styles
  button: {
    base: `
      px-4 py-2 rounded-lg font-medium
      transition-all duration-150
      focus:outline-none focus:ring-2 focus:ring-offset-2
    `,
    primary: `
      bg-gradient-to-r from-brand-blue to-brand-cyan
      text-white shadow-lg shadow-blue-500/25
      hover:shadow-xl hover:shadow-blue-500/40
      active:scale-[0.98]
    `,
    secondary: `
      bg-dark-100 border border-dark-300
      text-light-200
      hover:bg-dark-150 hover:border-dark-400
      active:scale-[0.98]
    `,
    ghost: `
      text-light-400
      hover:text-light-200 hover:bg-dark-100
      active:scale-[0.98]
    `,
  },

  // Input styles
  input: {
    base: `
      w-full px-4 py-2 rounded-lg
      bg-dark-100 border border-dark-300
      text-light-100 placeholder:text-light-500
      focus:outline-none focus:ring-2 focus:ring-brand-blue/50 focus:border-brand-blue
      transition-all duration-150
    `,
  },

  // Card styles
  card: {
    base: `
      bg-dark-100 border border-dark-200
      rounded-xl p-6
      shadow-xl
    `,
    hover: `
      hover:border-dark-300 hover:shadow-2xl
      transition-all duration-250
    `,
  },
} as const;