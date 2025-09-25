/**
 * URPO Design System - Professional UI Components
 *
 * Linear/Vercel-quality components with:
 * - Perfect 4px spacing grid
 * - Minimal, refined color palette
 * - Subtle micro-interactions
 * - Typography perfection
 * - Premium, expensive feel
 */

import React from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { clsx } from 'clsx';
import { LucideIcon, Loader2 } from 'lucide-react';

// ============================================================================
// REFINED ANIMATION SYSTEM
// ============================================================================

const animations = {
  // Ultra-smooth micro-interactions (150ms - perfect for premium feel)
  subtle: {
    initial: { opacity: 0 },
    animate: { opacity: 1 },
    exit: { opacity: 0 },
    transition: { duration: 0.15, ease: [0.23, 1, 0.32, 1] }
  },
  // Elegant slide-up with perfect easing
  slideUp: {
    initial: { opacity: 0, y: 4 },
    animate: { opacity: 1, y: 0 },
    exit: { opacity: 0, y: 4 },
    transition: { duration: 0.15, ease: [0.23, 1, 0.32, 1] }
  },
  // Scale with perfect spring feel
  scaleIn: {
    initial: { opacity: 0, scale: 0.96 },
    animate: { opacity: 1, scale: 1 },
    exit: { opacity: 0, scale: 0.96 },
    transition: { duration: 0.15, ease: [0.23, 1, 0.32, 1] }
  },
  // Perfect spring for interactive elements
  spring: {
    type: 'spring',
    stiffness: 400,
    damping: 30,
    mass: 0.8
  }
};

// ============================================================================
// BUTTON COMPONENTS - Linear/Vercel Quality
// ============================================================================

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
  size?: 'sm' | 'md' | 'lg';
  loading?: boolean;
  icon?: LucideIcon;
  children?: React.ReactNode;
}

export const Button: React.FC<ButtonProps> = ({
  variant = 'secondary',
  size = 'md',
  loading = false,
  icon: Icon,
  className,
  children,
  disabled,
  ...props
}) => {
  // Perfect base classes - follows Linear's approach
  const baseClasses = `
    relative inline-flex items-center justify-center
    font-medium rounded-md
    transition-all duration-150 ease-out
    focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:ring-offset-0
    disabled:opacity-40 disabled:cursor-not-allowed disabled:pointer-events-none
    select-none
  `;

  // Refined variants - minimal, elegant, expensive feel
  const variants = {
    primary: `
      bg-white text-gray-900 shadow-sm
      hover:bg-gray-50 hover:shadow-md
      active:bg-gray-100 active:shadow-sm active:scale-[0.98]
      border-0
    `,
    secondary: `
      bg-gray-900 text-gray-100 border border-gray-800
      hover:bg-gray-800 hover:border-gray-700 hover:text-white
      active:bg-gray-750 active:scale-[0.98]
    `,
    ghost: `
      text-gray-400 bg-transparent border-0
      hover:text-gray-300 hover:bg-gray-900/50
      active:bg-gray-900/70 active:scale-[0.98]
    `,
    danger: `
      bg-red-600 text-white border-0 shadow-sm
      hover:bg-red-500 hover:shadow-md
      active:bg-red-700 active:scale-[0.98]
    `
  };

  // Perfect spacing on 4px grid
  const sizes = {
    sm: 'h-8 px-3 text-sm gap-2',
    md: 'h-9 px-4 text-sm gap-2',
    lg: 'h-10 px-6 text-base gap-3'
  };

  return (
    <motion.button
      whileHover={{ scale: 1.01 }}
      whileTap={{ scale: 0.98 }}
      transition={animations.spring}
      className={clsx(
        baseClasses,
        variants[variant],
        sizes[size],
        className
      )}
      disabled={disabled || loading}
      {...(props as any)}
    >
      {loading ? (
        <Loader2 className="w-4 h-4 animate-spin" />
      ) : Icon ? (
        <Icon className="w-4 h-4 shrink-0" />
      ) : null}
      {children && (
        <span className="truncate">{children}</span>
      )}
    </motion.button>
  );
};

