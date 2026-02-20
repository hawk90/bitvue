/**
 * useCanvasInteraction Hook Tests
 * Tests canvas zoom, pan, and drag interactions
 */

import { describe, it, expect, vi } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useCanvasInteraction } from "@/hooks/useCanvasInteraction";

describe("useCanvasInteraction", () => {
  it("should return initial state", () => {
    const { result } = renderHook(() => useCanvasInteraction());

    expect(result.current.zoom).toBe(1);
    expect(result.current.pan).toEqual({ x: 0, y: 0 });
    expect(result.current.isDragging).toBe(false);
  });

  it("should zoom in", () => {
    const { result } = renderHook(() => useCanvasInteraction());

    act(() => {
      result.current.zoomIn();
    });

    expect(result.current.zoom).toBe(1.25);
  });

  it("should zoom out", () => {
    const { result } = renderHook(() => useCanvasInteraction());

    act(() => {
      result.current.zoomOut();
    });

    expect(result.current.zoom).toBe(0.75);
  });

  it("should respect min zoom limit", () => {
    const { result } = renderHook(() => useCanvasInteraction({ minZoom: 0.5 }));

    act(() => {
      result.current.setZoom(0.5);
      result.current.zoomOut();
    });

    expect(result.current.zoom).toBe(0.5);
  });

  it("should respect max zoom limit", () => {
    const { result } = renderHook(() => useCanvasInteraction({ maxZoom: 2 }));

    act(() => {
      result.current.setZoom(2);
      result.current.zoomIn();
    });

    expect(result.current.zoom).toBe(2);
  });

  it("should reset zoom", () => {
    const { result } = renderHook(() => useCanvasInteraction());

    act(() => {
      result.current.setZoom(2);
      result.current.setPan({ x: 50, y: 50 });
    });

    expect(result.current.zoom).toBe(2);
    expect(result.current.pan).toEqual({ x: 50, y: 50 });

    act(() => {
      result.current.resetZoom();
    });

    expect(result.current.zoom).toBe(1);
    expect(result.current.pan).toEqual({ x: 0, y: 0 });
  });

  it("should set zoom to specific value", () => {
    const { result } = renderHook(() => useCanvasInteraction());

    act(() => {
      result.current.setZoom(1.5);
    });

    expect(result.current.zoom).toBe(1.5);
  });

  it("should set pan to specific coordinates", () => {
    const { result } = renderHook(() => useCanvasInteraction());

    act(() => {
      result.current.setPan({ x: 100, y: 200 });
    });

    expect(result.current.pan).toEqual({ x: 100, y: 200 });
  });

  it("should use custom zoom step", () => {
    const { result } = renderHook(() =>
      useCanvasInteraction({ zoomStep: 0.5 }),
    );

    act(() => {
      result.current.zoomIn();
    });

    expect(result.current.zoom).toBe(1.5);
  });

  it("should use initial zoom value", () => {
    const { result } = renderHook(() =>
      useCanvasInteraction({ initialZoom: 2 }),
    );

    expect(result.current.zoom).toBe(2);
  });

  it("should use initial pan value", () => {
    const { result } = renderHook(() =>
      useCanvasInteraction({
        initialPan: { x: 50, y: 100 },
      }),
    );

    expect(result.current.pan).toEqual({ x: 50, y: 100 });
  });
});

