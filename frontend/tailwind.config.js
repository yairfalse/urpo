/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      // Professional observability platform colors inspired by Grafana, Datadog, Hubble
      colors: {
        // Sharp, high-contrast dark palette
        'dark': {
          0: '#0a0a0a',      // Pure black - maximum contrast
          50: '#0f0f0f',     // Near black - main background
          100: '#141414',    // Slightly lighter - card background
          150: '#1a1a1a',    // Elevated surfaces
          200: '#242424',    // Hover states
          300: '#2e2e2e',    // Active states
          400: '#383838',    // Borders
        },
        // Light accents on dark
        'light': {
          50: '#F9FAFB',     // Pure white text
          100: '#F3F4F6',    // Primary text
          200: '#E5E7EB',    // Secondary text
          300: '#D1D5DB',    // Muted text
          400: '#9CA3AF',    // Placeholder
          500: '#6B7280',    // Disabled
          600: '#4B5563',    // Very muted
        },
        // Data visualization palette - inspired by observability standards
        'data': {
          blue: '#5B8FF9',     // Primary metric
          green: '#5AD8A6',    // Success/healthy
          orange: '#FF9845',   // Warning
          red: '#F6465D',      // Error/critical
          purple: '#975FE4',   // Secondary metric
          cyan: '#5DCFFF',     // Info
          pink: '#FF6B9D',     // Highlight
          yellow: '#FFC53D',   // Attention
          teal: '#3BCBB0',     // Alternative
        },
        // Semantic colors for observability
        'semantic': {
          success: '#10B981',   // Operations successful
          warning: '#F59E0B',   // Degraded performance
          error: '#EF4444',     // Failed operations
          info: '#3B82F6',      // Information
          trace: '#8B5CF6',     // Trace indicators
          span: '#EC4899',      // Span indicators
          metric: '#06B6D4',    // Metric indicators
        },
        // Graph and chart specific
        'chart': {
          grid: '#1F2937',      // Grid lines
          axis: '#374151',      // Axis lines
          tooltip: '#111827',   // Tooltip background
        }
      },
      // Modern typography for data-heavy interfaces
      fontFamily: {
        'mono': ['SF Mono', 'Monaco', 'Inconsolata', 'Fira Code', 'monospace'],
        'sans': ['-apple-system', 'BlinkMacSystemFont', 'Segoe UI', 'Roboto', 'Helvetica', 'Arial', 'sans-serif'],
        'display': ['Inter', '-apple-system', 'BlinkMacSystemFont', 'sans-serif'],
      },
      fontWeight: {
        'ultralight': '200',
        'light': '300',
        'regular': '400',
        'medium': '500',
        'semibold': '600',
        'bold': '700',
      },
      // Precise spacing
      spacing: {
        '0.25': '0.0625rem', // 1px
        '0.5': '0.125rem',   // 2px
        '18': '4.5rem',      // 72px
        '22': '5.5rem',      // 88px
      },
      // Minimal borders for dark theme
      borderWidth: {
        '0.5': '0.5px',
        '1.5': '1.5px',
      },
      // Sharp shadows for crisp depth
      boxShadow: {
        'xs': '0 1px 2px 0 rgba(0, 0, 0, 0.8)',
        'sm': '0 2px 4px 0 rgba(0, 0, 0, 0.7)',
        'md': '0 4px 6px -1px rgba(0, 0, 0, 0.6)',
        'lg': '0 10px 15px -3px rgba(0, 0, 0, 0.5)',
        'xl': '0 20px 25px -5px rgba(0, 0, 0, 0.4)',
        // Sharp card borders
        'card': 'inset 0 0 0 1px rgba(255, 255, 255, 0.08)',
        'card-hover': 'inset 0 0 0 1px rgba(255, 255, 255, 0.15)',
        'glow': '0 0 30px rgba(14, 165, 233, 0.2)',
        'glow-sm': '0 0 10px rgba(14, 165, 233, 0.3)',
      },
      // Smooth animations for data updates
      animation: {
        'pulse-slow': 'pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite',
        'fade-in': 'fadeIn 0.2s ease-in',
        'slide-up': 'slideUp 0.3s ease-out',
      },
      keyframes: {
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        slideUp: {
          '0%': { transform: 'translateY(10px)', opacity: '0' },
          '100%': { transform: 'translateY(0)', opacity: '1' },
        },
      },

    },
  },
  plugins: [],
}