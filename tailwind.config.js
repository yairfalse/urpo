/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      // Ultra-sharp dark color palette
      colors: {
        // Deep black backgrounds
        'void': {
          950: '#050505', // Deep void black
          900: '#0A0A0B', // Primary surface
          800: '#0F0F10', // Elevated surface
          700: '#141416', // Hover surface
        },
        // Sharp grays
        'steel': {
          900: '#0C0C0D',
          800: '#111112', 
          700: '#1A1A1C',
          600: '#222224',
          500: '#2A2A2C',
          400: '#3A3A3C',
          300: '#8B8B8D', // Muted text
          200: '#B8B8BA', // Secondary text
          100: '#E8E8EA', // Primary text
          50: '#FFFFFF',  // Pure white
        },
        // Electric accent colors
        'electric': {
          blue: '#00D4FF',    // Primary accent
          cyan: '#00FFFF',    // Bright highlights
          green: '#00FF88',   // Success/positive
          amber: '#FFB800',   // Warning
          red: '#FF0040',     // Error/critical
          purple: '#A855F7',  // Special elements
        },
        // Glass effects
        'glass': {
          light: 'rgba(255, 255, 255, 0.05)',
          medium: 'rgba(255, 255, 255, 0.1)',
          heavy: 'rgba(255, 255, 255, 0.15)',
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
      // Performance-optimized animations
      animation: {
        'pulse-electric': 'pulse-electric 2s cubic-bezier(0.4, 0, 0.6, 1) infinite',
        'glow-pulse': 'glow-pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite',
        'slide-up': 'slide-up 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)',
        'slide-down': 'slide-down 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94)',
        'scale-in': 'scale-in 0.15s cubic-bezier(0.25, 0.46, 0.45, 0.94)',
        'knife-shine': 'knife-shine 3s ease-in-out infinite',
      },
      // High-performance keyframes
      keyframes: {
        'pulse-electric': {
          '0%, 100%': { 
            opacity: '1',
            boxShadow: '0 0 0 0 rgba(0, 212, 255, 0.4)'
          },
          '50%': { 
            opacity: '0.8',
            boxShadow: '0 0 0 8px rgba(0, 212, 255, 0.1)'
          },
        },
        'glow-pulse': {
          '0%, 100%': { 
            boxShadow: '0 0 4px rgba(0, 212, 255, 0.3), inset 0 0 4px rgba(0, 212, 255, 0.1)'
          },
          '50%': { 
            boxShadow: '0 0 8px rgba(0, 212, 255, 0.5), inset 0 0 8px rgba(0, 212, 255, 0.2)'
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
        'knife-shine': {
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