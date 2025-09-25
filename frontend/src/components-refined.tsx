/**
 * Refined Components - True Linear/Vercel Quality
 *
 * Minimal, consistent, and professional components
 */

import React, { ReactNode, useEffect, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { type LucideIcon } from 'lucide-react';
import clsx from 'clsx';

// ============================================================================
// CORE COMPONENTS
// ============================================================================

interface ButtonProps {
  children?: ReactNode;
  variant?: 'primary' | 'secondary' | 'ghost';
  size?: 'sm' | 'base' | 'lg';
  icon?: LucideIcon;
  className?: string;
  onClick?: () => void;
  disabled?: boolean;
  title?: string;
}

export const Button = ({
  children,
  variant = 'secondary',
  size = 'base',
  icon: Icon,
  className,
  onClick,
  disabled,
  title
}: ButtonProps) => (
  <motion.button
    whileHover={{ scale: disabled ? 1 : 1.02, y: -1 }}
    whileTap={{ scale: disabled ? 1 : 0.98, y: 0 }}
    initial={{ scale: 1, y: 0 }}
    transition={{ type: "spring", stiffness: 400, damping: 25 }}
    onClick={onClick}
    disabled={disabled}
    title={title}
    className={clsx(
      'btn',
      `btn-${variant}`,
      size !== 'base' && `btn-${size}`,
      disabled && 'opacity-50 cursor-not-allowed',
      className
    )}
  >
    {Icon && (
      <motion.div
        initial={{ rotate: 0 }}
        whileHover={{ rotate: variant === 'primary' ? 5 : 0 }}
        transition={{ type: "spring", stiffness: 300, damping: 20 }}
      >
        <Icon size={size === 'sm' ? 14 : 16} />
      </motion.div>
    )}
    {children}
  </motion.button>
);

// ============================================================================
// INPUT COMPONENTS
// ============================================================================

interface InputProps {
  id?: string;
  type?: string;
  value?: string;
  onChange?: (e: React.ChangeEvent<HTMLInputElement>) => void;
  placeholder?: string;
  icon?: LucideIcon;
  rightElement?: ReactNode;
  className?: string;
}

export const Input = ({
  id,
  type = 'text',
  value,
  onChange,
  placeholder,
  icon: Icon,
  rightElement,
  className
}: InputProps) => (
  <div className={clsx('input', className)}>
    {Icon && <Icon size={16} />}
    <input
      id={id}
      type={type}
      value={value}
      onChange={onChange}
      placeholder={placeholder}
    />
    {rightElement}
  </div>
);

// ============================================================================
// NAVIGATION
// ============================================================================

interface NavItemProps {
  icon: LucideIcon;
  label: string;
  active?: boolean;
  shortcut?: string;
  onClick?: () => void;
}

export const NavItem = ({ icon: Icon, label, active, shortcut, onClick }: NavItemProps) => (
  <motion.button
    whileHover={{ scale: 1.02, y: -1 }}
    whileTap={{ scale: 0.98 }}
    initial={{ scale: 1, y: 0 }}
    animate={{ scale: active ? 1.02 : 1, y: active ? -1 : 0 }}
    transition={{ type: "spring", stiffness: 300, damping: 20 }}
    onClick={onClick}
    className={clsx('nav-item', active && 'active')}
  >
    <motion.div
      animate={{ rotate: active ? 360 : 0 }}
      transition={{ duration: 0.3, ease: "easeInOut" }}
    >
      <Icon size={16} />
    </motion.div>
    <span>{label}</span>
    {shortcut && (
      <motion.span
        className="shortcut"
        animate={{ scale: active ? 1.1 : 1 }}
        transition={{ type: "spring", stiffness: 200 }}
      >
        {shortcut}
      </motion.span>
    )}
  </motion.button>
);

// ============================================================================
// STATUS & INDICATORS
// ============================================================================

interface StatusIndicatorProps {
  status: 'online' | 'warning' | 'error' | 'offline';
  label?: string;
  pulse?: boolean;
}

export const StatusIndicator = ({ status, label, pulse }: StatusIndicatorProps) => (
  <div className="status-indicator">
    <div className={clsx(
      'status-dot',
      `status-${status}`,
      pulse && 'status-pulse'
    )} />
    {label && <span>{label}</span>}
  </div>
);

// ============================================================================
// DATA DISPLAY
// ============================================================================

interface MetricProps {
  label: string;
  value: string | number;
  color?: 'blue' | 'green' | 'yellow' | 'red' | 'gray';
}

export const Metric = ({ label, value, color = 'gray' }: MetricProps) => (
  <div className="metric">
    <div className="metric-label">{label}</div>
    <div className={clsx('metric-value', color && `text-${color}-500`)}>{value}</div>
  </div>
);

interface BadgeProps {
  children: ReactNode;
  variant?: 'default' | 'success' | 'warning' | 'error' | 'info';
  size?: 'sm' | 'base';
}

export const Badge = ({ children, variant = 'default', size = 'base' }: BadgeProps) => (
  <span className={clsx(
    'badge',
    variant !== 'default' && `badge-${variant}`,
    size !== 'base' && `badge-${size}`
  )}>
    {children}
  </span>
);

// ============================================================================
// DROPDOWNS
// ============================================================================

interface DropdownProps {
  isOpen: boolean;
  onClose: () => void;
  children: ReactNode;
  className?: string;
}

export const Dropdown = ({ isOpen, onClose, children, className }: DropdownProps) => {
  const dropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        onClose();
      }
    };

    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        onClose();
      }
    };

    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside);
      document.addEventListener('keydown', handleEscape);
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('keydown', handleEscape);
    };
  }, [isOpen, onClose]);

  return (
    <AnimatePresence>
      {isOpen && (
        <motion.div
          ref={dropdownRef}
          initial={{ opacity: 0, scale: 0.95, y: -10 }}
          animate={{ opacity: 1, scale: 1, y: 0 }}
          exit={{ opacity: 0, scale: 0.95, y: -10 }}
          transition={{ duration: 0.15 }}
          className={clsx('dropdown', className)}
        >
          {children}
        </motion.div>
      )}
    </AnimatePresence>
  );
};