describe("useCanvasInteraction handlers", () => {
  it("should provide onWheel handler", () => {
    const { result } = renderHook(() => useCanvasInteraction());

    expect(typeof result.current.handlers.onWheel).toBe("function");
  });

  it("should provide onMouseDown handler", () => {
    const { result } = renderHook(() => useCanvasInteraction());

    expect(typeof result.current.handlers.onMouseDown).toBe("function");
  });

  it("should provide onMouseMove handler", () => {
    const { result } = renderHook(() => useCanvasInteraction());

    expect(typeof result.current.handlers.onMouseMove).toBe("function");
  });

  it("should provide onMouseUp handler", () => {
    const { result } = renderHook(() => useCanvasInteraction());

    expect(typeof result.current.handlers.onMouseUp).toBe("function");
  });

  it("should zoom on wheel with Ctrl key when requireModifierKey is true", () => {
    const { result } = renderHook(() =>
      useCanvasInteraction({ requireModifierKey: true }),
    );

    act(() => {
      const wheelEvent = new WheelEvent("wheel", {
        deltaY: -100,
        ctrlKey: true,
      });
      result.current.handlers.onWheel(wheelEvent);
    });

    expect(result.current.zoom).toBeGreaterThan(1);
  });

  it("should not zoom on wheel without Ctrl key when requireModifierKey is true", () => {
    const { result } = renderHook(() =>
      useCanvasInteraction({ requireModifierKey: true }),
    );

    const initialZoom = result.current.zoom;

    act(() => {
      const wheelEvent = new WheelEvent("wheel", {
        deltaY: -100,
        ctrlKey: false,
      });
      result.current.handlers.onWheel(wheelEvent);
    });

    expect(result.current.zoom).toBe(initialZoom);
  });

  it("should zoom on any wheel when requireModifierKey is false", () => {
    const { result } = renderHook(() =>
      useCanvasInteraction({ requireModifierKey: false }),
    );

    act(() => {
      const wheelEvent = new WheelEvent("wheel", {
        deltaY: -100,
        ctrlKey: false,
      });
      result.current.handlers.onWheel(wheelEvent);
    });

    expect(result.current.zoom).not.toBe(1);
  });

  it("should start drag on mouse down", () => {
    const { result } = renderHook(() => useCanvasInteraction());

    act(() => {
      const mouseEvent = new MouseEvent("mousedown", {
        clientX: 100,
        clientY: 100,
      });
      result.current.handlers.onMouseDown(mouseEvent);
    });

    expect(result.current.isDragging).toBe(true);
  });

  it("should update pan on mouse move while dragging", () => {
    const { result } = renderHook(() => useCanvasInteraction());

    act(() => {
      const downEvent = new MouseEvent("mousedown", {
        clientX: 100,
        clientY: 100,
      });
      result.current.handlers.onMouseDown(downEvent);
    });

    act(() => {
      const moveEvent = new MouseEvent("mousemove", {
        clientX: 150,
        clientY: 150,
      });
      result.current.handlers.onMouseMove(moveEvent);
    });

    expect(result.current.pan.x).not.toBe(0);
    expect(result.current.pan.y).not.toBe(0);
  });

  it("should end drag on mouse up", () => {
    const { result } = renderHook(() => useCanvasInteraction());

    act(() => {
      const downEvent = new MouseEvent("mousedown", {
        clientX: 100,
        clientY: 100,
      });
      result.current.handlers.onMouseDown(downEvent);
    });

    expect(result.current.isDragging).toBe(true);

    act(() => {
      const upEvent = new MouseEvent("mouseup", { clientX: 150, clientY: 150 });
      result.current.handlers.onMouseUp(upEvent);
    });

    expect(result.current.isDragging).toBe(false);
  });
});

describe("useCanvasInteraction edge cases", () => {
  it("should handle negative pan values", () => {
    const { result } = renderHook(() => useCanvasInteraction());

    act(() => {
      result.current.setPan({ x: -100, y: -200 });
    });

    expect(result.current.pan).toEqual({ x: -100, y: -200 });
  });

  it("should handle zero zoom", () => {
    const { result } = renderHook(() => useCanvasInteraction({ minZoom: 0 }));

    act(() => {
      result.current.setZoom(0.5);
      result.current.zoomOut();
      result.current.zoomOut();
    });

    // Should clamp to minZoom
    expect(result.current.zoom).toBe(0);
  });

  it("should handle very large zoom values", () => {
    const { result } = renderHook(() => useCanvasInteraction({ maxZoom: 10 }));

    act(() => {
      result.current.setZoom(8);
    });

    expect(result.current.zoom).toBe(8);
  });
});
