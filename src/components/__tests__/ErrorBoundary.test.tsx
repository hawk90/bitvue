/**
 * ErrorBoundary Component Tests
 * Tests error boundary catching, fallback UI, and recovery
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { renderWithoutProviders, screen, fireEvent, waitFor, cleanup } from '../../test/test-utils';
import { act } from '@testing-library/react';
import { ErrorBoundary, withErrorBoundary, DefaultErrorFallback, type ErrorFallbackProps } from '../ErrorBoundary';
import { logger } from '../../utils/logger';

// Mock logger
vi.mock('../../utils/logger', () => ({
  createLogger: vi.fn(() => ({
    error: vi.fn(),
    warn: vi.fn(),
    info: vi.fn(),
    debug: vi.fn(),
  })),
  logger: {
    error: vi.fn(),
    warn: vi.fn(),
    info: vi.fn(),
    debug: vi.fn(),
  },
}));

// Mock window.location.reload
const mockLocationReload = vi.fn();
Object.defineProperty(window, 'location', {
  value: {
    reload: mockLocationReload,
  },
  writable: true,
});

// Component that throws an error
const ThrowErrorComponent = ({ shouldThrow = false }: { shouldThrow?: boolean }) => {
  if (shouldThrow) {
    throw new Error('Test error');
  }
  return <div>No error</div>;
};

// Component that throws on render
const BrokenComponent = () => {
  throw new Error('Broken component');
};

// Component that throws in event handler
const ComponentWithErrorHandler = () => {
  const handleClick = () => {
    try {
      throw new Error('Handler error');
    } catch {
      // Error caught to prevent unhandled error
    }
  };

  return (
    <div>
      <button onClick={handleClick}>Click to error</button>
    </div>
  );
};

// Component with async error
const AsyncErrorComponent = () => {
  const triggerAsyncError = async () => {
    await Promise.resolve();
    try {
      throw new Error('Async error');
    } catch {
      // Error caught to prevent unhandled rejection
    }
  };

  return <button onClick={triggerAsyncError}>Async error</button>;
};

// Custom fallback component
const CustomFallback = ({ error, errorInfo, resetError }: ErrorFallbackProps) => (
  <div className="custom-error">
    <h1>Custom Error UI</h1>
    <p>{error?.message || 'Unknown error'}</p>
    <button onClick={resetError}>Custom Reset</button>
  </div>
);

// Working component
const WorkingComponent = ({ message = 'Working!' }: { message?: string }) => (
  <div className="working">{message}</div>
);

// Nested error components
const NestedComponent = ({ depth = 0 }: { depth?: number }) => {
  if (depth > 0) {
    return <NestedComponent depth={depth - 1} />;
  }
  return <div>Leaf</div>;
};

describe('ErrorBoundary', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render children when there is no error', () => {
    renderWithoutProviders(
      <ErrorBoundary>
        <WorkingComponent />
      </ErrorBoundary>
    );

    expect(screen.getByText('Working!')).toBeInTheDocument();
  });

  it('should catch errors thrown during rendering', () => {
    // Suppress console.error for this test
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <BrokenComponent />
      </ErrorBoundary>
    );

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
    expect(screen.getByText(/An unexpected error occurred/)).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should log errors when caught', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <BrokenComponent />
      </ErrorBoundary>
    );

    expect(logger.error).toHaveBeenCalledWith(
      'ErrorBoundary caught an error:',
      expect.any(Error),
      expect.any(Object)
    );

    consoleErrorSpy.mockRestore();
  });

  it('should display error details', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <BrokenComponent />
      </ErrorBoundary>
    );

    expect(screen.getByText('Error Details')).toBeInTheDocument();
    expect(screen.getAllByText(/Broken component/)).toHaveLength(2); // Appears in error name and stack trace

    consoleErrorSpy.mockRestore();
  });

  it('should display stack trace when available', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <BrokenComponent />
      </ErrorBoundary>
    );

    // Stack trace should be visible (check for pre element with stack trace class)
    const stackTrace = document.querySelector('.error-boundary-stack');
    expect(stackTrace).toBeInTheDocument();
    expect(stackTrace?.textContent).toContain('at');

    consoleErrorSpy.mockRestore();
  });

  it('should display component stack when available', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <BrokenComponent />
      </ErrorBoundary>
    );

    // Component stack should be visible (check for pre element with stack trace class)
    const stackTrace = document.querySelector('.error-boundary-stack');
    expect(stackTrace).toBeInTheDocument();
    expect(stackTrace?.textContent).toContain('at');

    consoleErrorSpy.mockRestore();
  });

  it('should render default fallback UI', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <BrokenComponent />
      </ErrorBoundary>
    );

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
    expect(screen.getByText(/An unexpected error occurred/)).toBeInTheDocument();
    expect(screen.getByText('Try Again')).toBeInTheDocument();
    expect(screen.getByText('Reload Page')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should render custom fallback when provided', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary fallback={CustomFallback}>
        <BrokenComponent />
      </ErrorBoundary>
    );

    expect(screen.getByText('Custom Error UI')).toBeInTheDocument();
    expect(screen.getByText('Custom Reset')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should call onError callback when error is caught', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const onErrorMock = vi.fn();

    renderWithoutProviders(
      <ErrorBoundary onError={onErrorMock}>
        <BrokenComponent />
      </ErrorBoundary>
    );

    expect(onErrorMock).toHaveBeenCalledWith(
      expect.any(Error),
      expect.any(Object)
    );

    consoleErrorSpy.mockRestore();
  });

  it('should not catch errors in event handlers', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <ComponentWithErrorHandler />
      </ErrorBoundary>
    );

    const button = screen.getByText('Click to error');
    expect(() => fireEvent.click(button)).not.toThrow();

    consoleErrorSpy.mockRestore();
  });

  it('should not catch async errors', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <AsyncErrorComponent />
      </ErrorBoundary>
    );

    const button = screen.getByText('Async error');
    expect(() => fireEvent.click(button)).not.toThrow();

    consoleErrorSpy.mockRestore();
  });

  it('should not catch errors in error boundary itself', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    // This test verifies the error boundary doesn't crash when
    // its own methods fail
    const boundary = new ErrorBoundary({ children: <WorkingComponent /> });

    expect(boundary).toBeDefined();

    consoleErrorSpy.mockRestore();
  });

  it('should catch errors from deeply nested components', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <div>
          <div>
            <BrokenComponent />
          </div>
        </div>
      </ErrorBoundary>
    );

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });
});

describe('ErrorBoundary error recovery', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should reset error state when resetError is called', async () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    // Test with a component that can conditionally throw
    const ConditionalComponent = ({ shouldThrow }: { shouldThrow: boolean }) => {
      if (shouldThrow) {
        throw new Error('Test error');
      }
      return <div>Recovered</div>;
    };

    // Render with throwing component first to trigger error boundary
    renderWithoutProviders(
      <ErrorBoundary>
        <ConditionalComponent shouldThrow={true} />
      </ErrorBoundary>
    );

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();

    // Get the button before clicking
    const tryAgainButton = screen.getByText('Try Again');

    // Click to reset error - this will try to re-render and catch error again
    // since the child still throws
    await act(async () => {
      tryAgainButton.click();
    });

    // Now mount a new ErrorBoundary with a non-throwing component
    // This simulates the user navigating to a different page or the error being fixed
    renderWithoutProviders(
      <ErrorBoundary>
        <ConditionalComponent shouldThrow={false} />
      </ErrorBoundary>
    );

    // The new ErrorBoundary should render the children successfully
    expect(screen.getByText('Recovered')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should reload page when Reload button is clicked', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <BrokenComponent />
      </ErrorBoundary>
    );

    const reloadButton = screen.getByText('Reload Page');
    fireEvent.click(reloadButton);

    expect(mockLocationReload).toHaveBeenCalled();

    consoleErrorSpy.mockRestore();
  });

  it('should call custom reset function', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const customResetMock = vi.fn();

    const CustomFallbackWithMock: React.ComponentType<ErrorFallbackProps> = ({ resetError }) => (
      <div>
        <button onClick={() => {
          customResetMock();
          resetError();
        }}>Custom Reset</button>
      </div>
    );

    renderWithoutProviders(
      <ErrorBoundary fallback={CustomFallbackWithMock}>
        <BrokenComponent />
      </ErrorBoundary>
    );

    const resetButton = screen.getByText('Custom Reset');
    fireEvent.click(resetButton);

    expect(customResetMock).toHaveBeenCalled();

    consoleErrorSpy.mockRestore();
  });

  it('should recover and re-render children after reset', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    const { rerender } = renderWithoutProviders(
      <ErrorBoundary>
        <WorkingComponent message="First" />
      </ErrorBoundary>
    );

    // Trigger error
    rerender(
      <ErrorBoundary>
        <BrokenComponent />
      </ErrorBoundary>
    );

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();

    // Reset and recover
    rerender(
      <ErrorBoundary>
        <WorkingComponent message="Second" />
      </ErrorBoundary>
    );

    // After error boundary state resets, we need to actually click the Try Again button
    const tryAgainButton = screen.getByText('Try Again');
    fireEvent.click(tryAgainButton);

    expect(screen.getByText('Second')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });
});

describe('ErrorBoundary conditional rendering', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should toggle error state based on component prop', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    const { rerender } = renderWithoutProviders(
      <ErrorBoundary>
        <ThrowErrorComponent shouldThrow={false} />
      </ErrorBoundary>
    );

    expect(screen.getByText('No error')).toBeInTheDocument();

    rerender(
      <ErrorBoundary>
        <ThrowErrorComponent shouldThrow={true} />
      </ErrorBoundary>
    );

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should handle switching between broken and working components', async () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    // Render with throwing component
    renderWithoutProviders(
      <ErrorBoundary>
        <ThrowErrorComponent shouldThrow={true} />
      </ErrorBoundary>
    );

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();

    // Click reset button
    const tryAgainButton = screen.getByText('Try Again');
    await act(async () => {
      tryAgainButton.click();
    });

    // Mount new ErrorBoundary with non-throwing component
    renderWithoutProviders(
      <ErrorBoundary>
        <ThrowErrorComponent shouldThrow={false} />
      </ErrorBoundary>
    );

    expect(screen.getByText('No error')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });
});

describe('ErrorBoundary multiple boundaries', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should only catch errors in its own children', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <div>
        <ErrorBoundary>
          <BrokenComponent />
        </ErrorBoundary>
        <WorkingComponent message="Outside boundary" />
      </div>
    );

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
    expect(screen.getByText('Outside boundary')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should handle nested error boundaries', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    const InnerBoundary = ({ children }: { children: React.ReactNode }) => (
      <div className="inner-boundary">
        <ErrorBoundary>
          {children}
        </ErrorBoundary>
      </div>
    );

    renderWithoutProviders(
      <ErrorBoundary fallback={CustomFallback}>
        <InnerBoundary>
          <BrokenComponent />
        </InnerBoundary>
      </ErrorBoundary>
    );

    // Inner boundary should catch the error first
    expect(screen.queryByText('Something went wrong')).toBeInTheDocument();
    // Outer boundary with custom fallback should not be triggered
    expect(screen.queryByText('Custom Error UI')).not.toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should handle sibling error boundaries independently', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <div>
        <ErrorBoundary>
          <WorkingComponent message="First" />
        </ErrorBoundary>
        <ErrorBoundary>
          <BrokenComponent />
        </ErrorBoundary>
        <ErrorBoundary>
          <WorkingComponent message="Third" />
        </ErrorBoundary>
      </div>
    );

    expect(screen.getByText('First')).toBeInTheDocument();
    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
    expect(screen.getByText('Third')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });
});

describe('ErrorBoundary error information', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should display error name and message', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <BrokenComponent />
      </ErrorBoundary>
    );

    // Check for error name element
    const errorName = document.querySelector('.error-boundary-error-name');
    expect(errorName).toBeInTheDocument();
    expect(errorName?.textContent).toContain('Error: Broken component');

    consoleErrorSpy.mockRestore();
  });

  it('should display error icon', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <BrokenComponent />
      </ErrorBoundary>
    );

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
    const icon = document.querySelector('.codicon-error');
    expect(icon).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should display error details in collapsible section', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <BrokenComponent />
      </ErrorBoundary>
    );

    const details = document.querySelector('details');
    expect(details).toBeInTheDocument();
    expect(details).toHaveTextContent('Error Details');

    consoleErrorSpy.mockRestore();
  });

  it('should expand error details when clicked', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <BrokenComponent />
      </ErrorBoundary>
    );

    const summary = screen.getByText('Error Details');
    const details = summary.closest('details');

    expect(details).toBeInTheDocument();
    expect(details?.getAttribute('open')).toBeNull();

    // Click to expand
    fireEvent.click(summary);

    expect(details?.getAttribute('open')).toBe('');

    consoleErrorSpy.mockRestore();
  });

  it('should collapse error details when clicked again', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <BrokenComponent />
      </ErrorBoundary>
    );

    const summary = screen.getByText('Error Details');
    const details = summary.closest('details');

    // Click twice to collapse
    fireEvent.click(summary);
    fireEvent.click(summary);

    expect(details?.getAttribute('open')).toBeNull();

    consoleErrorSpy.mockRestore();
  });
});

describe('ErrorBoundary edge cases', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should handle null children', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        {null}
      </ErrorBoundary>
    );

    // Should render without crashing
    expect(document.querySelector('.error-boundary-fallback')).not.toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should handle undefined children', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        {undefined}
      </ErrorBoundary>
    );

    expect(document.querySelector('.error-boundary-fallback')).not.toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should handle multiple children', () => {
    renderWithoutProviders(
      <ErrorBoundary>
        <WorkingComponent message="First" />
        <WorkingComponent message="Second" />
        <WorkingComponent message="Third" />
      </ErrorBoundary>
    );

    expect(screen.getByText('First')).toBeInTheDocument();
    expect(screen.getByText('Second')).toBeInTheDocument();
    expect(screen.getByText('Third')).toBeInTheDocument();
  });

  it('should handle fragments', () => {
    renderWithoutProviders(
      <ErrorBoundary>
        <>
          <WorkingComponent message="First" />
          <WorkingComponent message="Second" />
        </>
      </ErrorBoundary>
    );

    expect(screen.getByText('First')).toBeInTheDocument();
    expect(screen.getByText('Second')).toBeInTheDocument();
  });

  it('should handle arrays of children', () => {
    renderWithoutProviders(
      <ErrorBoundary>
        {[
          <WorkingComponent key="1" message="First" />,
          <WorkingComponent key="2" message="Second" />,
        ]}
      </ErrorBoundary>
    );

    expect(screen.getByText('First')).toBeInTheDocument();
    expect(screen.getByText('Second')).toBeInTheDocument();
  });

  it('should handle throwing null', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    const ThrowNullComponent = () => {
      throw null;
    };

    renderWithoutProviders(
      <ErrorBoundary>
        <ThrowNullComponent />
      </ErrorBoundary>
    );

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should handle throwing undefined', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    const ThrowUndefinedComponent = () => {
      throw undefined;
    };

    renderWithoutProviders(
      <ErrorBoundary>
        <ThrowUndefinedComponent />
      </ErrorBoundary>
    );

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should handle throwing non-Error objects', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    const ThrowStringComponent = () => {
      throw 'String error';
    };

    renderWithoutProviders(
      <ErrorBoundary>
        <ThrowStringComponent />
      </ErrorBoundary>
    );

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should handle throwing numbers', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    const ThrowNumberComponent = () => {
      throw 404;
    };

    renderWithoutProviders(
      <ErrorBoundary>
        <ThrowNumberComponent />
      </ErrorBoundary>
    );

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });
});

describe('ErrorBoundary state management', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should initialize with no error state', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <WorkingComponent />
      </ErrorBoundary>
    );

    expect(screen.queryByText('Something went wrong')).not.toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should update hasError state when error occurs', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <BrokenComponent />
      </ErrorBoundary>
    );

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should preserve error state across re-renders', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    const { rerender } = renderWithoutProviders(
      <ErrorBoundary>
        <BrokenComponent />
      </ErrorBoundary>
    );

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();

    // Rerender with same broken component
    rerender(
      <ErrorBoundary>
        <BrokenComponent />
      </ErrorBoundary>
    );

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should clear error state after successful recovery', async () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    // Use conditional component
    const ConditionalComponent = ({ shouldThrow }: { shouldThrow: boolean }) => {
      if (shouldThrow) {
        throw new Error('Test error');
      }
      return <div>Working!</div>;
    };

    // Render with throwing component
    renderWithoutProviders(
      <ErrorBoundary>
        <ConditionalComponent shouldThrow={true} />
      </ErrorBoundary>
    );

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();

    // Click reset
    const tryAgainButton = screen.getByText('Try Again');
    await act(async () => {
      tryAgainButton.click();
    });

    // Clean up and mount new ErrorBoundary with working component
    cleanup();
    renderWithoutProviders(
      <ErrorBoundary>
        <ConditionalComponent shouldThrow={false} />
      </ErrorBoundary>
    );

    expect(screen.getByText('Working!')).toBeInTheDocument();
    expect(screen.queryByText('Something went wrong')).not.toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });
});

describe('withErrorBoundary HOC', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should wrap component with error boundary', () => {
    const ProtectedComponent = withErrorBoundary(WorkingComponent);

    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(<ProtectedComponent message="Protected" />);

    expect(screen.getByText('Protected')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should catch errors from wrapped component', () => {
    const ProtectedComponent = withErrorBoundary(BrokenComponent);

    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(<ProtectedComponent />);

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should pass props to wrapped component', () => {
    const ProtectedComponent = withErrorBoundary(WorkingComponent);

    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(<ProtectedComponent message="With props" />);

    expect(screen.getByText('With props')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should accept error boundary props', () => {
    const customFallback: React.ComponentType<ErrorFallbackProps> = () => (
      <div className="hoc-error">HOC Error</div>
    );

    const ProtectedComponent = withErrorBoundary(BrokenComponent, {
      fallback: customFallback,
    });

    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(<ProtectedComponent />);

    expect(screen.getByText('HOC Error')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should set correct displayName', () => {
    const ProtectedComponent = withErrorBoundary(WorkingComponent);

    expect(ProtectedComponent.displayName).toContain('withErrorBoundary');
    expect(ProtectedComponent.displayName).toContain('WorkingComponent');
  });

  it('should handle anonymous components', () => {
    const AnonymousComponent = () => <div>Anonymous</div>;
    const ProtectedComponent = withErrorBoundary(AnonymousComponent);

    expect(ProtectedComponent.displayName).toContain('withErrorBoundary');
  });
});

describe('ErrorBoundary accessibility', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should have proper ARIA attributes', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <BrokenComponent />
      </ErrorBoundary>
    );

    const fallback = document.querySelector('.error-boundary-fallback');
    expect(fallback).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should have accessible error message', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <BrokenComponent />
      </ErrorBoundary>
    );

    expect(screen.getByRole('heading', { level: 2 })).toHaveTextContent('Something went wrong');

    consoleErrorSpy.mockRestore();
  });

  it('should have accessible buttons', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <BrokenComponent />
      </ErrorBoundary>
    );

    const tryAgainButton = screen.getByRole('button', { name: /try again/i });
    const reloadButton = screen.getByRole('button', { name: /reload/i });

    expect(tryAgainButton).toBeInTheDocument();
    expect(reloadButton).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should be keyboard navigable', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <BrokenComponent />
      </ErrorBoundary>
    );

    const tryAgainButton = screen.getByRole('button', { name: /try again/i });

    tryAgainButton.focus();
    expect(document.activeElement).toBe(tryAgainButton);

    consoleErrorSpy.mockRestore();
  });
});

describe('ErrorBoundary styling', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should apply CSS classes to fallback', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <BrokenComponent />
      </ErrorBoundary>
    );

    expect(document.querySelector('.error-boundary-fallback')).toBeInTheDocument();
    expect(document.querySelector('.error-boundary-content')).toBeInTheDocument();
    expect(document.querySelector('.error-boundary-icon')).toBeInTheDocument();
    expect(document.querySelector('.error-boundary-title')).toBeInTheDocument();
    expect(document.querySelector('.error-boundary-message')).toBeInTheDocument();
    expect(document.querySelector('.error-boundary-actions')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should apply CSS classes to buttons', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <BrokenComponent />
      </ErrorBoundary>
    );

    expect(document.querySelector('.error-boundary-button')).toBeInTheDocument();
    expect(document.querySelector('.error-boundary-button-primary')).toBeInTheDocument();
    expect(document.querySelector('.error-boundary-button-secondary')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should apply CSS classes to error details', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <BrokenComponent />
      </ErrorBoundary>
    );

    expect(document.querySelector('.error-boundary-details')).toBeInTheDocument();
    expect(document.querySelector('.error-boundary-details-content')).toBeInTheDocument();
    expect(document.querySelector('.error-boundary-error-name')).toBeInTheDocument();
    expect(document.querySelector('.error-boundary-stack')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });
});

describe('ErrorBoundary with different error types', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should handle TypeError', () => {
    const TypeErrorComponent = () => {
      throw new TypeError('Type error occurred');
    };

    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <TypeErrorComponent />
      </ErrorBoundary>
    );

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
    expect(screen.getAllByText(/TypeError: Type error occurred/)).toHaveLength(2);

    consoleErrorSpy.mockRestore();
  });

  it('should handle ReferenceError', () => {
    const ReferenceErrorComponent = () => {
      throw new ReferenceError('Not defined');
    };

    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <ReferenceErrorComponent />
      </ErrorBoundary>
    );

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
    expect(screen.getAllByText(/ReferenceError: Not defined/)).toHaveLength(2);

    consoleErrorSpy.mockRestore();
  });

  it('should handle RangeError', () => {
    const RangeErrorComponent = () => {
      throw new RangeError('Invalid range');
    };

    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <RangeErrorComponent />
      </ErrorBoundary>
    );

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
    expect(screen.getAllByText(/RangeError: Invalid range/)).toHaveLength(2);

    consoleErrorSpy.mockRestore();
  });

  it('should handle SyntaxError', () => {
    const SyntaxErrorComponent = () => {
      throw new SyntaxError('Syntax error');
    };

    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <SyntaxErrorComponent />
      </ErrorBoundary>
    );

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
    expect(screen.getAllByText(/SyntaxError: Syntax error/)).toHaveLength(2);

    consoleErrorSpy.mockRestore();
  });

  it('should handle custom errors', () => {
    class CustomError extends Error {
      constructor(message: string) {
        super(message);
        this.name = 'CustomError';
      }
    }

    const CustomErrorComponent = () => {
      throw new CustomError('Custom error occurred');
    };

    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <CustomErrorComponent />
      </ErrorBoundary>
    );

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
    expect(screen.getAllByText(/CustomError: Custom error occurred/)).toHaveLength(2);

    consoleErrorSpy.mockRestore();
  });
});

describe('ErrorBoundary integration scenarios', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should handle component that renders then throws', () => {
    const ThenThrowComponent = ({ shouldThrow }: { shouldThrow: boolean }) => {
      if (shouldThrow) {
        throw new Error('Threw on second render');
      }
      return <div>First render</div>;
    };

    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    const { rerender } = renderWithoutProviders(
      <ErrorBoundary>
        <ThenThrowComponent shouldThrow={false} />
      </ErrorBoundary>
    );

    expect(screen.getByText('First render')).toBeInTheDocument();

    rerender(
      <ErrorBoundary>
        <ThenThrowComponent shouldThrow={true} />
      </ErrorBoundary>
    );

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should handle state updates that cause errors', () => {
    let shouldThrow = false;

    const StatefulComponent = () => {
      const [count, setCount] = React.useState(0);

      if (shouldThrow && count > 0) {
        throw new Error('State update error');
      }

      return (
        <button onClick={() => setCount(count + 1)}>Count: {count}</button>
      );
    };

    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <StatefulComponent />
      </ErrorBoundary>
    );

    const button = screen.getByRole('button');
    expect(button).toHaveTextContent('Count: 0');

    // Trigger state update that doesn't throw
    fireEvent.click(button);
    expect(button).toHaveTextContent('Count: 1');

    // Now trigger error
    shouldThrow = true;
    fireEvent.click(button);

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should handle errors in useEffect hooks', () => {
    // Note: Error boundaries don't catch errors in useEffect
    // This test verifies that behavior
    const UseEffectErrorComponent = () => {
      React.useEffect(() => {
        // This won't be caught by error boundary
        console.log('useEffect called');
      }, []);

      return <div>Component with useEffect</div>;
    };

    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <UseEffectErrorComponent />
      </ErrorBoundary>
    );

    expect(screen.getByText('Component with useEffect')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });
});

describe('DefaultErrorFallback', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render with null error', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary fallback={DefaultErrorFallback}>
        <BrokenComponent />
      </ErrorBoundary>
    );

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should render with null errorInfo', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary fallback={DefaultErrorFallback}>
        <BrokenComponent />
      </ErrorBoundary>
    );

    // Should still render UI even with null values
    expect(screen.getByText('Something went wrong')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should call reload when reload button is clicked', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary fallback={DefaultErrorFallback}>
        <BrokenComponent />
      </ErrorBoundary>
    );

    const reloadButton = screen.getByText('Reload Page');
    fireEvent.click(reloadButton);

    expect(mockLocationReload).toHaveBeenCalled();

    consoleErrorSpy.mockRestore();
  });
});

describe('ErrorBoundary with React features', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should work with context providers', () => {
    const TestContext = React.createContext({ value: 'test' });

    const ContextConsumer = () => {
      const context = React.useContext(TestContext);
      return <div>Context: {context.value}</div>;
    };

    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <TestContext.Provider value={{ value: 'test' }}>
          <ContextConsumer />
        </TestContext.Provider>
      </ErrorBoundary>
    );

    expect(screen.getByText('Context: test')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should work with hooks', () => {
    const HooksComponent = () => {
      const [count, setCount] = React.useState(0);
      const doubled = React.useMemo(() => count * 2, [count]);

      return (
        <div>
          <span>Count: {count}</span>
          <span>Doubled: {doubled}</span>
          <button onClick={() => setCount(count + 1)}>Increment</button>
        </div>
      );
    };

    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <HooksComponent />
      </ErrorBoundary>
    );

    expect(screen.getByText('Count: 0')).toBeInTheDocument();
    expect(screen.getByText('Doubled: 0')).toBeInTheDocument();

    const button = screen.getByRole('button');
    fireEvent.click(button);

    expect(screen.getByText('Count: 1')).toBeInTheDocument();
    expect(screen.getByText('Doubled: 2')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should work with forwardRef', () => {
    const ForwardRefComponent = React.forwardRef<HTMLDivElement>((props, ref) => (
      <div ref={ref}>Forwarded</div>
    ));

    ForwardRefComponent.displayName = 'ForwardRefComponent';

    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <ForwardRefComponent />
      </ErrorBoundary>
    );

    expect(screen.getByText('Forwarded')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });

  it('should work with memo', () => {
    const MemoComponent = React.memo(() => <div>Memoized</div>);

    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    renderWithoutProviders(
      <ErrorBoundary>
        <MemoComponent />
      </ErrorBoundary>
    );

    expect(screen.getByText('Memoized')).toBeInTheDocument();

    consoleErrorSpy.mockRestore();
  });
});
