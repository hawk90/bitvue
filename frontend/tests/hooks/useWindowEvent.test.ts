/**
 * useWindowEvent Hook Tests
 * Tests window event subscription with cleanup
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook } from "@testing-library/react";
import { useWindowEvent, useWindowEvents } from "@/hooks/useWindowEvent";

// Mock window.addEventListener and removeEventListener
const addEventListenerSpy = vi.spyOn(window, "addEventListener");
const removeEventListenerSpy = vi.spyOn(window, "removeEventListener");

describe("useWindowEvent", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    addEventListenerSpy.mockClear();
    removeEventListenerSpy.mockClear();
  });

  it("should add event listener on mount", () => {
    const handler = vi.fn();

    renderHook(() => useWindowEvent("keydown", handler));

    expect(addEventListenerSpy).toHaveBeenCalledWith(
      "keydown",
      expect.any(Function),
      undefined,
    );
  });

  it("should remove event listener on unmount", () => {
    const handler = vi.fn();
    const { unmount } = renderHook(() => useWindowEvent("keydown", handler));

    unmount();

    expect(removeEventListenerSpy).toHaveBeenCalledWith(
      "keydown",
      expect.any(Function),
    );
  });

  it("should pass options to addEventListener", () => {
    const handler = vi.fn();
    const options = { passive: true };

    renderHook(() => useWindowEvent("scroll", handler, options));

    expect(addEventListenerSpy).toHaveBeenCalledWith(
      "scroll",
      expect.any(Function),
      options,
    );
  });

  it("should call handler when event is triggered", () => {
    const handler = vi.fn();

    renderHook(() => useWindowEvent("keydown", handler));

    // Get the registered listener
    const listener = addEventListenerSpy.mock.calls[0][1];

    // Simulate event
    const mockEvent = new KeyboardEvent("keydown", { key: "Escape" });
    listener(mockEvent);

    expect(handler).toHaveBeenCalledWith(mockEvent);
  });

  it("should update handler when handler function changes", () => {
    const handler1 = vi.fn();
    const handler2 = vi.fn();

    const { rerender } = renderHook(({ h }) => useWindowEvent("keydown", h), {
      initialProps: { h: handler1 },
    });

    rerender({ h: handler2 });

    // Hook uses ref pattern - listener stays registered, handler updates via ref
    // No re-registration needed when only handler changes
    expect(addEventListenerSpy).toHaveBeenCalledTimes(1);
    expect(removeEventListenerSpy).toHaveBeenCalledTimes(0);
  });

  it("should support multiple event types", () => {
    renderHook(() => {
      useWindowEvent("resize", vi.fn());
      useWindowEvent("scroll", vi.fn());
      useWindowEvent("keydown", vi.fn());
    });

    expect(addEventListenerSpy).toHaveBeenCalledTimes(3);
  });
});

describe("useWindowEvents", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    addEventListenerSpy.mockClear();
    removeEventListenerSpy.mockClear();
  });

  it("should register multiple event listeners", () => {
    const handlers = {
      keydown: vi.fn(),
      resize: vi.fn(),
      scroll: vi.fn(),
    };

    renderHook(() => useWindowEvents(handlers));

    expect(addEventListenerSpy).toHaveBeenCalledTimes(3);
  });

  it("should remove all event listeners on unmount", () => {
    const handlers = {
      keydown: vi.fn(),
      resize: vi.fn(),
    };

    const { unmount } = renderHook(() => useWindowEvents(handlers));

    unmount();

    expect(removeEventListenerSpy).toHaveBeenCalledTimes(2);
  });

  it("should pass options to all listeners", () => {
    const handlers = {
      keydown: vi.fn(),
      resize: vi.fn(),
    };
    const options = { passive: true };

    renderHook(() => useWindowEvents(handlers, options));

    expect(addEventListenerSpy).toHaveBeenCalledWith(
      "keydown",
      expect.any(Function),
      options,
    );
    expect(addEventListenerSpy).toHaveBeenCalledWith(
      "resize",
      expect.any(Function),
      options,
    );
  });

  it("should call correct handler for each event", () => {
    const handlers = {
      keydown: vi.fn(),
      resize: vi.fn(),
    };

    renderHook(() => useWindowEvents(handlers));

    // Get the registered listeners
    const keydownListener = addEventListenerSpy.mock.calls.find(
      (c) => c[0] === "keydown",
    )?.[1];
    const resizeListener = addEventListenerSpy.mock.calls.find(
      (c) => c[0] === "resize",
    )?.[1];

    // Trigger events
    keydownListener(new KeyboardEvent("keydown", { key: "Escape" }));
    resizeListener(new Event("resize"));

    expect(handlers.keydown).toHaveBeenCalledTimes(1);
    expect(handlers.resize).toHaveBeenCalledTimes(1);
  });

  it("should handle empty events object", () => {
    const { unmount } = renderHook(() => useWindowEvents({}));

    expect(addEventListenerSpy).not.toHaveBeenCalled();

    unmount();

    expect(removeEventListenerSpy).not.toHaveBeenCalled();
  });

  it("should update when handlers change", () => {
    const handlers1 = { keydown: vi.fn() };
    const handlers2 = { resize: vi.fn() };

    const { rerender } = renderHook(({ h }) => useWindowEvents(h), {
      initialProps: { h: handlers1 },
    });

    rerender({ h: handlers2 });

    expect(addEventListenerSpy).toHaveBeenCalledTimes(2); // initial + update
    expect(removeEventListenerSpy).toHaveBeenCalledTimes(1); // cleanup of first
  });
});

describe("useWindowEvent edge cases", () => {
  it("should handle events with no payload", () => {
    const handler = vi.fn();

    renderHook(() => useWindowEvent("focus", handler));

    const listener = addEventListenerSpy.mock.calls[0][1];

    listener(new Event("focus"));

    expect(handler).toHaveBeenCalled();
  });

  it("should handle events with complex payloads", () => {
    const handler = vi.fn();

    renderHook(() => useWindowEvent("resize", handler));

    // Use find to get the resize listener specifically (previous tests may have registered other listeners)
    const listener = addEventListenerSpy.mock.calls.find(
      (c) => c[0] === "resize",
    )?.[1];

    listener?.(new Event("resize"));

    expect(handler).toHaveBeenCalledWith(expect.any(Event));
  });

  it("should handle once option", () => {
    const handler = vi.fn();
    const options = { once: true };

    renderHook(() => useWindowEvent("click", handler, options));

    expect(addEventListenerSpy).toHaveBeenCalledWith(
      "click",
      expect.any(Function),
      options,
    );
  });

  it("should handle capture option", () => {
    const handler = vi.fn();
    const options = { capture: true };

    renderHook(() => useWindowEvent("click", handler, options));

    expect(addEventListenerSpy).toHaveBeenCalledWith(
      "click",
      expect.any(Function),
      options,
    );
  });

  it("should handle signal option", () => {
    const handler = vi.fn();
    const abortController = new AbortController();
    const options = { signal: abortController.signal };

    renderHook(() => useWindowEvent("click", handler, options));

    expect(addEventListenerSpy).toHaveBeenCalledWith(
      "click",
      expect.any(Function),
      options,
    );
  });
});
