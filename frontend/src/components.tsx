/**
 * URPO UI Components
 *
 * Minimal, professional components inspired by Linear/Vercel aesthetics.
 * Every component serves a specific purpose with zero boilerplate.
 * Built for speed, beauty, and observability excellence.
 */

import React from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { clsx } from 'clsx';
import { LucideIcon, Loader2 } from 'lucide-react';

// ============================================================================
// ANIMATION PRESETS
// ============================================================================

const animations = {
  fadeIn: {
    initial: { opacity: 0 },
    animate: { opacity: 1 },
    exit: { opacity: 0 },
    transition: { duration: 0.15 }
  },
  slideUp: {
    initial: { opacity: 0, y: 10 },
    animate: { opacity: 1, y: 0 },
    exit: { opacity: 0, y: 10 },
    transition: { duration: 0.2 }
  },
  scaleIn: {
    initial: { opacity: 0, scale: 0.95 },
    animate: { opacity: 1, scale: 1 },
    exit: { opacity: 0, scale: 0.95 },
    transition: { duration: 0.15 }
  },
  spring: {
    type: 'spring',
    stiffness: 280,
    damping: 30
  }
};

// ============================================================================
// BUTTON COMPONENTS
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
  const baseClasses = 'relative inline-flex items-center justify-center font-medium rounded-lg transition-all duration-150 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-dark-50 disabled:opacity-50 disabled:cursor-not-allowed';

  const variants = {
    primary: 'bg-gradient-to-r from-data-blue to-data-cyan text-white shadow-lg shadow-data-blue/25 hover:shadow-xl hover:shadow-data-blue/40 focus:ring-data-blue/50 active:scale-[0.98]',
    secondary: 'bg-dark-100 border border-dark-400 text-light-200 hover:bg-dark-150 hover:border-dark-300 hover:text-light-100 focus:ring-dark-300 active:scale-[0.98]',
    ghost: 'text-light-400 hover:text-light-200 hover:bg-dark-100 focus:ring-dark-300 active:scale-[0.98]',
    danger: 'bg-semantic-error text-white shadow-lg shadow-semantic-error/25 hover:shadow-xl hover:shadow-semantic-error/40 focus:ring-semantic-error/50 active:scale-[0.98]'
  };

  const sizes = {
    sm: 'px-3 py-1.5 text-sm gap-1.5',
    md: 'px-4 py-2 text-sm gap-2',
    lg: 'px-6 py-3 text-base gap-2'
  };

  return (
    <motion.button
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
        <Icon className="w-4 h-4" />
      ) : null}
      {children}
    </motion.button>
  );
};

// ============================================================================
// INPUT COMPONENTS
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
      {/* Subtle glow effect */}
      <div className="absolute inset-0 bg-gradient-to-r from-data-blue/10 to-data-cyan/10 rounded-lg blur opacity-0 group-focus-within:opacity-100 transition-opacity" />

      <div className="relative">
        {Icon && (
          <Icon className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-light-500" />
        )}
        <input
          className={clsx(
            'w-full bg-dark-100 border border-dark-400 rounded-lg text-light-100 placeholder:text-light-500',
            'focus:outline-none focus:ring-2 focus:ring-data-blue/50 focus:border-data-blue',
            'transition-all duration-150',
            Icon ? 'pl-10' : 'pl-4',
            rightElement ? 'pr-12' : 'pr-4',
            'py-2',
            className
          )}
          {...props}
        />
        {rightElement && (
          <div className="absolute right-3 top-1/2 -translate-y-1/2">
            {rightElement}
          </div>
        )}
      </div>
    </div>
  );
};

// ============================================================================
// CARD COMPONENTS
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
  const paddingClasses = {
    none: '',
    sm: 'p-4',
    md: 'p-6',
    lg: 'p-8'
  };

  return (
    <motion.div
      initial={animations.fadeIn.initial}
      animate={animations.fadeIn.animate}
      transition={animations.fadeIn.transition}
      className={clsx(
        'bg-dark-100 border border-dark-400 rounded-xl shadow-card',
        hover && 'hover:border-dark-300 hover:shadow-card-hover transition-all duration-250',
        paddingClasses[padding],
        className
      )}
    >
      {children}
    </motion.div>
  );
};

// ============================================================================
// NAVIGATION COMPONENTS
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
      whileTap={{ scale: 0.98 }}
      onClick={onClick}
      className={clsx(
        'flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all duration-150',
        'focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-dark-50',
        active
          ? 'bg-dark-200 text-light-100 shadow-sm border border-dark-300 focus:ring-data-blue/50'
          : 'text-light-400 hover:text-light-200 hover:bg-dark-150 focus:ring-dark-300'
      )}
    >
      <Icon className="w-4 h-4" />
      <span>{label}</span>
      {shortcut && (
        <kbd className="hidden lg:inline-block ml-auto px-2 py-0.5 text-xs bg-dark-300 text-light-500 rounded">
          {shortcut}
        </kbd>
      )}
    </motion.button>
  );
};

