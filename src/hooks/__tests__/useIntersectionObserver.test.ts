/**
 * useIntersectionObserver Hook Tests
 * Tests intersection observer for lazy loading
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook } from '@testing-library/react';
import { useIntersectionObserver } from '../useIntersectionObserver';

// Mock IntersectionObserver
const mockIntersect = vi.fn();
const mockDisconnect = vi.fn();
const mockObserve = vi.fn();
const mockUnobserve = vi.fn();

class MockIntersectionObserver {
  root = null;
  rootMargin = '';
  thresholds = [];

  constructor(callback: IntersectionObserverCallback) {
    mockIntersect(callback);
  }

  observe = mockObserve;
  unobserve = mockUnobserve;
  disconnect = mockDisconnect;

  takeRecords = () => [] as IntersectionObserverEntry[];
  readonly rootBounds = DOMRectReadOnly.fromRect({ x: 0, y: 0, width: 0, height: 0 });
}

global.IntersectionObserver = MockIntersectionObserver as any;

describe('useIntersectionObserver', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('should return intersection state', () => {
    const { result } = renderHook(() => useIntersectionObserver({ threshold: 0.5 }));

    expect(result.current).toBeDefined();
  });

  it('should return isIntersecting boolean', () => {
    const { result } = renderHook(() => useIntersectionObserver({ threshold: 0.5 }));

    expect(typeof result.current.isIntersecting).toBe('boolean');
  });

  it('should return entry object', () => {
    const { result } = renderHook(() => useIntersectionObserver({ threshold: 0.5 }));

    expect(result.current.entry).toBeDefined();
  });

  it('should create observer on mount', () => {
    renderHook(() => useIntersectionObserver({ threshold: 0.5 }));

    expect(mockObserve).toHaveBeenCalled();
  });

  it('should disconnect observer on unmount', () => {
    const { unmount } = renderHook(() => useIntersectionObserver({ threshold: 0.5 }));

    unmount();

    expect(mockDisconnect).toHaveBeenCalled();
  });

  it('should handle target ref', () => {
    const targetRef = { current: null };
    const { result } = renderHook(() => useIntersectionObserver({ threshold: 0.5 }));

    // The hook should accept a targetRef parameter
    expect(result.current).toBeDefined();
  });
});

describe('useIntersectionObserver threshold', () => {
  it('should accept single threshold value', () => {
    renderHook(() => useIntersectionObserver({ threshold: 0.5 }));

    expect(mockObserve).toHaveBeenCalledWith(
      expect.anything(),
      expect.objectContaining({ threshold: 0.5 })
    );
  });

  it('should accept array of thresholds', () => {
    renderHook(() => useIntersectionObserver({ threshold: [0, 0.5, 1] }));

    expect(mockObserve).toHaveBeenCalledWith(
      expect.anything(),
      expect.objectContaining({ threshold: [0, 0.5, 1] })
    );
  });

  it('should accept root element', () => {
    const rootElement = document.createElement('div');

    renderHook(() => useIntersectionObserver({ root: rootElement, threshold: 0.5 }));

    expect(mockObserve).toHaveBeenCalledWith(
      expect.anything(),
      expect.objectContaining({ root: rootElement })
    );
  });

  it('should accept root margin', () => {
    renderHook(() => useIntersectionObserver({ rootMargin: '10px', threshold: 0.5 }));

    expect(mockObserve).toHaveBeenCalledWith(
      expect.anything(),
      expect.objectContaining({ rootMargin: '10px' })
    );
  });
});

describe('useIntersectionObserver state updates', () => {
  it('should update state when intersection occurs', () => {
    let callback: IntersectionObserverCallback | null = null;

    mockObserve.mockImplementation((cb) => {
      callback = cb;
      return mockDisconnect;
    });

    const { result, rerender } = renderHook(
      () => useIntersectionObserver({ threshold: 0.5 })
    );

    // Simulate intersection change
    if (callback) {
      const entry = {
        isIntersecting: true,
        boundingClientRect: new DOMRectReadOnly(),
        intersectionRatio: 0.75,
        target: new Element(),
        time: 1000,
        rootBounds: new DOMRectReadOnly(),
        intersectionRect: new DOMRectReadOnly(),
      };

      // Trigger callback
      callback([entry], new IntersectionObserver(() => {}));
    }

    // Should trigger re-render with updated state
    expect(result.current.isIntersecting).toBe(true);
  });
});

describe('useIntersectionObserver edge cases', () => {
  it('should handle null target ref gracefully', () => {
    const { result } = renderHook(() => useIntersectionObserver({ threshold: 0.5 }));

    expect(result.current).toBeDefined();
  });

  it('should handle zero threshold', () => {
    renderHook(() => useIntersectionObserver({ threshold: 0 }));

    expect(mockObserve).toHaveBeenCalled();
  });

  it('should handle threshold of 1', () => {
    renderHook(() => useIntersectionObserver({ threshold: 1 }));

    expect(mockObserve).toHaveBeenCalled();
  });

  it('should handle empty threshold array', () => {
    renderHook(() => useIntersectionObserver({ threshold: [] }));

    expect(mockObserve).toHaveBeenCalled();
  });

  it('should handle trigger once option', () => {
    renderHook(() => useIntersectionObserver({ threshold: 0.5, triggerOnce: true }));

    expect(mockObserve).toHaveBeenCalled();
  });
});
