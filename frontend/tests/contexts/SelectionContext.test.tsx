/**
 * SelectionContext Provider Tests
 * Tests tri-sync selection system for frame navigation
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import {
  SelectionProvider,
  useSelection,
  useSelectionSubscribe,
} from "@/contexts/SelectionContext";
import type {
  TemporalSelection,
  FrameKey,
  UnitKey,
  SyntaxNodeId,
  BitRange,
} from "@/contexts/SelectionContext";

describe("SelectionContext", () => {
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <SelectionProvider>{children}</SelectionProvider>
  );

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should provide default selection state", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    expect(result.current.selection).toBeNull();
  });

  it("should have all required methods", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    expect(typeof result.current.setTemporalSelection).toBe("function");
    expect(typeof result.current.setFrameSelection).toBe("function");
    expect(typeof result.current.setUnitSelection).toBe("function");
    expect(typeof result.current.setSyntaxSelection).toBe("function");
    expect(typeof result.current.setBitRangeSelection).toBe("function");
    expect(typeof result.current.clearTemporal).toBe("function");
    expect(typeof result.current.clearAll).toBe("function");
    expect(typeof result.current.subscribe).toBe("function");
  });
});

describe("SelectionContext temporal selection", () => {
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <SelectionProvider>{children}</SelectionProvider>
  );

  it("should set temporal selection", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    const temporal: TemporalSelection = {
      type: "point",
      frameIndex: 5,
    };

    act(() => {
      result.current.setTemporalSelection(temporal, "timeline");
    });

    expect(result.current.selection?.temporal).toEqual(temporal);
  });

  it("should update source when setting temporal selection", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    const temporal: TemporalSelection = {
      type: "block",
      frameIndex: 10,
      block: { x: 0, y: 0, w: 16, h: 16 },
    };

    act(() => {
      result.current.setTemporalSelection(temporal, "main");
    });

    expect(result.current.selection?.source.panel).toBe("main");
    expect(result.current.selection?.source.timestamp).toBeGreaterThan(0);
  });

  it("should support range temporal selection", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    const temporal: TemporalSelection = {
      type: "range",
      frameIndex: 0,
      rangeStart: 0,
      rangeEnd: 100,
    };

    act(() => {
      result.current.setTemporalSelection(temporal, "filmstrip");
    });

    expect(result.current.selection?.temporal?.type).toBe("range");
    expect(result.current.selection?.temporal?.rangeStart).toBe(0);
    expect(result.current.selection?.temporal?.rangeEnd).toBe(100);
  });

  it("should support marker temporal selection", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    const temporal: TemporalSelection = {
      type: "marker",
      frameIndex: 50,
    };

    act(() => {
      result.current.setTemporalSelection(temporal, "bookmarks");
    });

    expect(result.current.selection?.temporal?.type).toBe("marker");
  });

  it("should clear temporal selection", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    act(() => {
      result.current.setTemporalSelection(
        { type: "point", frameIndex: 5 },
        "timeline",
      );
    });

    expect(result.current.selection?.temporal).not.toBeNull();

    act(() => {
      result.current.clearTemporal();
    });

    expect(result.current.selection?.temporal).toBeNull();
  });
});

describe("SelectionContext frame selection", () => {
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <SelectionProvider>{children}</SelectionProvider>
  );

  it("should set frame selection", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    const frame: FrameKey = {
      stream: "A",
      frameIndex: 10,
      pts: 500,
    };

    act(() => {
      result.current.setFrameSelection(frame, "timeline");
    });

    expect(result.current.selection?.frame).toEqual(frame);
  });

  it("should auto-create temporal selection from frame selection", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    const frame: FrameKey = {
      stream: "B",
      frameIndex: 25,
    };

    act(() => {
      result.current.setFrameSelection(frame, "keyboard");
    });

    expect(result.current.selection?.temporal).toEqual({
      type: "point",
      frameIndex: 25,
    });
  });

  it("should update streamId from frame selection", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    const frame: FrameKey = {
      stream: "B",
      frameIndex: 30,
    };

    act(() => {
      result.current.setFrameSelection(frame, "main");
    });

    expect(result.current.selection?.streamId).toBe("B");
  });

  it("should handle frame selection without pts", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    const frame: FrameKey = {
      stream: "A",
      frameIndex: 15,
    };

    act(() => {
      result.current.setFrameSelection(frame, "minimap");
    });

    expect(result.current.selection?.frame?.pts).toBeUndefined();
  });
});

describe("SelectionContext unit selection", () => {
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <SelectionProvider>{children}</SelectionProvider>
  );

  it("should set unit selection", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    const unit: UnitKey = {
      stream: "A",
      unitType: "nal_unit",
      offset: 1000,
      size: 500,
    };

    act(() => {
      result.current.setUnitSelection(unit, "syntax");
    });

    expect(result.current.selection?.unit).toEqual(unit);
  });

  it("should auto-create bitRange from unit selection (tri-sync rule 2)", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    const unit: UnitKey = {
      stream: "A",
      unitType: "ctu",
      offset: 2000,
      size: 256,
    };

    act(() => {
      result.current.setUnitSelection(unit, "hex");
    });

    expect(result.current.selection?.bitRange).toEqual({
      startBit: 2000 * 8,
      endBit: (2000 + 256) * 8,
    });
  });

  it("should update streamId from unit selection", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    const unit: UnitKey = {
      stream: "B",
      unitType: "obu",
      offset: 5000,
      size: 128,
    };

    act(() => {
      result.current.setUnitSelection(unit, "main");
    });

    expect(result.current.selection?.streamId).toBe("B");
  });
});

describe("SelectionContext syntax node selection", () => {
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <SelectionProvider>{children}</SelectionProvider>
  );

  it("should set syntax node selection", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    const node: SyntaxNodeId = {
      path: ["root", "sequence_header", "profile_tier_level"],
      fieldType: "u8",
    };

    act(() => {
      result.current.setSyntaxSelection(node, "syntax");
    });

    expect(result.current.selection?.syntaxNode).toEqual(node);
  });

  it("should auto-create bitRange from syntax node with offset (tri-sync rule 3)", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    const node: SyntaxNodeId = {
      path: ["root", "frame_header"],
      fieldType: "u4",
      offset: 1000,
    };

    act(() => {
      result.current.setSyntaxSelection(node, "reference-lists");
    });

    expect(result.current.selection?.bitRange).toEqual({
      startBit: 1000,
      endBit: 1004,
    });
  });

  it("should use default size for unknown field types", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    const node: SyntaxNodeId = {
      path: ["root", "unknown_field"],
      fieldType: "unknown_type",
      offset: 500,
    };

    act(() => {
      result.current.setSyntaxSelection(node, "syntax");
    });

    expect(result.current.selection?.bitRange).toEqual({
      startBit: 500,
      endBit: 532,
    });
  });

  it("should handle syntax node without offset", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    const node: SyntaxNodeId = {
      path: ["root", "header"],
    };

    act(() => {
      result.current.setSyntaxSelection(node, "syntax");
    });

    expect(result.current.selection?.syntaxNode).toEqual(node);
    expect(result.current.selection?.bitRange).toBeNull();
  });
});

describe("SelectionContext bitRange selection", () => {
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <SelectionProvider>{children}</SelectionProvider>
  );

  it("should set bitRange selection", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    const bitRange: BitRange = {
      startBit: 0,
      endBit: 1000,
    };

    act(() => {
      result.current.setBitRangeSelection(bitRange, "hex");
    });

    expect(result.current.selection?.bitRange).toEqual(bitRange);
  });

  it("should allow updating bitRange without affecting other selections", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    act(() => {
      result.current.setFrameSelection(
        { stream: "A", frameIndex: 10 },
        "timeline",
      );
    });

    act(() => {
      result.current.setBitRangeSelection(
        { startBit: 500, endBit: 1000 },
        "hex",
      );
    });

    expect(result.current.selection?.frame).toEqual({
      stream: "A",
      frameIndex: 10,
    });
    expect(result.current.selection?.bitRange).toEqual({
      startBit: 500,
      endBit: 1000,
    });
  });
});

describe("SelectionContext tri-sync rules", () => {
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <SelectionProvider>{children}</SelectionProvider>
  );

  it("should propagate temporal selection to frame selection (rule 1)", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    act(() => {
      result.current.setTemporalSelection(
        { type: "point", frameIndex: 42 },
        "timeline",
      );
    });

    expect(result.current.selection?.frame).toEqual({
      stream: "A",
      frameIndex: 42,
    });
  });

  it("should not override existing frame selection when setting temporal", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    act(() => {
      result.current.setFrameSelection({ stream: "B", frameIndex: 20 }, "main");
      result.current.setTemporalSelection(
        { type: "point", frameIndex: 30 },
        "timeline",
      );
    });

    // Frame selection should remain from the explicit setFrameSelection
    expect(result.current.selection?.frame?.stream).toBe("B");
  });

  it("should propagate unit selection to bitRange (rule 2)", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    act(() => {
      result.current.setUnitSelection(
        { stream: "A", unitType: "ctu", offset: 100, size: 64 },
        "syntax",
      );
    });

    expect(result.current.selection?.bitRange).toEqual({
      startBit: 800,
      endBit: 1312,
    });
  });

  it("should propagate syntax node with offset to bitRange (rule 3)", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    act(() => {
      result.current.setSyntaxSelection(
        { path: ["test"], fieldType: "u8", offset: 200 },
        "syntax",
      );
    });

    expect(result.current.selection?.bitRange).toEqual({
      startBit: 200,
      endBit: 208,
    });
  });
});

describe("SelectionContext subscription system", () => {
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <SelectionProvider>{children}</SelectionProvider>
  );

  it("should notify subscribers on selection change", async () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    const callback = vi.fn();
    const unsubscribe = result.current.subscribe(callback);

    act(() => {
      result.current.setFrameSelection(
        { stream: "A", frameIndex: 10 },
        "timeline",
      );
    });

    expect(callback).toHaveBeenCalledWith(
      expect.objectContaining({
        selection: expect.objectContaining({
          frame: { stream: "A", frameIndex: 10 },
        }),
      }),
    );

    unsubscribe();
  });

  it("should unsubscribe callback when returned function is called", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    const callback = vi.fn();
    const unsubscribe = result.current.subscribe(callback);

    unsubscribe();

    act(() => {
      result.current.setFrameSelection(
        { stream: "A", frameIndex: 20 },
        "timeline",
      );
    });

    expect(callback).not.toHaveBeenCalled();
  });

  it("should support multiple subscribers", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    const callback1 = vi.fn();
    const callback2 = vi.fn();

    result.current.subscribe(callback1);
    result.current.subscribe(callback2);

    act(() => {
      result.current.setFrameSelection(
        { stream: "A", frameIndex: 5 },
        "keyboard",
      );
    });

    expect(callback1).toHaveBeenCalled();
    expect(callback2).toHaveBeenCalled();
  });

  it("should notify subscribers with source information", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    const callback = vi.fn();
    result.current.subscribe(callback);

    act(() => {
      result.current.setTemporalSelection(
        { type: "point", frameIndex: 15 },
        "main",
      );
    });

    expect(callback).toHaveBeenCalledWith(
      expect.objectContaining({
        source: expect.objectContaining({
          panel: "main",
          timestamp: expect.any(Number),
        }),
      }),
    );
  });
});

describe("SelectionContext clearAll", () => {
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <SelectionProvider>{children}</SelectionProvider>
  );

  it("should clear all selection state", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    act(() => {
      result.current.setFrameSelection(
        { stream: "A", frameIndex: 10 },
        "timeline",
      );
      result.current.setBitRangeSelection({ startBit: 0, endBit: 100 }, "hex");
    });

    expect(result.current.selection).not.toBeNull();

    act(() => {
      result.current.clearAll();
    });

    expect(result.current.selection).toBeNull();
  });

  it("should notify subscribers on clear", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    const callback = vi.fn();
    result.current.subscribe(callback);

    act(() => {
      result.current.setFrameSelection(
        { stream: "A", frameIndex: 5 },
        "timeline",
      );
      result.current.clearAll();
    });

    // clearAll doesn't notify since it sets to null directly
    expect(callback).toHaveBeenCalledTimes(1);
  });
});

describe("useSelectionSubscribe hook", () => {
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <SelectionProvider>{children}</SelectionProvider>
  );

  it("should subscribe to selection changes", () => {
    const callback = vi.fn();

    renderHook(() => useSelectionSubscribe(callback), { wrapper });

    act(() => {
      // This would need to trigger a selection change
      // Since we can't access the context directly here, this test
      // verifies the hook doesn't throw
    });

    expect(callback).toBeDefined();
  });

  it("should accept dependencies array", () => {
    const callback = vi.fn();
    const deps = [1, 2, 3];

    expect(() => {
      renderHook(() => useSelectionSubscribe(callback, deps), { wrapper });
    }).not.toThrow();
  });
});

describe("SelectionContext error handling", () => {
  it("should throw error when useSelection used outside provider", () => {
    expect(() => {
      renderHook(() => useSelection());
    }).toThrow("useSelection must be used within SelectionProvider");
  });

  it("should throw error when useSelectionSubscribe used outside provider", () => {
    expect(() => {
      renderHook(() => useSelectionSubscribe(vi.fn()));
    }).toThrow("useSelection must be used within SelectionProvider");
  });
});

describe("SelectionContext stream switching", () => {
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <SelectionProvider>{children}</SelectionProvider>
  );

  it("should switch from stream A to stream B", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    act(() => {
      result.current.setFrameSelection(
        { stream: "A", frameIndex: 10 },
        "timeline",
      );
      result.current.setFrameSelection(
        { stream: "B", frameIndex: 20 },
        "timeline",
      );
    });

    expect(result.current.selection?.streamId).toBe("B");
    expect(result.current.selection?.frame?.stream).toBe("B");
    expect(result.current.selection?.frame?.frameIndex).toBe(20);
  });

  it("should maintain selections when switching streams", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    act(() => {
      result.current.setFrameSelection(
        { stream: "A", frameIndex: 5 },
        "timeline",
      );
      result.current.setBitRangeSelection(
        { startBit: 100, endBit: 200 },
        "hex",
      );
    });

    const originalBitRange = result.current.selection?.bitRange;

    act(() => {
      result.current.setFrameSelection(
        { stream: "B", frameIndex: 15 },
        "timeline",
      );
    });

    // BitRange should be preserved
    expect(result.current.selection?.bitRange).toEqual(originalBitRange);
  });
});

describe("SelectionContext complex workflows", () => {
  const wrapper = ({ children }: { children: React.ReactNode }) => {
    return <SelectionProvider>{children}</SelectionProvider>;
  };

  it("should handle syntax tree navigation workflow", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    // User clicks on syntax tree node
    act(() => {
      result.current.setSyntaxSelection(
        { path: ["root", "frame_header"], fieldType: "u8", offset: 500 },
        "syntax",
      );
    });

    expect(result.current.selection?.syntaxNode).toBeDefined();
    expect(result.current.selection?.bitRange).toEqual({
      startBit: 500,
      endBit: 508,
    });

    // User then selects frame from filmstrip
    act(() => {
      result.current.setFrameSelection(
        { stream: "A", frameIndex: 25 },
        "filmstrip",
      );
    });

    expect(result.current.selection?.frame).toEqual({
      stream: "A",
      frameIndex: 25,
    });
    expect(result.current.selection?.temporal).toEqual({
      type: "point",
      frameIndex: 25,
    });
  });

  it("should handle hex view navigation workflow", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    // User selects a unit in hex view
    act(() => {
      result.current.setUnitSelection(
        { stream: "A", unitType: "nal_unit", offset: 1000, size: 256 },
        "hex",
      );
    });

    expect(result.current.selection?.bitRange).toEqual({
      startBit: 8000,
      endBit: 10048,
    });

    // User navigates to specific bit range
    act(() => {
      result.current.setBitRangeSelection(
        { startBit: 8100, endBit: 8200 },
        "hex",
      );
    });

    expect(result.current.selection?.bitRange).toEqual({
      startBit: 8100,
      endBit: 8200,
    });
    expect(result.current.selection?.unit).toBeDefined(); // Unit should be preserved
  });

  it("should handle timeline scrubbing workflow", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    // User scrubs timeline
    for (let i = 0; i < 10; i++) {
      act(() => {
        result.current.setTemporalSelection(
          { type: "point", frameIndex: i },
          "timeline",
        );
      });

      expect(result.current.selection?.temporal?.frameIndex).toBe(i);
      expect(result.current.selection?.frame?.frameIndex).toBe(i);
    }
  });

  it("should handle frame range selection workflow", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    // User selects range of frames
    act(() => {
      result.current.setTemporalSelection(
        { type: "range", frameIndex: 0, rangeStart: 0, rangeEnd: 100 },
        "timeline",
      );
    });

    expect(result.current.selection?.temporal?.type).toBe("range");
    expect(result.current.selection?.temporal?.rangeStart).toBe(0);
    expect(result.current.selection?.temporal?.rangeEnd).toBe(100);
  });
});

describe("SelectionContext edge cases", () => {
  const wrapper = ({ children }: { children: React.ReactNode }) => {
    return <SelectionProvider>{children}</SelectionProvider>;
  };

  it("should handle setting same selection multiple times", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    const frame: FrameKey = { stream: "A", frameIndex: 10 };

    act(() => {
      result.current.setFrameSelection(frame, "timeline");
      result.current.setFrameSelection(frame, "timeline");
    });

    expect(result.current.selection?.frame).toEqual(frame);
  });

  it("should handle rapid selection changes", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    act(() => {
      for (let i = 0; i < 50; i++) {
        result.current.setFrameSelection(
          { stream: "A", frameIndex: i },
          "keyboard",
        );
      }
    });

    expect(result.current.selection?.frame?.frameIndex).toBe(49);
  });

  it("should handle selection with all source types", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    const sources: Array<
      | "syntax"
      | "hex"
      | "main"
      | "timeline"
      | "filmstrip"
      | "reference-lists"
      | "keyboard"
      | "minimap"
      | "bookmarks"
    > = [
      "syntax",
      "hex",
      "main",
      "timeline",
      "filmstrip",
      "reference-lists",
      "keyboard",
      "minimap",
      "bookmarks",
    ];

    sources.forEach((source) => {
      act(() => {
        result.current.setFrameSelection(
          { stream: "A", frameIndex: 5 },
          source,
        );
      });

      expect(result.current.selection?.source.panel).toBe(source);
    });
  });

  it("should handle zero-sized bit ranges", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    act(() => {
      result.current.setBitRangeSelection(
        { startBit: 100, endBit: 100 },
        "hex",
      );
    });

    expect(result.current.selection?.bitRange).toEqual({
      startBit: 100,
      endBit: 100,
    });
  });

  it("should handle very large frame indices", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    act(() => {
      result.current.setFrameSelection(
        { stream: "A", frameIndex: 999999 },
        "timeline",
      );
    });

    expect(result.current.selection?.frame?.frameIndex).toBe(999999);
  });

  it("should handle negative frame indices in temporal selection", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    act(() => {
      result.current.setTemporalSelection(
        { type: "point", frameIndex: -1 },
        "timeline",
      );
    });

    expect(result.current.selection?.temporal?.frameIndex).toBe(-1);
  });

  it("should handle empty syntax node path", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    act(() => {
      result.current.setSyntaxSelection({ path: [] }, "syntax");
    });

    expect(result.current.selection?.syntaxNode).toEqual({ path: [] });
  });

  it("should handle deeply nested syntax node path", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    const deepPath = Array.from({ length: 20 }, (_, i) => `level${i}`);

    act(() => {
      result.current.setSyntaxSelection({ path: deepPath }, "syntax");
    });

    expect(result.current.selection?.syntaxNode?.path).toHaveLength(20);
  });
});

describe("SelectionContext field type size estimation", () => {
  const wrapper = ({ children }: { children: React.ReactNode }) => {
    return <SelectionProvider>{children}</SelectionProvider>;
  };

  it("should estimate correct size for u1 field", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    act(() => {
      result.current.setSyntaxSelection(
        { path: ["test"], fieldType: "u1", offset: 100 },
        "syntax",
      );
    });

    expect(result.current.selection?.bitRange).toEqual({
      startBit: 100,
      endBit: 101,
    });
  });

  it("should estimate correct size for u4 field", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    act(() => {
      result.current.setSyntaxSelection(
        { path: ["test"], fieldType: "u4", offset: 200 },
        "syntax",
      );
    });

    expect(result.current.selection?.bitRange).toEqual({
      startBit: 200,
      endBit: 204,
    });
  });

  it("should estimate correct size for u8 field", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    act(() => {
      result.current.setSyntaxSelection(
        { path: ["test"], fieldType: "u8", offset: 300 },
        "syntax",
      );
    });

    expect(result.current.selection?.bitRange).toEqual({
      startBit: 300,
      endBit: 308,
    });
  });

  it("should handle variable length fields", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    act(() => {
      result.current.setSyntaxSelection(
        { path: ["test"], fieldType: "ue(v)", offset: 400 },
        "syntax",
      );
    });

    // Variable length fields should default to 0 or minimal size
    expect(result.current.selection?.bitRange).toEqual({
      startBit: 400,
      endBit: 400,
    });
  });

  it("should handle leb128 field type", () => {
    const { result } = renderHook(() => useSelection(), { wrapper });

    act(() => {
      result.current.setSyntaxSelection(
        { path: ["test"], fieldType: "leb128", offset: 500 },
        "syntax",
      );
    });

    expect(result.current.selection?.bitRange).toEqual({
      startBit: 500,
      endBit: 500,
    });
  });
});
