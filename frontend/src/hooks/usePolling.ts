import { useEffect, useRef, useCallback } from 'react';

/**
 * Custom hook for polling data at regular intervals
 * Handles cleanup and prevents memory leaks
 */
export function usePolling(
  callback: () => void | Promise<void>,
  interval: number,
  dependencies: React.DependencyList = [],
  enabled = true
) {
  const savedCallback = useRef(callback);
  const intervalIdRef = useRef<NodeJS.Timeout | null>(null);

  // Update callback ref when it changes
  useEffect(() => {
    savedCallback.current = callback;
  }, [callback]);

  const startPolling = useCallback(() => {
    if (intervalIdRef.current) {
      clearInterval(intervalIdRef.current);
    }

    if (enabled && interval > 0) {
      // Execute immediately
      savedCallback.current();

      // Then poll at interval
      intervalIdRef.current = setInterval(() => {
        savedCallback.current();
      }, interval);
    }
  }, [interval, enabled]);

  const stopPolling = useCallback(() => {
    if (intervalIdRef.current) {
      clearInterval(intervalIdRef.current);
      intervalIdRef.current = null;
    }
  }, []);

  useEffect(() => {
    if (enabled) {
      startPolling();
    } else {
      stopPolling();
    }

    return () => {
      stopPolling();
    };
  }, [enabled, interval, startPolling, stopPolling, ...dependencies]);

  return {
    startPolling,
    stopPolling,
    isPolling: enabled && intervalIdRef.current !== null
  };
}