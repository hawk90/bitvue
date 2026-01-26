/**
 * Error Boundary Component
 *
 * Catches JavaScript errors in component tree, displays fallback UI,
 * and logs error information. Use this to wrap critical sections of the app.
 *
 * Based on React error boundary pattern.
 */

import { Component, ReactNode } from 'react';
import { logger } from '../utils/logger';

interface ErrorBoundaryProps {
  children: ReactNode;
  /** Custom fallback component to render on error */
  fallback?: React.ComponentType<ErrorFallbackProps>;
  /** Called when error is caught */
  onError?: (error: Error, errorInfo: React.ErrorInfo) => void;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
  errorInfo: React.ErrorInfo | null;
}

export interface ErrorFallbackProps {
  error: Error | null;
  errorInfo: React.ErrorInfo | null;
  resetError: () => void;
}

/**
 * Default error fallback UI
 */
function DefaultErrorFallback({ error, errorInfo, resetError }: ErrorFallbackProps) {
  const handleReload = () => {
    window.location.reload();
  };

  return (
    <div className="error-boundary-fallback">
      <div className="error-boundary-content">
        <div className="error-boundary-icon">
          <i className="codicon codicon-error" />
        </div>
        <h2 className="error-boundary-title">Something went wrong</h2>
        <p className="error-boundary-message">
          An unexpected error occurred. You can try reloading or restarting the application.
        </p>

        {error && (
          <details className="error-boundary-details">
            <summary>Error Details</summary>
            <div className="error-boundary-details-content">
              <div className="error-boundary-error-name">{error.name}: {error.message}</div>
              {errorInfo && (
                <pre className="error-boundary-stack">
                  {errorInfo.componentStack}
                </pre>
              )}
              {error.stack && (
                <pre className="error-boundary-stack">
                  {error.stack}
                </pre>
              )}
            </div>
          </details>
        )}

        <div className="error-boundary-actions">
          <button
            className="error-boundary-button error-boundary-button-primary"
            onClick={resetError}
          >
            Try Again
          </button>
          <button
            className="error-boundary-button error-boundary-button-secondary"
            onClick={handleReload}
          >
            Reload Page
          </button>
        </div>
      </div>
    </div>
  );
}

/**
 * Error Boundary Class Component
 *
 * Usage:
 * ```tsx
 * <ErrorBoundary>
 *   <YourComponent />
 * </ErrorBoundary>
 * ```
 *
 * With custom fallback:
 * ```tsx
 * <ErrorBoundary fallback={MyErrorFallback}>
 *   <YourComponent />
 * </ErrorBoundary>
 * ```
 */
export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = {
      hasError: false,
      error: null,
      errorInfo: null,
    };
  }

  static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryState> {
    return {
      hasError: true,
      error,
    };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    // Log error to console
    logger.error('ErrorBoundary caught an error:', error, errorInfo);

    // Update state with error info
    this.setState({
      errorInfo,
    });

    // Call custom error handler if provided
    this.props.onError?.(error, errorInfo);
  }

  resetError = () => {
    this.setState({
      hasError: false,
      error: null,
      errorInfo: null,
    });
  };

  render() {
    if (this.state.hasError) {
      const FallbackComponent = this.props.fallback || DefaultErrorFallback;
      return (
        <FallbackComponent
          error={this.state.error}
          errorInfo={this.state.errorInfo}
          resetError={this.resetError}
        />
      );
    }

    return this.props.children;
  }
}

/**
 * Higher-order component version for wrapping components
 *
 * Usage:
 * ```tsx
 * const MyComponentWithErrorBoundary = withErrorBoundary(MyComponent);
 * ```
 */
export function withErrorBoundary<P extends object>(
  Component: React.ComponentType<P>,
  errorBoundaryProps?: Omit<ErrorBoundaryProps, 'children'>
): React.ComponentType<P> {
  const WrappedComponent = (props: P) => (
    <ErrorBoundary {...errorBoundaryProps}>
      <Component {...props} />
    </ErrorBoundary>
  );

  WrappedComponent.displayName = `withErrorBoundary(${Component.displayName || Component.name || 'Component'})`;

  return WrappedComponent;
}
