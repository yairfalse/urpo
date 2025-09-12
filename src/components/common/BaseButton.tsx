import { ReactNode } from 'react';
import { clsx } from 'clsx';

interface BaseButtonProps {
  children: ReactNode;
  onClick?: () => void;
  variant?: 'primary' | 'secondary' | 'ghost';
  size?: 'sm' | 'md' | 'lg';
  active?: boolean;
  disabled?: boolean;
  className?: string;
  icon?: ReactNode;
}

export const BaseButton = ({
  children,
  onClick,
  variant = 'secondary',
  size = 'md',
  active = false,
  disabled = false,
  className = '',
  icon
}: BaseButtonProps) => {
  const baseClasses = clsx(
    'inline-flex items-center justify-center font-medium transition-all rounded-lg',
    {
      // Size variants
      'px-2 py-1 text-xs gap-1': size === 'sm',
      'px-3 py-2 text-sm gap-2': size === 'md',
      'px-4 py-3 text-base gap-2': size === 'lg',
      
      // Color variants
      'bg-text-900 text-surface-50 hover:bg-text-700': variant === 'primary' && !active,
      'bg-surface-50 border border-surface-300 text-text-700 hover:bg-surface-100 hover:border-surface-400': variant === 'secondary' && !active,
      'text-text-700 hover:bg-surface-100': variant === 'ghost' && !active,
      
      // Active state
      'bg-text-900 text-surface-50': active,
      
      // Disabled state
      'opacity-50 cursor-not-allowed': disabled,
      'cursor-pointer': !disabled
    },
    className
  );

  return (
    <button
      className={baseClasses}
      onClick={onClick}
      disabled={disabled}
    >
      {icon && <span className="w-4 h-4">{icon}</span>}
      {children}
    </button>
  );
};