// ============================================================================
// STATUS INDICATORS
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
  const colors = {
    online: 'bg-semantic-success',
    offline: 'bg-light-500',
    warning: 'bg-semantic-warning',
    error: 'bg-semantic-error'
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
        <span className="text-xs font-medium text-light-300">{label}</span>
      )}
    </div>
  );
};

// ============================================================================
// DROPDOWN COMPONENTS
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
            'absolute right-0 top-full mt-2 w-72 bg-dark-100 border border-dark-400 rounded-xl shadow-xl z-50',
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
        'w-full text-left px-4 py-2 text-sm transition-colors duration-150 first:rounded-t-xl last:rounded-b-xl',
        variant === 'default'
          ? 'text-light-300 hover:bg-dark-150 hover:text-light-100'
          : 'text-semantic-error hover:bg-semantic-error/10'
      )}
    >
      {children}
    </button>
  );
};

// ============================================================================
// LOADING COMPONENTS
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
    md: 'w-8 h-8',
    lg: 'w-12 h-12'
  };

  return (
    <div className={clsx('flex items-center justify-center', className)}>
      <div className={clsx(
        'rounded-full border-2 border-data-blue border-t-transparent animate-spin',
        sizes[size]
      )} />
    </div>
  );
};

export const LoadingScreen: React.FC<{ title?: string; subtitle?: string }> = ({
  title = "Initializing URPO",
  subtitle = "Starting observability engine..."
}) => {
  return (
    <motion.div
      {...animations.fadeIn}
      className="flex items-center justify-center h-screen bg-dark-50"
    >
      <div className="text-center">
        <LoadingSpinner size="lg" className="mb-4" />
        <h2 className="text-light-100 font-semibold text-lg mb-2">{title}</h2>
        <p className="text-light-400 text-sm">{subtitle}</p>
      </div>
    </motion.div>
  );
};

// ============================================================================
// LAYOUT HELPERS
// ============================================================================

interface HeaderProps {
  children: React.ReactNode;
  className?: string;
}

export const Header: React.FC<HeaderProps> = ({ children, className }) => (
  <motion.header
    initial={{ opacity: 0, y: -10 }}
    animate={{ opacity: 1, y: 0 }}
    transition={{ duration: 0.3 }}
    className={clsx(
      'bg-dark-100 border-b border-dark-400 shadow-lg relative overflow-hidden',
      className
    )}
  >
    {/* Gradient accent line */}
    <div className="absolute top-0 left-0 right-0 h-0.5 bg-gradient-to-r from-transparent via-data-blue to-transparent opacity-80" />
    {children}
  </motion.header>
);

interface PageProps {
  children: React.ReactNode;
  className?: string;
}

export const Page: React.FC<PageProps> = ({ children, className }) => (
  <motion.div
    initial={animations.fadeIn.initial}
    animate={animations.fadeIn.animate}
    transition={animations.fadeIn.transition}
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
  <Card className="h-full">
    <div className="flex items-center justify-between mb-6">
      <div>
        <h2 className="text-xl font-bold text-light-50 mb-1">{title}</h2>
        {subtitle && (
          <p className="text-sm text-light-400">{subtitle}</p>
        )}
      </div>
      {action && <div className="flex items-center gap-2">{action}</div>}
    </div>
    <div className="h-[calc(100%-100px)]">
      {children}
    </div>
  </Card>
);

// ============================================================================
// BADGE COMPONENTS
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
  const variants = {
    default: 'bg-dark-300 text-light-300',
    success: 'bg-semantic-success/20 text-semantic-success',
    warning: 'bg-semantic-warning/20 text-semantic-warning',
    error: 'bg-semantic-error/20 text-semantic-error',
    info: 'bg-data-blue/20 text-data-blue'
  };

  const sizes = {
    sm: 'px-2 py-0.5 text-xs',
    md: 'px-3 py-1 text-sm'
  };

  return (
    <span className={clsx(
      'inline-flex items-center font-medium rounded-full',
      variants[variant],
      sizes[size]
    )}>
      {children}
    </span>
  );
};

// ============================================================================
// METRICS DISPLAY
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
  const colors = {
    blue: 'text-data-blue',
    green: 'text-data-green',
    yellow: 'text-data-yellow',
    red: 'text-data-red',
    cyan: 'text-data-cyan'
  };

  return (
    <div className="flex items-center gap-2">
      <span className="text-xs text-light-500">{label}</span>
      <span className={clsx('text-sm font-medium', colors[color])}>
        {typeof value === 'number' ? value.toLocaleString() : value}
      </span>
    </div>
  );
};