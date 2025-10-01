/**
 * URPO Design System - Single Source of Truth
 *
 * Philosophy: Minimal, Dark, Data-Dense, Professional
 * Everything derives from this file. No exceptions.
 */

import React, { ReactNode } from 'react';
import clsx from 'clsx';
import './core.css';

// ============================================================================
// DESIGN TOKENS - The only truth
// ============================================================================

export const COLORS = {
  // Base palette - ONLY these colors allowed
  bg: {
    primary: '#0A0A0A',    // Main background
    secondary: '#141414',  // Card/Section background
    elevated: '#1A1A1A',   // Hover/Active states
    overlay: '#222222',    // Modals/Dropdowns
  },

  border: {
    subtle: '#262626',     // Default borders
    default: '#333333',    // Emphasized borders
    strong: '#404040',     // Active/Hover borders
  },

  text: {
    primary: '#FFFFFF',    // Headers, important text
    secondary: '#A3A3A3',  // Body text
    tertiary: '#666666',   // Labels, hints
    disabled: '#4A4A4A',   // Disabled state
  },

  accent: {
    primary: '#0EA5E9',    // Primary actions (sky-500)
    success: '#10B981',    // Success states (emerald-500)
    warning: '#F59E0B',    // Warning states (amber-500)
    error: '#EF4444',      // Error states (red-500)
    info: '#8B5CF6',       // Info states (violet-500)
  },

  chart: {
    // Consistent chart colors
    series: ['#0EA5E9', '#10B981', '#F59E0B', '#EF4444', '#8B5CF6', '#EC4899', '#06B6D4', '#84CC16'],
  }
} as const;

export const SPACING = {
  xs: '4px',
  sm: '8px',
  md: '12px',
  lg: '16px',
  xl: '24px',
  '2xl': '32px',
  '3xl': '48px',
} as const;

export const TYPOGRAPHY = {
  size: {
    xs: '11px',
    sm: '12px',
    base: '13px',
    lg: '14px',
    xl: '16px',
    '2xl': '20px',
    '3xl': '24px',
  },
  weight: {
    normal: 400,
    medium: 500,
    semibold: 600,
    bold: 700,
  },
  font: {
    mono: 'SF Mono, Monaco, Inconsolata, "Courier New", monospace',
    sans: '-apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif',
  }
} as const;

export const RADIUS = {
  sm: '4px',
  md: '6px',
  lg: '8px',
  full: '9999px',
} as const;

// ============================================================================
// CORE LAYOUT COMPONENTS
// ============================================================================

interface PageProps {
  children: ReactNode;
  className?: string;
}

export const Page = ({ children, className }: PageProps) => (
  <div className={clsx('urpo-page', className)}>
    {children}
  </div>
);

interface PageHeaderProps {
  title: string;
  subtitle?: string;
  actions?: ReactNode;
  metrics?: ReactNode;
}

export const PageHeader = ({ title, subtitle, actions, metrics }: PageHeaderProps) => (
  <div className="urpo-page-header">
    <div className="urpo-page-header-main">
      <div className="urpo-page-header-text">
        <h1 className="urpo-page-title">{title}</h1>
        {subtitle && <p className="urpo-page-subtitle">{subtitle}</p>}
      </div>
      {actions && <div className="urpo-page-actions">{actions}</div>}
    </div>
    {metrics && <div className="urpo-page-metrics">{metrics}</div>}
  </div>
);

// ============================================================================
// DATA DISPLAY COMPONENTS
// ============================================================================

interface CardProps {
  children: ReactNode;
  className?: string;
  onClick?: () => void;
}

export const Card = ({ children, className, onClick }: CardProps) => (
  <div
    className={clsx('urpo-card', onClick && 'urpo-card-clickable', className)}
    onClick={onClick}
  >
    {children}
  </div>
);

interface MetricProps {
  label: string;
  value: string | number;
  trend?: 'up' | 'down' | 'neutral';
  color?: keyof typeof COLORS.accent;
}

export const Metric = ({ label, value, trend, color }: MetricProps) => (
  <div className="urpo-metric">
    <span className="urpo-metric-label">{label}</span>
    <span
      className={clsx('urpo-metric-value', {
        [`urpo-metric-${color}`]: color,
        'urpo-metric-up': trend === 'up',
        'urpo-metric-down': trend === 'down',
      })}
    >
      {value}
    </span>
  </div>
);

// ============================================================================
// TABLE COMPONENT
// ============================================================================

interface TableColumn<T> {
  key: keyof T | string;
  label: string;
  width?: string;
  align?: 'left' | 'center' | 'right';
  render?: (item: T) => ReactNode;
}

interface TableProps<T> {
  data: T[];
  columns: TableColumn<T>[];
  onRowClick?: (item: T) => void;
  className?: string;
}

