import { useEffect, useCallback } from 'react';

interface Shortcut {
  key: string;
  ctrl?: boolean;
  shift?: boolean;
  alt?: boolean;
  meta?: boolean;
  handler: () => void;
  description?: string;
}

/**
 * Custom hook for managing keyboard shortcuts
 */
export function useKeyboardShortcuts(shortcuts: Shortcut[], enabled = true) {
  const handleKeyDown = useCallback((event: KeyboardEvent) => {
    if (!enabled) return;

    shortcuts.forEach(shortcut => {
      const keyMatch = event.key.toLowerCase() === shortcut.key.toLowerCase();
      const ctrlMatch = !shortcut.ctrl || event.ctrlKey === shortcut.ctrl;
      const shiftMatch = !shortcut.shift || event.shiftKey === shortcut.shift;
      const altMatch = !shortcut.alt || event.altKey === shortcut.alt;
      const metaMatch = !shortcut.meta || event.metaKey === shortcut.meta;

      if (keyMatch && ctrlMatch && shiftMatch && altMatch && metaMatch) {
        event.preventDefault();
        shortcut.handler();
      }
    });
  }, [shortcuts, enabled]);

  useEffect(() => {
    if (enabled) {
      window.addEventListener('keydown', handleKeyDown);
      return () => window.removeEventListener('keydown', handleKeyDown);
    }
  }, [handleKeyDown, enabled]);

  return shortcuts.map(s => ({
    key: s.key,
    modifiers: [
      s.ctrl && 'Ctrl',
      s.shift && 'Shift',
      s.alt && 'Alt',
      s.meta && 'Cmd'
    ].filter(Boolean).join('+'),
    description: s.description
  }));
}

/**
 * Hook for single keyboard shortcut
 */
export function useKeyboardShortcut(
  key: string,
  handler: () => void,
  options?: {
    ctrl?: boolean;
    shift?: boolean;
    alt?: boolean;
    meta?: boolean;
    enabled?: boolean;
  }
) {
  const shortcut: Shortcut = {
    key,
    handler,
    ...options
  };

  useKeyboardShortcuts([shortcut], options?.enabled ?? true);
}