// ============================================================================
// INPUT COMPONENTS - Perfect Typography & Spacing
// ============================================================================

interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  icon?: LucideIcon;
  rightElement?: React.ReactNode;
}

export const Input: React.FC<InputProps> = ({
  icon: Icon,
  rightElement,
  className,
  ...props
}) => {
  return (
    <div className="relative group">
      <div className="relative">
        {Icon && (
          <Icon className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-500 pointer-events-none" />
        )}
        <input
          className={clsx(
            // Base styling - clean & minimal
            'w-full h-9 bg-gray-950 border border-gray-800 rounded-md',
            'text-gray-100 text-sm placeholder:text-gray-500',
            // Focus states - subtle blue accent
            'focus:outline-none focus:ring-1 focus:ring-blue-500/30 focus:border-blue-500/50',
            // Hover state
            'hover:border-gray-700',
            // Transitions
            'transition-all duration-150 ease-out',
            // Spacing
            Icon ? 'pl-10' : 'pl-3',
            rightElement ? 'pr-12' : 'pr-3',
            className
          )}
          {...props}
        />
        {rightElement && (
          <div className="absolute right-3 top-1/2 -translate-y-1/2 flex items-center">
            {rightElement}
          </div>
        )}
      </div>
    </div>
  );
};

// ============================================================================
// CARD COMPONENTS - Linear-Style Clean Cards
// ============================================================================

interface CardProps {
  children: React.ReactNode;
  className?: string;
  hover?: boolean;
  padding?: 'none' | 'sm' | 'md' | 'lg';
}

export const Card: React.FC<CardProps> = ({
  children,
  className,
  hover = false,
  padding = 'md'
}) => {
  // Perfect 4px grid spacing
  const paddingClasses = {
    none: '',
    sm: 'p-4',    // 16px
    md: 'p-6',    // 24px
    lg: 'p-8'     // 32px
  };

  return (
    <motion.div
      initial={animations.subtle.initial}
      animate={animations.subtle.animate}
      transition={animations.subtle.transition}
      className={clsx(
        // Base card styling - clean & minimal
        'bg-gray-950 border border-gray-900 rounded-lg',
        // Subtle shadow for depth
        'shadow-sm',
        // Hover effects (if enabled)
        hover && 'hover:border-gray-800 hover:shadow-md transition-all duration-150',
        paddingClasses[padding],
        className
      )}
    >
      {children}
    </motion.div>
  );
};

// ============================================================================
// NAVIGATION COMPONENTS - Linear-Style Nav
// ============================================================================

interface NavItemProps {
  icon: LucideIcon;
  label: string;
  active?: boolean;
  shortcut?: string;
  onClick?: () => void;
}

export const NavItem: React.FC<NavItemProps> = ({
  icon: Icon,
  label,
  active = false,
  shortcut,
  onClick
}) => {
  return (
    <motion.button
      whileHover={{ scale: 1.01 }}
      whileTap={{ scale: 0.98 }}
      onClick={onClick}
      className={clsx(
        // Base styling
        'flex items-center gap-3 px-3 py-2 rounded-md text-sm font-medium',
        'transition-all duration-150 ease-out',
        'focus:outline-none focus:ring-1 focus:ring-blue-500/20',
        // Active state - clean white accent
        active
          ? 'bg-gray-900 text-white border border-gray-800 shadow-sm'
          : 'text-gray-400 hover:text-gray-300 hover:bg-gray-900/50'
      )}
    >
      <Icon className="w-4 h-4 shrink-0" />
      <span className="truncate">{label}</span>
      {shortcut && (
        <kbd className="ml-auto px-2 py-1 text-xs bg-gray-800 text-gray-400 rounded border border-gray-700 font-mono hidden sm:block">
          {shortcut}
        </kbd>
      )}
    </motion.button>
  );
};

// ============================================================================
// STATUS INDICATORS - Refined & Minimal
// ============================================================================