export function Table<T extends Record<string, any>>({
  data,
  columns,
  onRowClick,
  className
}: TableProps<T>) {
  return (
    <div className={clsx('urpo-table-container', className)}>
      <table className="urpo-table">
        <thead>
          <tr>
            {columns.map(col => (
              <th
                key={col.key as string}
                className={clsx('urpo-table-header', `urpo-align-${col.align || 'left'}`)}
                style={{ width: col.width }}
              >
                {col.label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {data.map((item, i) => (
            <tr
              key={i}
              className={clsx('urpo-table-row', onRowClick && 'urpo-table-row-clickable')}
              onClick={() => onRowClick?.(item)}
            >
              {columns.map(col => (
                <td
                  key={col.key as string}
                  className={clsx('urpo-table-cell', `urpo-align-${col.align || 'left'}`)}
                >
                  {col.render ? col.render(item) : item[col.key as keyof T]}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

// ============================================================================
// LIST COMPONENT
// ============================================================================

interface ListItemProps {
  title: string;
  subtitle?: string;
  value?: string | number;
  status?: keyof typeof COLORS.accent;
  onClick?: () => void;
}

export const ListItem = ({ title, subtitle, value, status, onClick }: ListItemProps) => (
  <div
    className={clsx('urpo-list-item', onClick && 'urpo-list-item-clickable')}
    onClick={onClick}
  >
    <div className="urpo-list-item-content">
      <div className="urpo-list-item-title">{title}</div>
      {subtitle && <div className="urpo-list-item-subtitle">{subtitle}</div>}
    </div>
    {(value || status) && (
      <div className="urpo-list-item-meta">
        {status && <StatusDot status={status} />}
        {value && <span className="urpo-list-item-value">{value}</span>}
      </div>
    )}
  </div>
);

// ============================================================================
// FORM COMPONENTS
// ============================================================================

interface ButtonProps {
  children: ReactNode;
  variant?: 'primary' | 'secondary' | 'ghost';
  size?: 'sm' | 'md' | 'lg';
  onClick?: () => void;
  disabled?: boolean;
  className?: string;
  style?: React.CSSProperties;
}

export const Button = ({
  children,
  variant = 'secondary',
  size = 'md',
  onClick,
  disabled,
  className,
  style
}: ButtonProps) => (
  <button
    className={clsx(
      'urpo-button',
      `urpo-button-${variant}`,
      `urpo-button-${size}`,
      className
    )}
    onClick={onClick}
    disabled={disabled}
    style={style}
  >
    {children}
  </button>
);

interface InputProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  type?: string;
  className?: string;
}

export const Input = ({ value, onChange, placeholder, type = 'text', className }: InputProps) => (
  <input
    type={type}
    value={value}
    onChange={(e) => onChange(e.target.value)}
    placeholder={placeholder}
    className={clsx('urpo-input', className)}
  />
);

// ============================================================================
// STATUS COMPONENTS
// ============================================================================

interface StatusDotProps {
  status: keyof typeof COLORS.accent;
  pulse?: boolean;
}

export const StatusDot = ({ status, pulse }: StatusDotProps) => (
  <span
    className={clsx(
      'urpo-status-dot',
      `urpo-status-${status}`,
      pulse && 'urpo-status-pulse'
    )}
  />
);

interface BadgeProps {
  children: ReactNode;
  variant?: keyof typeof COLORS.accent;
}

export const Badge = ({ children, variant = 'primary' }: BadgeProps) => (
  <span className={clsx('urpo-badge', `urpo-badge-${variant}`)}>
    {children}
  </span>
);

// ============================================================================
// EMPTY STATE
// ============================================================================

interface EmptyStateProps {
  message: string;
  description?: string;
}

export const EmptyState = ({ message, description }: EmptyStateProps) => (
  <div className="urpo-empty-state">
    <div className="urpo-empty-state-icon">
      <svg width="48" height="48" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4" />
      </svg>
    </div>
    <h3 className="urpo-empty-state-title">{message}</h3>
    {description && <p className="urpo-empty-state-description">{description}</p>}
  </div>
);

// ============================================================================
// LOADING STATE
// ============================================================================

export const LoadingState = () => (
  <div className="urpo-loading-state">
    <div className="urpo-spinner" />
    <span className="urpo-loading-text">Loading...</span>
  </div>
);

// ============================================================================
// GRID SYSTEM
// ============================================================================

interface GridProps {
  children: ReactNode;
  cols?: 1 | 2 | 3 | 4;
  gap?: keyof typeof SPACING;
  className?: string;
}

export const Grid = ({ children, cols = 1, gap = 'md', className }: GridProps) => (
  <div
    className={clsx('urpo-grid', `urpo-grid-${cols}`, className)}
    style={{ gap: SPACING[gap] }}
  >
    {children}
  </div>
);

// ============================================================================
// EXPORT ALL
// ============================================================================

export default {
  // Tokens
  COLORS,
  SPACING,
  TYPOGRAPHY,
  RADIUS,

  // Components
  Page,
  PageHeader,
  Card,
  Metric,
  Table,
  ListItem,
  Button,
  Input,
  StatusDot,
  Badge,
  EmptyState,
  LoadingState,
  Grid,
};