/**
 * Unified Layout Components
 * Ensures consistent look and feel across all pages
 */

import React, { ReactNode } from 'react';
import { motion } from 'framer-motion';
import {
  RefreshCw,
  Filter,
  Download,
  Settings,
  Search,
  type LucideIcon
} from 'lucide-react';
import { Button, Input, Badge } from '../components-refined';
import clsx from 'clsx';

// ============================================================================
// UNIFIED PAGE CONTAINER
// ============================================================================

interface UnifiedPageProps {
  title: string;
  subtitle?: string;
  icon?: LucideIcon;
  actions?: ReactNode;
  filters?: ReactNode;
  children: ReactNode;
  className?: string;
  onRefresh?: () => void;
  isLoading?: boolean;
}

export const UnifiedPage = ({
  title,
  subtitle,
  icon: Icon,
  actions,
  filters,
  children,
  className,
  onRefresh,
  isLoading
}: UnifiedPageProps) => (
  <motion.div
    initial={{ opacity: 0 }}
    animate={{ opacity: 1 }}
    className={clsx('unified-page', className)}
  >
    {/* Page Header */}
    <div className="page-header">
      <div className="page-header-content">
        <div className="page-header-title">
          {Icon && (
            <div className="page-icon">
              <Icon size={20} />
            </div>
          )}
          <div>
            <h1 className="page-title">{title}</h1>
            {subtitle && <p className="page-subtitle">{subtitle}</p>}
          </div>
        </div>

        <div className="page-header-actions">
          {onRefresh && (
            <Button
              variant="ghost"
              size="sm"
              icon={RefreshCw}
              onClick={onRefresh}
              disabled={isLoading}
              className={isLoading ? 'animate-spin' : ''}
            />
          )}
          {actions}
        </div>
      </div>

      {filters && (
        <div className="page-filters">
          {filters}
        </div>
      )}
    </div>

    {/* Page Content */}
    <div className="page-content">
      {children}
    </div>
  </motion.div>
);

// ============================================================================
// UNIFIED DATA TABLE
// ============================================================================

interface Column<T> {
  key: string;
  label: string;
  width?: string;
  render?: (item: T) => ReactNode;
  align?: 'left' | 'center' | 'right';
}

interface UnifiedTableProps<T> {
  data: T[];
  columns: Column<T>[];
  onRowClick?: (item: T) => void;
  emptyMessage?: string;
  isLoading?: boolean;
}