interface DropdownItemProps {
  children: ReactNode;
  variant?: 'default' | 'danger';
  icon?: LucideIcon;
  onClick?: () => void;
}

export const DropdownItem = ({ children, variant = 'default', icon: Icon, onClick }: DropdownItemProps) => (
  <motion.button
    whileHover={{ scale: 1.01 }}
    whileTap={{ scale: 0.99 }}
    onClick={onClick}
    className={clsx('dropdown-item', variant !== 'default' && variant)}
  >
    {Icon && <Icon size={16} />}
    {children}
  </motion.button>
);

// ============================================================================
// LAYOUT COMPONENTS
// ============================================================================

interface HeaderProps {
  children: ReactNode;
}

export const Header = ({ children }: HeaderProps) => (
  <header className="header">
    {children}
  </header>
);

interface PageProps {
  children: ReactNode;
  className?: string;
}

export const Page = ({ children, className }: PageProps) => (
  <div className={clsx('page', className)}>
    {children}
  </div>
);

interface SectionProps {
  title: string;
  subtitle?: string;
  action?: ReactNode;
  children: ReactNode;
}

export const Section = ({ title, subtitle, action, children }: SectionProps) => (
  <div className="card">
    <div className="section-header">
      <div>
        <h2 className="section-title">{title}</h2>
        {subtitle && <p className="section-subtitle">{subtitle}</p>}
      </div>
      {action && <div>{action}</div>}
    </div>
    {children}
  </div>
);

// ============================================================================
// LOADING STATES
// ============================================================================

export const LoadingScreen = () => (
  <motion.div
    initial={{ opacity: 0 }}
    animate={{ opacity: 1 }}
    className="flex items-center justify-center h-screen bg-gray-950"
  >
    <div className="text-center">
      <div className="loading-spinner mb-4 mx-auto" />
      <h2 className="text-lg font-semibold text-gray-50 mb-2">Starting URPO</h2>
      <p className="text-sm text-gray-400">Initializing observability engine...</p>
    </div>
  </motion.div>
);

export const LoadingSpinner = ({ size = 20 }: { size?: number }) => (
  <div
    className="loading-spinner"
    style={{ width: size, height: size }}
  />
);