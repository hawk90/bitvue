/**
 * LayoutContext Provider Tests
 * Tests layout context provider for panel management
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import {
  LayoutProvider,
  useLayout,
  DEFAULT_LAYOUT,
} from "@/contexts/LayoutContext";

// Mock localStorage
const localStorageMock = (() => {
  let store: Record<string, string> = {};
  return {
    getItem: (key: string) => store[key] || null,
    setItem: (key: string, value: string) => {
      store[key] = value.toString();
    },
    removeItem: (key: string) => {
      delete store[key];
    },
    clear: () => {
      store = {};
    },
  };
})();

Object.defineProperty(global, "localStorage", {
  value: localStorageMock,
});

describe("LayoutContext", () => {
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <LayoutProvider>{children}</LayoutProvider>
  );

  beforeEach(() => {
    localStorageMock.clear();
  });

  it("should provide default layout state", () => {
    const { result } = renderHook(() => useLayout(), { wrapper });

    expect(result.current.layoutState).toBeDefined();
    expect(result.current.layoutState.leftPanelSize).toBe(
      DEFAULT_LAYOUT.leftPanelSize,
    );
    expect(result.current.layoutState.topPanelSize).toBe(
      DEFAULT_LAYOUT.topPanelSize,
    );
    expect(result.current.layoutState.bottomPanelSizes).toEqual(
      DEFAULT_LAYOUT.bottomPanelSizes,
    );
  });

  it("should have resetLayout function", () => {
    const { result } = renderHook(() => useLayout(), { wrapper });

    expect(typeof result.current.resetLayout).toBe("function");
  });

  it("should have saveLayout function", () => {
    const { result } = renderHook(() => useLayout(), { wrapper });

    expect(typeof result.current.saveLayout).toBe("function");
  });

  it("should have loadLayout function", () => {
    const { result } = renderHook(() => useLayout(), { wrapper });

    expect(typeof result.current.loadLayout).toBe("function");
  });

  it("should have updateLeftPanel function", () => {
    const { result } = renderHook(() => useLayout(), { wrapper });

    expect(typeof result.current.updateLeftPanel).toBe("function");
  });

  it("should have updateTopPanel function", () => {
    const { result } = renderHook(() => useLayout(), { wrapper });

    expect(typeof result.current.updateTopPanel).toBe("function");
  });

  it("should have updateBottomPanel function", () => {
    const { result } = renderHook(() => useLayout(), { wrapper });

    expect(typeof result.current.updateBottomPanel).toBe("function");
  });
});

describe("LayoutContext panel management", () => {
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <LayoutProvider>{children}</LayoutProvider>
  );

  beforeEach(() => {
    localStorageMock.clear();
  });

  it("should update left panel size", () => {
    const { result } = renderHook(() => useLayout(), { wrapper });

    act(() => {
      result.current.updateLeftPanel(50);
    });

    expect(result.current.layoutState.leftPanelSize).toBe(50);
  });

  it("should update top panel size", () => {
    const { result } = renderHook(() => useLayout(), { wrapper });

    act(() => {
      result.current.updateTopPanel(30);
    });

    expect(result.current.layoutState.topPanelSize).toBe(30);
  });

  it("should update bottom panel size at specific index", () => {
    const { result } = renderHook(() => useLayout(), { wrapper });

    act(() => {
      result.current.updateBottomPanel(0, 50);
    });

    expect(result.current.layoutState.bottomPanelSizes[0]).toBe(50);
  });

  it("should update multiple bottom panel sizes", () => {
    const { result } = renderHook(() => useLayout(), { wrapper });

    act(() => {
      result.current.updateBottomPanel(0, 40);
      result.current.updateBottomPanel(1, 30);
      result.current.updateBottomPanel(2, 30);
    });

    expect(result.current.layoutState.bottomPanelSizes).toEqual([40, 30, 30]);
  });
});

describe("LayoutContext resetLayout", () => {
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <LayoutProvider>{children}</LayoutProvider>
  );

  beforeEach(() => {
    localStorageMock.clear();
  });

  it("should reset to default layout", () => {
    const { result } = renderHook(() => useLayout(), { wrapper });

    act(() => {
      result.current.updateLeftPanel(60);
      result.current.updateTopPanel(40);
    });

    expect(result.current.layoutState.leftPanelSize).toBe(60);
    expect(result.current.layoutState.topPanelSize).toBe(40);

    act(() => {
      result.current.resetLayout();
    });

    expect(result.current.layoutState.leftPanelSize).toBe(
      DEFAULT_LAYOUT.leftPanelSize,
    );
    expect(result.current.layoutState.topPanelSize).toBe(
      DEFAULT_LAYOUT.topPanelSize,
    );
    expect(result.current.layoutState.bottomPanelSizes).toEqual(
      DEFAULT_LAYOUT.bottomPanelSizes,
    );
  });

  it("should clear localStorage on reset", () => {
    const { result } = renderHook(() => useLayout(), { wrapper });

    act(() => {
      result.current.updateLeftPanel(60);
      result.current.saveLayout();
    });

    expect(localStorageMock.getItem("bitvue-layout")).toBeDefined();

    act(() => {
      result.current.resetLayout();
    });

    expect(localStorageMock.getItem("bitvue-layout")).toBeNull();
  });
});

describe("LayoutContext persistence", () => {
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <LayoutProvider>{children}</LayoutProvider>
  );

  beforeEach(() => {
    localStorageMock.clear();
  });

  it("should save layout to localStorage", () => {
    const setItemSpy = vi.spyOn(localStorageMock, "setItem");

    const { result } = renderHook(() => useLayout(), { wrapper });

    act(() => {
      result.current.updateLeftPanel(55);
    });

    // State update should be complete
    expect(result.current.layoutState.leftPanelSize).toBe(55);

    act(() => {
      result.current.saveLayout();
    });

    expect(setItemSpy).toHaveBeenCalled();

    const saved = localStorageMock.getItem("bitvue-layout");
    expect(saved).toBeDefined();

    const parsed = JSON.parse(saved!);
    expect(parsed.leftPanelSize).toBe(55);

    setItemSpy.mockRestore();
  });

  it("should load layout from localStorage", () => {
    const savedLayout = {
      leftPanelSize: 70,
      topPanelSize: 25,
      bottomPanelSizes: [20, 30, 50],
    };

    localStorageMock.setItem("bitvue-layout", JSON.stringify(savedLayout));

    const { result } = renderHook(() => useLayout(), { wrapper });

    act(() => {
      result.current.loadLayout();
    });

    expect(result.current.layoutState.leftPanelSize).toBe(70);
    expect(result.current.layoutState.topPanelSize).toBe(25);
    expect(result.current.layoutState.bottomPanelSizes).toEqual([20, 30, 50]);
  });

  it("should not load layout in test environment on mount", () => {
    const { result } = renderHook(() => useLayout(), { wrapper });

    // Should have defaults, not anything from localStorage
    expect(result.current.layoutState.leftPanelSize).toBe(
      DEFAULT_LAYOUT.leftPanelSize,
    );
  });
});

describe("LayoutContext error handling", () => {
  it("should throw error when useLayout used outside provider", () => {
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});

    expect(() => {
      renderHook(() => useLayout());
    }).toThrow("useLayout must be used within LayoutProvider");

    consoleSpy.mockRestore();
  });

  it("should handle corrupted localStorage data gracefully", () => {
    const wrapper = ({ children }: { children: React.ReactNode }) => (
      <LayoutProvider>{children}</LayoutProvider>
    );

    localStorageMock.setItem("bitvue-layout", "invalid json");

    const { result } = renderHook(() => useLayout(), { wrapper });

    // Should have default layout after failed load
    expect(result.current.layoutState).toBeDefined();

    act(() => {
      result.current.loadLayout();
    });

    // Should still have valid layout state
    expect(result.current.layoutState).toBeDefined();
    expect(typeof result.current.layoutState.leftPanelSize).toBe("number");
  });

  it("should handle missing localStorage gracefully", () => {
    const wrapper = ({ children }: { children: React.ReactNode }) => (
      <LayoutProvider>{children}</LayoutProvider>
    );

    // localStorage is empty
    const { result } = renderHook(() => useLayout(), { wrapper });

    act(() => {
      result.current.loadLayout();
    });

    // Should have default layout
    expect(result.current.layoutState.leftPanelSize).toBe(
      DEFAULT_LAYOUT.leftPanelSize,
    );
  });
});

describe("LayoutContext edge cases", () => {
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <LayoutProvider>{children}</LayoutProvider>
  );

  beforeEach(() => {
    localStorageMock.clear();
  });

  it("should handle zero panel sizes", () => {
    const { result } = renderHook(() => useLayout(), { wrapper });

    act(() => {
      result.current.updateLeftPanel(0);
      result.current.updateTopPanel(0);
      result.current.updateBottomPanel(0, 0);
    });

    expect(result.current.layoutState.leftPanelSize).toBe(0);
    expect(result.current.layoutState.topPanelSize).toBe(0);
    expect(result.current.layoutState.bottomPanelSizes[0]).toBe(0);
  });

  it("should handle large panel sizes", () => {
    const { result } = renderHook(() => useLayout(), { wrapper });

    act(() => {
      result.current.updateLeftPanel(100);
      result.current.updateTopPanel(100);
      result.current.updateBottomPanel(0, 100);
    });

    expect(result.current.layoutState.leftPanelSize).toBe(100);
    expect(result.current.layoutState.topPanelSize).toBe(100);
    expect(result.current.layoutState.bottomPanelSizes[0]).toBe(100);
  });

  it("should handle rapid updates to same panel", () => {
    const { result } = renderHook(() => useLayout(), { wrapper });

    act(() => {
      result.current.updateLeftPanel(10);
      result.current.updateLeftPanel(20);
      result.current.updateLeftPanel(30);
      result.current.updateLeftPanel(40);
    });

    expect(result.current.layoutState.leftPanelSize).toBe(40);
  });

  it("should preserve other panel sizes when updating one", () => {
    const { result } = renderHook(() => useLayout(), { wrapper });

    const originalTop = result.current.layoutState.topPanelSize;
    const originalBottom = [...result.current.layoutState.bottomPanelSizes];

    act(() => {
      result.current.updateLeftPanel(80);
    });

    expect(result.current.layoutState.topPanelSize).toBe(originalTop);
    expect(result.current.layoutState.bottomPanelSizes).toEqual(originalBottom);
    expect(result.current.layoutState.leftPanelSize).toBe(80);
  });
});