export function UnifiedTable<T extends { id?: string | number }>({
  data,
  columns,
  onRowClick,
  emptyMessage = 'No data available',
  isLoading
}: UnifiedTableProps<T>) {
  if (isLoading) {
    return <UnifiedLoadingState />;
  }

  if (!data || data.length === 0) {
    return <UnifiedEmptyState message={emptyMessage} />;
  }

  return (
    <div className="unified-table">
      <table>
        <thead>
          <tr>
            {columns.map(column => (
              <th
                key={column.key}
                style={{ width: column.width }}
                className={clsx(
                  'table-header',
                  column.align && `text-${column.align}`
                )}
              >
                {column.label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {data.map((item, index) => (
            <motion.tr
              key={item.id || index}
              initial={{ opacity: 0, x: -20 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ delay: index * 0.02, duration: 0.2 }}
              onClick={() => onRowClick?.(item)}
              className={onRowClick ? 'cursor-pointer hover:bg-gray-800/50' : ''}
            >
              {columns.map(column => (
                <td
                  key={column.key}
                  className={clsx(
                    'table-cell',
                    column.align && `text-${column.align}`
                  )}
                >
                  {column.render
                    ? column.render(item)
                    : (item as any)[column.key]}
                </td>
              ))}
            </motion.tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

// ============================================================================
// UNIFIED CARD GRID
// ============================================================================

interface UnifiedCardProps {
  title: string;
  subtitle?: string;
  icon?: LucideIcon;
  value?: string | number;
  trend?: 'up' | 'down' | 'neutral';
  onClick?: () => void;
  children?: ReactNode;
  className?: string;
}

export const UnifiedCard = ({
  title,
  subtitle,
  icon: Icon,
  value,
  trend,
  onClick,
  children,
  className
}: UnifiedCardProps) => (
  <motion.div
    whileHover={{ scale: onClick ? 1.02 : 1, y: onClick ? -2 : 0 }}
    whileTap={{ scale: onClick ? 0.98 : 1 }}
    onClick={onClick}
    className={clsx(
      'unified-card',
      onClick && 'cursor-pointer',
      className
    )}
  >
    <div className="card-header">
      {Icon && (
        <div className="card-icon">
          <Icon size={16} />
        </div>
      )}
      <div className="card-title-group">
        <h3 className="card-title">{title}</h3>
        {subtitle && <p className="card-subtitle">{subtitle}</p>}
      </div>
    </div>

    {value !== undefined && (
      <div className="card-value">
        <span className={clsx(
          'value',
          trend === 'up' && 'text-green-500',
          trend === 'down' && 'text-red-500'
        )}>
          {value}
        </span>
      </div>
    )}

    {children && (
      <div className="card-content">
        {children}
      </div>
    )}
  </motion.div>
);

// ============================================================================
// UNIFIED METRIC ROW
// ============================================================================

interface MetricItem {
  label: string;
  value: string | number;
  color?: 'blue' | 'green' | 'yellow' | 'red' | 'gray';
  icon?: LucideIcon;
}

interface UnifiedMetricsProps {
  metrics: MetricItem[];
  className?: string;
}

export const UnifiedMetrics = ({ metrics, className }: UnifiedMetricsProps) => (
  <div className={clsx('unified-metrics', className)}>
    {metrics.map((metric, index) => (
      <motion.div
        key={metric.label}
        initial={{ opacity: 0, y: 10 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: index * 0.05 }}
        className="metric-item"
      >
        {metric.icon && (
          <metric.icon size={14} className={`text-${metric.color}-500`} />
        )}
        <div className="metric-content">
          <span className="metric-label">{metric.label}</span>
          <span className={clsx(
            'metric-value',
            metric.color && `text-${metric.color}-500`
          )}>
            {metric.value}
          </span>
        </div>
      </motion.div>
    ))}
  </div>
);

// ============================================================================
// UNIFIED EMPTY STATE
// ============================================================================

interface UnifiedEmptyStateProps {
  message?: string;
  description?: string;
  icon?: LucideIcon;
  action?: ReactNode;
}

export const UnifiedEmptyState = ({
  message = 'No data found',
  description,
  icon: Icon = Search,
  action
}: UnifiedEmptyStateProps) => (
  <motion.div
    initial={{ opacity: 0, scale: 0.95 }}
    animate={{ opacity: 1, scale: 1 }}
    className="unified-empty"
  >
    <Icon size={48} className="empty-icon" />
    <h3 className="empty-message">{message}</h3>
    {description && <p className="empty-description">{description}</p>}
    {action && <div className="empty-action">{action}</div>}
  </motion.div>
);

// ============================================================================
// UNIFIED LOADING STATE
// ============================================================================

export const UnifiedLoadingState = () => (
  <motion.div
    initial={{ opacity: 0 }}
    animate={{ opacity: 1 }}
    className="unified-loading"
  >
    <div className="loading-spinner" />
    <p className="loading-text">Loading...</p>
  </motion.div>
);

// ============================================================================
// UNIFIED LIST
// ============================================================================

interface UnifiedListItem {
  id: string | number;
  title: string;
  subtitle?: string;
  badge?: string;
  icon?: LucideIcon;
  status?: 'success' | 'warning' | 'error' | 'info';
}

interface UnifiedListProps {
  items: UnifiedListItem[];
  onItemClick?: (item: UnifiedListItem) => void;
  emptyMessage?: string;
  isLoading?: boolean;
}

export const UnifiedList = ({
  items,
  onItemClick,
  emptyMessage,
  isLoading
}: UnifiedListProps) => {
  if (isLoading) {
    return <UnifiedLoadingState />;
  }

  if (!items || items.length === 0) {
    return <UnifiedEmptyState message={emptyMessage} />;
  }

  return (
    <div className="unified-list">
      {items.map((item, index) => (
        <motion.div
          key={item.id}
          initial={{ opacity: 0, x: -20 }}
          animate={{ opacity: 1, x: 0 }}
          transition={{ delay: index * 0.02 }}
          onClick={() => onItemClick?.(item)}
          className={clsx(
            'list-item',
            onItemClick && 'cursor-pointer'
          )}
        >
          {item.icon && (
            <div className="list-icon">
              <item.icon size={16} />
            </div>
          )}
          <div className="list-content">
            <div className="list-title">{item.title}</div>
            {item.subtitle && (
              <div className="list-subtitle">{item.subtitle}</div>
            )}
          </div>
          {item.badge && (
            <Badge variant={item.status}>
              {item.badge}
            </Badge>
          )}
        </motion.div>
      ))}
    </div>
  );
};