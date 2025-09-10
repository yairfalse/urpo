/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      // Hubble-inspired clean professional palette
      colors: {
        // Clean backgrounds - white and light grays
        'background': {
          50: '#FFFFFF',     // Pure white background
          100: '#F8F9FA',    // Subtle gray background
          150: '#F1F3F4',    // Light elevated surfaces
          200: '#E8EAED',    // Border/divider color
        },
        // Text colors - high contrast, readable
        'text': {
          900: '#1F2937',    // Primary text (dark gray)
          700: '#374151',    // Secondary text
          500: '#6B7280',    // Muted text
          300: '#9CA3AF',    // Placeholder text
          100: '#F3F4F6',    // Light text (on dark backgrounds)
        },
        // Professional accent colors
        'accent': {
          blue: '#3B82F6',    // Primary blue (links, actions)
          cyan: '#06B6D4',    // Info/secondary
          green: '#10B981',   // Success states
          amber: '#F59E0B',   // Warning states
          red: '#EF4444',     // Error states
          purple: '#8B5CF6',  // Special elements
        },
        // Surface colors
        'surface': {
          50: '#FFFFFF',      // Cards, panels
          100: '#F9FAFB',     // Elevated surfaces
          200: '#F3F4F6',     // Hover states
          300: '#E5E7EB',     // Borders
          400: '#D1D5DB',     // Dividers
        },
        // Status indicators - clean, no glow
        'status': {
          healthy: '#10B981',   // Green
          warning: '#F59E0B',   // Amber
          error: '#EF4444',     // Red
          info: '#3B82F6',      // Blue
          unknown: '#6B7280',   // Gray
        }
      },
      // Sharp typography system
      fontFamily: {
        'mono': ['JetBrains Mono', 'Fira Code', 'Consolas', 'monospace'],
        'sans': ['Inter Tight', 'Inter', 'SF Pro Display', 'system-ui', 'sans-serif'],
        'display': ['Inter Tight', 'Inter', 'system-ui', 'sans-serif'],
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
      // Razor-sharp borders
      borderWidth: {
        '0.5': '0.5px',
        '1.5': '1.5px',
      },
      // Glass blur effects
      backdropBlur: {
        'xs': '2px',
        'knife': '8px',
      },
      // Subtle professional animations
      animation: {
        'pulse-subtle': 'pulse-subtle 2s cubic-bezier(0.4, 0, 0.6, 1) infinite',
        'fade-in': 'fade-in 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)',
        'slide-up': 'slide-up 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)',
        'slide-down': 'slide-down 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)',
        'scale-in': 'scale-in 0.15s cubic-bezier(0.25, 0.46, 0.45, 0.94)',
        'shine-subtle': 'shine-subtle 3s ease-in-out infinite',
      },
      // Subtle professional keyframes
      keyframes: {
        'pulse-subtle': {
          '0%, 100%': { 
            opacity: '1'
          },
          '50%': { 
            opacity: '0.7'
          },
        },
        'fade-in': {
          'from': { 
            opacity: '0'
          },
          'to': { 
            opacity: '1'
          },
        },
        'slide-up': {
          'from': { 
            opacity: '0', 
            transform: 'translateY(8px)' 
          },
          'to': { 
            opacity: '1', 
            transform: 'translateY(0)' 
          },
        },
        'slide-down': {
          'from': { 
            opacity: '0', 
            transform: 'translateY(-8px)' 
          },
          'to': { 
            opacity: '1', 
            transform: 'translateY(0)' 
          },
        },
        'scale-in': {
          'from': { 
            opacity: '0', 
            transform: 'scale(0.95)' 
          },
          'to': { 
            opacity: '1', 
            transform: 'scale(1)' 
          },
        },
        'shine-subtle': {
          '0%': { 
            backgroundPosition: '-200% 0' 
          },
          '100%': { 
            backgroundPosition: '200% 0' 
          },
        },
      },
      // Sharp box shadows
      boxShadow: {
        'knife': '0 0 0 0.5px rgba(255, 255, 255, 0.1), 0 2px 8px rgba(0, 0, 0, 0.8)',
        'knife-hover': '0 0 0 0.5px rgba(0, 212, 255, 0.3), 0 4px 16px rgba(0, 0, 0, 0.9)',
        'electric': '0 0 8px rgba(0, 212, 255, 0.4), inset 0 0 8px rgba(0, 212, 255, 0.1)',
        'glass': 'inset 0 1px 0 rgba(255, 255, 255, 0.1), 0 2px 4px rgba(0, 0, 0, 0.5)',
        'void': '0 8px 32px rgba(0, 0, 0, 0.8)',
      },
      // Glass gradients
      backgroundImage: {
        'glass-gradient': 'linear-gradient(135deg, rgba(255, 255, 255, 0.1) 0%, rgba(255, 255, 255, 0.05) 100%)',
        'electric-gradient': 'linear-gradient(135deg, rgba(0, 212, 255, 0.1) 0%, rgba(0, 255, 136, 0.1) 100%)',
        'knife-shine': 'linear-gradient(90deg, transparent 0%, rgba(255, 255, 255, 0.1) 50%, transparent 100%)',
      },
    },
  },
  plugins: [],
}