interface StatusIndicatorProps {
  status: 'online' | 'offline' | 'warning' | 'error';
  label?: string;
  pulse?: boolean;
}

export const StatusIndicator: React.FC<StatusIndicatorProps> = ({
  status,
  label,
  pulse = false
}) => {
  // Refined, professional colors
  const colors = {
    online: 'bg-green-500',
    offline: 'bg-gray-500',
    warning: 'bg-yellow-500',
    error: 'bg-red-500'
  };

  return (
    <div className="flex items-center gap-2">
      <div
        className={clsx(
          'w-2 h-2 rounded-full',
          colors[status],
          pulse && 'animate-pulse'
        )}
      />
      {label && (
        <span className="text-xs font-medium text-gray-400 select-none">{label}</span>
      )}
    </div>
  );
};

// ============================================================================
// DROPDOWN COMPONENTS - Vercel-Style Popovers
// ============================================================================

interface DropdownProps {
  isOpen: boolean;
  onClose: () => void;
  children: React.ReactNode;
  className?: string;
}

export const Dropdown: React.FC<DropdownProps> = ({
  isOpen,
  onClose,
  children,
  className
}) => {
  React.useEffect(() => {
    if (isOpen) {
      const handleEscape = (e: KeyboardEvent) => {
        if (e.key === 'Escape') onClose();
      };
      const handleClickOutside = (e: MouseEvent) => {
        const target = e.target as Element;
        if (!target.closest('[data-dropdown]')) onClose();
      };

      document.addEventListener('keydown', handleEscape);
      document.addEventListener('mousedown', handleClickOutside);
      return () => {
        document.removeEventListener('keydown', handleEscape);
        document.removeEventListener('mousedown', handleClickOutside);
      };
    }
  }, [isOpen, onClose]);

  return (
    <AnimatePresence>
      {isOpen && (
        <motion.div
          {...animations.scaleIn}
          data-dropdown
          className={clsx(
            // Refined dropdown styling
            'absolute right-0 top-full mt-2 w-64',
            'bg-gray-950 border border-gray-800 rounded-lg shadow-lg',
            'z-50 overflow-hidden',
            className
          )}
        >
          {children}
        </motion.div>
      )}
    </AnimatePresence>
  );
};

interface DropdownItemProps {
  children: React.ReactNode;
  onClick?: () => void;
  variant?: 'default' | 'danger';
}

export const DropdownItem: React.FC<DropdownItemProps> = ({
  children,
  onClick,
  variant = 'default'
}) => {
  return (
    <button
      onClick={onClick}
      className={clsx(
        'w-full text-left px-3 py-2 text-sm transition-colors duration-150',
        'flex items-center gap-2',
        variant === 'default'
          ? 'text-gray-300 hover:bg-gray-900 hover:text-white'
          : 'text-red-400 hover:bg-red-500/10 hover:text-red-300'
      )}
    >
      {children}
    </button>
  );
};

// ============================================================================
// LOADING COMPONENTS - Elegant Spinners
// ============================================================================

interface LoadingSpinnerProps {
  size?: 'sm' | 'md' | 'lg';
  className?: string;
}

export const LoadingSpinner: React.FC<LoadingSpinnerProps> = ({
  size = 'md',
  className
}) => {
  const sizes = {
    sm: 'w-4 h-4',
    md: 'w-6 h-6',
    lg: 'w-8 h-8'
  };

  return (
    <div className={clsx('flex items-center justify-center', className)}>
      <div className={clsx(
        'rounded-full border-2 border-gray-800 border-t-white animate-spin',
        sizes[size]
      )} />
    </div>
  );
};

export const LoadingScreen: React.FC<{ title?: string; subtitle?: string }> = ({
  title = "Starting URPO",
  subtitle = "Initializing trace explorer..."
}) => {
  return (
    <motion.div
      {...animations.subtle}
      className="flex items-center justify-center h-screen bg-black"
    >
      <div className="text-center space-y-4">
        <LoadingSpinner size="lg" />
        <div className="space-y-2">
          <h2 className="text-white font-medium text-lg">{title}</h2>
          <p className="text-gray-400 text-sm">{subtitle}</p>
        </div>
      </div>
    </motion.div>
  );
};

