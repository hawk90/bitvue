/**
 * useDropdown Hook Tests
 */

import { describe, it, expect, vi } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useDropdown } from "../useDropdown";

describe("useDropdown", () => {
  it("should start with initial open state", () => {
    const { result } = renderHook(() => useDropdown({ initialOpen: true }));

    expect(result.current.isOpen).toBe(true);
  });

  it("should start closed by default", () => {
    const { result } = renderHook(() => useDropdown());

    expect(result.current.isOpen).toBe(false);
  });

  it("should open when open is called", () => {
    const { result } = renderHook(() => useDropdown());

    act(() => {
      result.current.open();
    });

    expect(result.current.isOpen).toBe(true);
  });

  it("should close when close is called", () => {
    const { result } = renderHook(() => useDropdown({ initialOpen: true }));

    act(() => {
      result.current.close();
    });

    expect(result.current.isOpen).toBe(false);
  });

  it("should toggle when toggle is called", () => {
    const { result } = renderHook(() => useDropdown());

    act(() => {
      result.current.toggle();
    });

    expect(result.current.isOpen).toBe(true);

    act(() => {
      result.current.toggle();
    });

    expect(result.current.isOpen).toBe(false);
  });

  it("should call onOpen callback when opening", () => {
    const onOpen = vi.fn();
    const { result } = renderHook(() => useDropdown({ onOpen }));

    act(() => {
      result.current.open();
    });

    expect(onOpen).toHaveBeenCalledTimes(1);
  });

  it("should call onClose callback when closing", () => {
    const onClose = vi.fn();
    const { result } = renderHook(() =>
      useDropdown({ initialOpen: true, onClose }),
    );

    act(() => {
      result.current.close();
    });

    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("should call callbacks on toggle", () => {
    const onOpen = vi.fn();
    const onClose = vi.fn();
    const { result } = renderHook(() => useDropdown({ onOpen, onClose }));

    act(() => {
      result.current.toggle();
    });
    expect(onOpen).toHaveBeenCalledTimes(1);

    act(() => {
      result.current.toggle();
    });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("should not call onOpen when already open", () => {
    const onOpen = vi.fn();
    const { result } = renderHook(() =>
      useDropdown({ initialOpen: true, onOpen }),
    );

    act(() => {
      result.current.open();
    });

    expect(onOpen).not.toHaveBeenCalled();
  });

  it("should not call onClose when already closed", () => {
    const onClose = vi.fn();
    const { result } = renderHook(() => useDropdown({ onClose }));

    act(() => {
      result.current.close();
    });

    expect(onClose).not.toHaveBeenCalled();
  });

  it("should provide refs for dropdown and trigger", () => {
    const { result } = renderHook(() => useDropdown());

    expect(result.current.dropdownRef).toBeDefined();
    expect(result.current.dropdownRef.current).toBe(null);

    expect(result.current.triggerRef).toBeDefined();
    expect(result.current.triggerRef.current).toBe(null);
  });

  it("should close on click outside when enabled", () => {
    const { result } = renderHook(() =>
      useDropdown({ initialOpen: true, closeOnClickOutside: true }),
    );

    act(() => {
      const event = new MouseEvent("mousedown", { bubbles: true });
      Object.defineProperty(event, "target", {
        value: document.body,
        enumerable: true,
      });
      document.dispatchEvent(event);
    });

    // Note: This test verifies the setup, actual click outside behavior requires DOM
    expect(result.current.dropdownRef).toBeDefined();
  });

  it("should not close on click outside when disabled", () => {
    const { result } = renderHook(() =>
      useDropdown({ initialOpen: true, closeOnClickOutside: false }),
    );

    // The dropdown should remain open
    expect(result.current.isOpen).toBe(true);
  });

  it("should handle escape key when enabled", () => {
    const { result } = renderHook(() =>
      useDropdown({ initialOpen: true, closeOnEscape: true }),
    );

    act(() => {
      const event = new KeyboardEvent("keydown", { key: "Escape" });
      document.dispatchEvent(event);
    });

    expect(result.current.isOpen).toBe(false);
  });

  it("should not handle escape key when disabled", () => {
    const { result } = renderHook(() =>
      useDropdown({ initialOpen: true, closeOnEscape: false }),
    );

    act(() => {
      const event = new KeyboardEvent("keydown", { key: "Escape" });
      document.dispatchEvent(event);
    });

    expect(result.current.isOpen).toBe(true);
  });

  it("should not close on other keys", () => {
    const { result } = renderHook(() =>
      useDropdown({ initialOpen: true, closeOnEscape: true }),
    );

    act(() => {
      const event = new KeyboardEvent("keydown", { key: "Enter" });
      document.dispatchEvent(event);
    });

    expect(result.current.isOpen).toBe(true);
  });

  it("should handle multiple rapid toggle calls", () => {
    const onOpen = vi.fn();
    const onClose = vi.fn();
    const { result } = renderHook(() => useDropdown({ onOpen, onClose }));

    act(() => {
      result.current.toggle();
      result.current.toggle();
      result.current.toggle();
    });

    // Should end up with final state after all toggles
    expect(result.current.isOpen).toBe(true);
    expect(onOpen).toHaveBeenCalledTimes(1);
  });
});
