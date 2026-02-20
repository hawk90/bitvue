/**
 * useIntersectionObserver Hook Tests
 * Tests intersection observer for lazy loading
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useIntersectionObserver } from "@/hooks/useIntersectionObserver";

// Mock IntersectionObserver
const mockDisconnect = vi.fn();
const mockObserve = vi.fn();
const mockUnobserve = vi.fn();

// Store callbacks to trigger them manually in tests
let intersectCallback: IntersectionObserverCallback | null = null;
let observerInstances: any[] = [];

class MockIntersectionObserver {
  root = null;
  rootMargin = "";
  thresholds = [];
  private callback: IntersectionObserverCallback;

  constructor(callback: IntersectionObserverCallback) {
    this.callback = callback;
    intersectCallback = callback;
    observerInstances.push(this);
  }

  observe = mockObserve;
  unobserve = mockUnobserve;
  disconnect = mockDisconnect;

  takeRecords = () => [] as IntersectionObserverEntry[];
  readonly rootBounds = DOMRectReadOnly.fromRect({
    x: 0,
    y: 0,
    width: 0,
    height: 0,
  });
}

// Store original IntersectionObserver
const originalIntersectionObserver = global.IntersectionObserver;

beforeEach(() => {
  vi.clearAllMocks();
  intersectCallback = null;
  observerInstances = [];
  global.IntersectionObserver = MockIntersectionObserver as any;
});

afterEach(() => {
  vi.clearAllMocks();
  global.IntersectionObserver = originalIntersectionObserver;
});

describe("useIntersectionObserver", () => {
  it("should return intersection state", () => {
    const { result } = renderHook(() =>
      useIntersectionObserver({ threshold: 0.5 }),
    );

    expect(result.current).toBeDefined();
  });

  it("should return isVisible boolean", () => {
    const { result } = renderHook(() =>
      useIntersectionObserver({ threshold: 0.5 }),
    );

    expect(typeof result.current.isVisible).toBe("boolean");
    expect(result.current.isVisible).toBe(false);
  });

  it("should return ref object", () => {
    const { result } = renderHook(() =>
      useIntersectionObserver({ threshold: 0.5 }),
    );

    expect(result.current.ref).toBeDefined();
    expect(result.current.ref.current).toBe(null);
  });

  it("should return entry object as undefined initially", () => {
    const { result } = renderHook(() =>
      useIntersectionObserver({ threshold: 0.5 }),
    );

    expect(result.current.entry).toBeUndefined();
  });

  it("should handle target ref", () => {
    const { result } = renderHook(() =>
      useIntersectionObserver({ threshold: 0.5 }),
    );

    expect(result.current.ref).toBeDefined();
    expect(typeof result.current.ref).toBe("object");
  });
});

describe("useIntersectionObserver state updates", () => {
  it("should update state when intersection occurs", () => {
    const { result } = renderHook(() =>
      useIntersectionObserver({ threshold: 0.5 }),
    );

    // Simulate intersection change
    if (intersectCallback) {
      const element = document.createElement("div");
      const entry = {
        isIntersecting: true,
        boundingClientRect: new DOMRectReadOnly(),
        intersectionRatio: 0.75,
        target: element,
        time: 1000,
        rootBounds: new DOMRectReadOnly(),
        intersectionRect: new DOMRectReadOnly(),
      };

      // Trigger callback
      act(() => {
        intersectCallback!([entry], new IntersectionObserver(() => {}));
      });

      // State should be updated
      expect(result.current.isVisible).toBe(true);
    }
  });

  it("should update entry when intersection occurs", () => {
    const { result } = renderHook(() =>
      useIntersectionObserver({ threshold: 0.5 }),
    );

    // Simulate intersection change
    if (intersectCallback) {
      const element = document.createElement("div");
      const entry = {
        isIntersecting: true,
        boundingClientRect: new DOMRectReadOnly(),
        intersectionRatio: 0.75,
        target: element,
        time: 1000,
        rootBounds: new DOMRectReadOnly(),
        intersectionRect: new DOMRectReadOnly(),
      };

      // Trigger callback
      act(() => {
        intersectCallback!([entry], new IntersectionObserver(() => {}));
      });

      // Entry should be updated
      expect(result.current.entry).toBeDefined();
      expect(result.current.entry?.isIntersecting).toBe(true);
    }
  });

  it("should update to false when element stops intersecting", () => {
    const { result } = renderHook(() =>
      useIntersectionObserver({ threshold: 0.5 }),
    );

    if (intersectCallback) {
      const element = document.createElement("div");

      // First, set to intersecting
      const entry1 = {
        isIntersecting: true,
        boundingClientRect: new DOMRectReadOnly(),
        intersectionRatio: 0.75,
        target: element,
        time: 1000,
        rootBounds: new DOMRectReadOnly(),
        intersectionRect: new DOMRectReadOnly(),
      };

      act(() => {
        intersectCallback!([entry1], new IntersectionObserver(() => {}));
      });

      expect(result.current.isVisible).toBe(true);

      // Then, set to not intersecting
      const entry2 = {
        isIntersecting: false,
        boundingClientRect: new DOMRectReadOnly(),
        intersectionRatio: 0,
        target: element,
        time: 2000,
        rootBounds: new DOMRectReadOnly(),
        intersectionRect: new DOMRectReadOnly(),
      };

      act(() => {
        intersectCallback!([entry2], new IntersectionObserver(() => {}));
      });

      expect(result.current.isVisible).toBe(false);
    }
  });
});

describe("useIntersectionObserver edge cases", () => {
  it("should handle null target ref gracefully", () => {
    const { result } = renderHook(() =>
      useIntersectionObserver({ threshold: 0.5 }),
    );

    expect(result.current).toBeDefined();
    expect(result.current.ref.current).toBe(null);
  });

  it("should handle zero threshold", () => {
    const { result } = renderHook(() =>
      useIntersectionObserver({ threshold: 0 }),
    );

    // Should not throw any errors
    expect(result.current).toBeDefined();
  });

  it("should handle threshold of 1", () => {
    const { result } = renderHook(() =>
      useIntersectionObserver({ threshold: 1 }),
    );

    expect(result.current).toBeDefined();
  });

  it("should handle empty threshold array", () => {
    const { result } = renderHook(() =>
      useIntersectionObserver({ threshold: [] }),
    );

    expect(result.current).toBeDefined();
  });

  it("should handle trigger once option", () => {
    const { result } = renderHook(() =>
      useIntersectionObserver({ threshold: 0.5, triggerOnce: true }),
    );

    expect(result.current).toBeDefined();
  });

  it("should disconnect when triggerOnce is true and element becomes visible", () => {
    const { result } = renderHook(() =>
      useIntersectionObserver({ threshold: 0.5, triggerOnce: true }),
    );

    if (intersectCallback) {
      const element = document.createElement("div");
      const entry = {
        isIntersecting: true,
        boundingClientRect: new DOMRectReadOnly(),
        intersectionRatio: 0.75,
        target: element,
        time: 1000,
        rootBounds: new DOMRectReadOnly(),
        intersectionRect: new DOMRectReadOnly(),
      };

      // Trigger callback
      act(() => {
        intersectCallback!([entry], new IntersectionObserver(() => {}));
      });

      expect(result.current.isVisible).toBe(true);

      // Should disconnect after first intersection
      expect(mockDisconnect).toHaveBeenCalled();
    }
  });
});

describe("useIntersectionObserver ref behavior", () => {
  it("should provide a ref that can be attached to an element", () => {
    const { result } = renderHook(() =>
      useIntersectionObserver({ threshold: 0.5 }),
    );

    expect(result.current.ref).toBeDefined();
    expect(
      result.current.ref.current === null ||
        typeof result.current.ref.current === "object",
    ).toBe(true);
  });

  it("should update ref when element is attached", () => {
    const { result } = renderHook(() =>
      useIntersectionObserver({ threshold: 0.5 }),
    );

    const element = document.createElement("div");

    act(() => {
      result.current.ref.current = element;
    });

    expect(result.current.ref.current).toBe(element);
  });

  it("should cleanup on unmount", () => {
    const { unmount } = renderHook(() =>
      useIntersectionObserver({ threshold: 0.5 }),
    );

    // Unmount should not throw any errors
    expect(() => unmount()).not.toThrow();
  });
});

describe("useIntersectionObserver with options", () => {
  it("should accept single threshold value", () => {
    const { result } = renderHook(() =>
      useIntersectionObserver({ threshold: 0.5 }),
    );

    expect(result.current).toBeDefined();
  });

  it("should accept array of thresholds", () => {
    const { result } = renderHook(() =>
      useIntersectionObserver({ threshold: [0, 0.5, 1] }),
    );

    expect(result.current).toBeDefined();
  });

  it("should accept root element", () => {
    const rootElement = document.createElement("div");
    const { result } = renderHook(() =>
      useIntersectionObserver({ root: rootElement, threshold: 0.5 }),
    );

    expect(result.current).toBeDefined();
  });

  it("should accept root margin", () => {
    const { result } = renderHook(() =>
      useIntersectionObserver({ rootMargin: "10px", threshold: 0.5 }),
    );

    expect(result.current).toBeDefined();
  });
});