// ============================================================================
// LAYOUT HELPERS - Linear-Style Clean Layouts
// ============================================================================

interface HeaderProps {
  children: React.ReactNode;
  className?: string;
}

export const Header: React.FC<HeaderProps> = ({ children, className }) => (
  <motion.header
    initial={{ opacity: 0, y: -4 }}
    animate={{ opacity: 1, y: 0 }}
    transition={{ duration: 0.2, ease: [0.23, 1, 0.32, 1] }}
    className={clsx(
      'bg-black border-b border-gray-900 relative',
      className
    )}
  >
    {children}
  </motion.header>
);

interface PageProps {
  children: React.ReactNode;
  className?: string;
}

export const Page: React.FC<PageProps> = ({ children, className }) => (
  <motion.div
    initial={animations.subtle.initial}
    animate={animations.subtle.animate}
    transition={animations.subtle.transition}
    className={clsx('h-full p-6', className)}
  >
    {children}
  </motion.div>
);

interface SectionProps {
  title: string;
  subtitle?: string;
  action?: React.ReactNode;
  children: React.ReactNode;
}

export const Section: React.FC<SectionProps> = ({
  title,
  subtitle,
  action,
  children
}) => (
  <Card className="h-full" padding="lg">
    {/* Perfect header spacing */}
    <div className="flex items-start justify-between mb-6">
      <div className="space-y-1">
        <h2 className="text-lg font-semibold text-white">{title}</h2>
        {subtitle && (
          <p className="text-sm text-gray-400">{subtitle}</p>
        )}
      </div>
      {action && (
        <div className="flex items-center gap-2">
          {action}
        </div>
      )}
    </div>
    {/* Content with proper spacing */}
    <div className="h-[calc(100%-84px)]">
      {children}
    </div>
  </Card>
);

// ============================================================================
// BADGE COMPONENTS - Clean & Minimal
// ============================================================================

interface BadgeProps {
  children: React.ReactNode;
  variant?: 'default' | 'success' | 'warning' | 'error' | 'info';
  size?: 'sm' | 'md';
}

export const Badge: React.FC<BadgeProps> = ({
  children,
  variant = 'default',
  size = 'sm'
}) => {
  // Refined badge colors - subtle & professional
  const variants = {
    default: 'bg-gray-800 text-gray-300 border border-gray-700',
    success: 'bg-green-500/10 text-green-400 border border-green-500/20',
    warning: 'bg-yellow-500/10 text-yellow-400 border border-yellow-500/20',
    error: 'bg-red-500/10 text-red-400 border border-red-500/20',
    info: 'bg-blue-500/10 text-blue-400 border border-blue-500/20'
  };

  const sizes = {
    sm: 'px-2 py-1 text-xs',
    md: 'px-3 py-1 text-sm'
  };

  return (
    <span className={clsx(
      'inline-flex items-center font-medium rounded-md select-none',
      variants[variant],
      sizes[size]
    )}>
      {children}
    </span>
  );
};

// ============================================================================
// METRICS DISPLAY - Professional Data Presentation
// ============================================================================

interface MetricProps {
  label: string;
  value: string | number;
  trend?: 'up' | 'down' | 'neutral';
  color?: 'blue' | 'green' | 'yellow' | 'red' | 'cyan';
}

export const Metric: React.FC<MetricProps> = ({
  label,
  value,
  color = 'blue'
}) => {
  // Refined metric colors - subtle but clear
  const colors = {
    blue: 'text-blue-400',
    green: 'text-green-400',
    yellow: 'text-yellow-400',
    red: 'text-red-400',
    cyan: 'text-cyan-400'
  };

  return (
    <div className="flex items-center gap-2">
      <span className="text-xs text-gray-500 font-medium">{label}</span>
      <span className={clsx('text-sm font-semibold tabular-nums', colors[color])}>
        {typeof value === 'number' ? value.toLocaleString() : value}
      </span>
    </div>
  );
};