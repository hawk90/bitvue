/**
 * useWindowEventMap Hook Tests
 */

import { describe, it, expect, vi, afterEach } from "vitest";
import { renderHook } from "@testing-library/react";
import { useWindowEventMap } from "@/hooks/useWindowEvent";

afterEach(() => {
  vi.restoreAllMocks();
});

describe("useWindowEventMap", () => {
  it("calls the handler registered for each event name", () => {
    const a = vi.fn();
    const b = vi.fn();
    renderHook(() => useWindowEventMap({ "x-a": a, "x-b": b }));
    window.dispatchEvent(new Event("x-a"));
    expect(a).toHaveBeenCalledTimes(1);
    expect(b).not.toHaveBeenCalled();
    window.dispatchEvent(new Event("x-b"));
    expect(b).toHaveBeenCalledTimes(1);
  });

  it("uses the latest handler without re-subscribing", () => {
    const addSpy = vi.spyOn(window, "addEventListener");
    const first = vi.fn();
    const second = vi.fn();
    const { rerender } = renderHook(
      ({ h }) => useWindowEventMap({ "x-latest": h }),
      { initialProps: { h: first } },
    );
    const addsBefore = addSpy.mock.calls.filter(
      ([t]) => t === "x-latest",
    ).length;
    rerender({ h: second });
    window.dispatchEvent(new Event("x-latest"));
    expect(first).not.toHaveBeenCalled();
    expect(second).toHaveBeenCalledTimes(1);
    expect(addSpy.mock.calls.filter(([t]) => t === "x-latest").length).toBe(
      addsBefore,
    );
  });

  it("removes every listener on unmount", () => {
    const h = vi.fn();
    const { unmount } = renderHook(() =>
      useWindowEventMap({ "x-one": h, "x-two": h }),
    );
    unmount();
    window.dispatchEvent(new Event("x-one"));
    window.dispatchEvent(new Event("x-two"));
    expect(h).not.toHaveBeenCalled();
  });

  it("re-subscribes when the set of names changes", () => {
    const h = vi.fn();
    const { rerender } = renderHook(
      ({ name }) => useWindowEventMap({ [name]: h }),
      { initialProps: { name: "x-old" } },
    );
    rerender({ name: "x-new" });
    window.dispatchEvent(new Event("x-old"));
    expect(h).not.toHaveBeenCalled();
    window.dispatchEvent(new Event("x-new"));
    expect(h).toHaveBeenCalledTimes(1);
  });

  it("handles an empty table", () => {
    const { unmount } = renderHook(() => useWindowEventMap({}));
    expect(() => unmount()).not.toThrow();
  });
});
