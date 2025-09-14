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
        // Professional accent colors - NEUTRAL ONLY
        'accent': {
          primary: '#111827',   // Dark gray for primary actions
          secondary: '#6B7280', // Medium gray for secondary
          warning: '#F59E0B',   // Amber for warnings only
          error: '#EF4444',     // Red for errors only
        },
        // Surface colors
        'surface': {
          50: '#FFFFFF',      // Cards, panels
          100: '#F9FAFB',     // Elevated surfaces
          200: '#F3F4F6',     // Hover states
          300: '#E5E7EB',     // Borders
          400: '#D1D5DB',     // Dividers
        },
        // Status indicators - professional neutral
        'status': {
          healthy: '#6B7280',   // Gray (not green)
          warning: '#F59E0B',   // Amber  
          error: '#EF4444',     // Red
          info: '#6B7280',      // Gray (not blue)
          unknown: '#9CA3AF',   // Light gray
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
      // Clean professional shadows
      boxShadow: {
        'sm-clean': '0 1px 2px 0 rgba(0, 0, 0, 0.05)',
        'md-clean': '0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06)',
        'lg-clean': '0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -2px rgba(0, 0, 0, 0.05)',
        'card': '0 1px 3px 0 rgba(0, 0, 0, 0.1), 0 1px 2px 0 rgba(0, 0, 0, 0.06)',
        'card-hover': '0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06)',
        'inset-clean': 'inset 0 2px 4px 0 rgba(0, 0, 0, 0.06)',
      },
      // Clean gradients - neutral only
      backgroundImage: {
        'gradient-subtle': 'linear-gradient(135deg, rgba(107, 114, 128, 0.05) 0%, rgba(156, 163, 175, 0.05) 100%)',
        'shine-subtle': 'linear-gradient(90deg, transparent 0%, rgba(255, 255, 255, 0.4) 50%, transparent 100%)',
      },
    },
  },
  plugins: [],
}