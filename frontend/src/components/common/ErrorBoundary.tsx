import { Component, ReactNode, ErrorInfo } from 'react';
import { AlertTriangle, RefreshCw, ChevronDown, ChevronUp } from 'lucide-react';

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
  onError?: (error: Error, errorInfo: ErrorInfo) => void;
  resetKeys?: Array<string | number>;
  resetOnPropsChange?: boolean;
  isolate?: boolean;
  componentName?: string;
}

interface State {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
  errorCount: number;
  showDetails: boolean;
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = {
      hasError: false,
      error: null,
      errorInfo: null,
      errorCount: 0,
      showDetails: false
    };
  }

  static getDerivedStateFromError(error: Error): Partial<State> {
    return {
      hasError: true,
      error
    };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    const { onError, componentName } = this.props;
    
    // Log to console in development
    if (process.env.NODE_ENV === 'development') {
      console.error(`Error in ${componentName || 'Component'}:`, error);
      console.error('Component Stack:', errorInfo.componentStack);
    }

    // Call custom error handler if provided
    if (onError) {
      onError(error, errorInfo);
    }

    // Update state with error details
    this.setState(prevState => ({
      errorInfo,
      errorCount: prevState.errorCount + 1
    }));

    // Report to error tracking service in production
    if (process.env.NODE_ENV === 'production') {
      // TODO: Send to Sentry or similar service
      console.error('Production error:', { error, errorInfo, componentName });
    }
  }

  componentDidUpdate(prevProps: Props) {
    const { resetKeys, resetOnPropsChange } = this.props;
    const { hasError } = this.state;
    
    // Reset on prop changes if specified
    if (hasError && prevProps.resetKeys !== resetKeys && resetOnPropsChange) {
      this.resetErrorBoundary();
    }
  }

  resetErrorBoundary = () => {
    this.setState({
      hasError: false,
      error: null,
      errorInfo: null,
      showDetails: false
    });
  };

  toggleDetails = () => {
    this.setState(prevState => ({
      showDetails: !prevState.showDetails
    }));
  };

  render() {
    const { hasError, error, errorInfo, errorCount, showDetails } = this.state;
    const { children, fallback, isolate, componentName } = this.props;

    if (hasError && error) {
      // Use custom fallback if provided
      if (fallback) {
        return <>{fallback}</>;
      }

      // Default error UI
      return (
        <div className={`${isolate ? 'relative' : 'min-h-screen'} flex items-center justify-center p-4`}>
          <div className="clean-card p-6 max-w-2xl w-full">
            <div className="flex items-start gap-4">
              <div className="flex-shrink-0">
                <AlertTriangle className="w-8 h-8 text-status-error" />
              </div>
              
              <div className="flex-1">
                <h2 className="text-lg font-semibold text-text-900 mb-2">
                  Something went wrong
                  {componentName && ` in ${componentName}`}
                </h2>
                
                <p className="text-text-700 mb-4">
                  {error.message || 'An unexpected error occurred'}
                </p>

                {errorCount > 1 && (
                  <p className="text-sm text-status-warning mb-3">
                    This error has occurred {errorCount} times
                  </p>
                )}

                <div className="flex gap-2 mb-4">
                  <button
                    onClick={this.resetErrorBoundary}
                    className="clean-button flex items-center gap-2"
                  >
                    <RefreshCw className="w-4 h-4" />
                    Try Again
                  </button>

                  {process.env.NODE_ENV === 'development' && (
                    <button
                      onClick={this.toggleDetails}
                      className="clean-button flex items-center gap-2"
                    >
                      {showDetails ? (
                        <>
                          <ChevronUp className="w-4 h-4" />
                          Hide Details
                        </>
                      ) : (
                        <>
                          <ChevronDown className="w-4 h-4" />
                          Show Details
                        </>
                      )}
                    </button>
                  )}
                </div>

                {showDetails && errorInfo && (
                  <div className="mt-4 space-y-3">
                    <div className="p-3 bg-surface-100 rounded border border-surface-300">
                      <h3 className="text-sm font-semibold text-text-900 mb-2">Error Stack:</h3>
                      <pre className="text-xs font-mono text-text-700 whitespace-pre-wrap break-all">
                        {error.stack}
                      </pre>
                    </div>

                    <div className="p-3 bg-surface-100 rounded border border-surface-300">
                      <h3 className="text-sm font-semibold text-text-900 mb-2">Component Stack:</h3>
                      <pre className="text-xs font-mono text-text-700 whitespace-pre-wrap">
                        {errorInfo.componentStack}
                      </pre>
                    </div>
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>
      );
    }

    return children;
  }
}

// Convenience wrapper for functional components
export const withErrorBoundary = <P extends object>(
  Component: React.ComponentType<P>,
  errorBoundaryProps?: Omit<Props, 'children'>
) => {
  const WrappedComponent = (props: P) => (
    <ErrorBoundary {...errorBoundaryProps}>
      <Component {...props} />
    </ErrorBoundary>
  );
  
  WrappedComponent.displayName = `withErrorBoundary(${Component.displayName || Component.name})`;
  
  return WrappedComponent;
};