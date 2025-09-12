import { ReactNode } from 'react';
import { clsx } from 'clsx';

interface BaseCardProps {
  children: ReactNode;
  className?: string;
  hover?: boolean;
  onClick?: () => void;
  noPadding?: boolean;
  variant?: 'default' | 'bordered' | 'elevated';
}

export const BaseCard = ({ 
  children, 
  className = '', 
  hover = false,
  onClick,
  noPadding = false,
  variant = 'default'
}: BaseCardProps) => {
  const baseClasses = clsx(
    'bg-surface-50 border border-surface-300 rounded-lg',
    {
      'hover:border-surface-400 hover:shadow-md transition-all cursor-pointer': hover || onClick,
      'p-6': !noPadding,
      'shadow-sm': variant === 'default',
      'shadow-md': variant === 'elevated',
    },
    className
  );

  return (
    <div className={baseClasses} onClick={onClick}>
      {children}
    </div>
  );
};