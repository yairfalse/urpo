import { useCallback } from 'react';

/**
 * Custom hook for consistent error handling across the app
 */
export function useErrorHandler(componentName?: string) {
  const logError = useCallback((error: Error, context?: string) => {
    const errorMessage = `Error in ${componentName || 'Component'}${context ? ` (${context})` : ''}: ${error.message}`;
    
    // Log to console in development
    if (process.env.NODE_ENV === 'development') {
      console.error(errorMessage);
      console.error(error.stack);
    }
    
    // In production, send to error tracking service
    if (process.env.NODE_ENV === 'production') {
      // TODO: Send to Sentry or similar
      console.error('Production error:', { error, componentName, context });
    }
  }, [componentName]);

  const handleAsyncError = useCallback(async <T,>(
    promise: Promise<T>,
    context?: string
  ): Promise<T | null> => {
    try {
      return await promise;
    } catch (error) {
      logError(error as Error, context);
      return null;
    }
  }, [logError]);

  const handleSyncError = useCallback(<T,>(
    fn: () => T,
    fallback: T,
    context?: string
  ): T => {
    try {
      return fn();
    } catch (error) {
      logError(error as Error, context);
      return fallback;
    }
  }, [logError]);

  return {
    logError,
    handleAsyncError,
    handleSyncError
  };
}