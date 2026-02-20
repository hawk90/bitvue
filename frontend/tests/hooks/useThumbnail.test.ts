/**
 * useThumbnail Hook Tests
 * Tests thumbnail caching and loading functionality
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";

// Mock Tauri invoke
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

// Mock logger
vi.mock("../../utils/logger", () => ({
  createLogger: () => ({
    error: vi.fn(),
    warn: vi.fn(),
    info: vi.fn(),
    debug: vi.fn(),
  }),
}));

// Import after mocking
import { invoke } from "@tauri-apps/api/core";
import { useThumbnail, ThumbnailResult } from "@/hooks/useThumbnail";

const mockInvoke = invoke as unknown as ReturnType<typeof vi.fn>;

describe("useThumbnail", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("should initialize with empty cache", () => {
    const { result } = renderHook(() => useThumbnail());

    expect(result.current.thumbnails).toBeInstanceOf(Map);
    expect(result.current.thumbnails.size).toBe(0);
    expect(result.current.loading).toBeInstanceOf(Set);
    expect(result.current.loading.size).toBe(0);
  });

  it("should have all required methods", () => {
    const { result } = renderHook(() => useThumbnail());

    expect(typeof result.current.loadThumbnails).toBe("function");
    expect(typeof result.current.clearCache).toBe("function");
    expect(typeof result.current.has).toBe("function");
    expect(typeof result.current.get).toBe("function");
  });

  it("should load thumbnails for specified indices", async () => {
    const mockResults: ThumbnailResult[] = [
      {
        frame_index: 0,
        thumbnail_data: "iVBORw0KGgo",
        width: 320,
        height: 180,
        success: true,
      },
      {
        frame_index: 1,
        thumbnail_data: "iVBORw0KGgo",
        width: 320,
        height: 180,
        success: true,
      },
    ];

    mockInvoke.mockResolvedValue(mockResults);

    const { result } = renderHook(() => useThumbnail());

    await act(async () => {
      await result.current.loadThumbnails([0, 1]);
    });

    expect(result.current.thumbnails.get(0)).toBe(
      "data:image/png;base64,iVBORw0KGgo",
    );
    expect(result.current.thumbnails.get(1)).toBe(
      "data:image/png;base64,iVBORw0KGgo",
    );
    expect(mockInvoke).toHaveBeenCalledWith("get_thumbnails", {
      frameIndices: [0, 1],
    });
  });

  it("should mark frames as loading during fetch", async () => {
    let resolveInvoke: (value: ThumbnailResult[]) => void;
    mockInvoke.mockImplementation(() => {
      return new Promise((resolve) => {
        resolveInvoke = resolve;
      });
    });

    const { result } = renderHook(() => useThumbnail());

    // Start the loading process inside act
    let loadPromise: Promise<void>;
    await act(async () => {
      loadPromise = result.current.loadThumbnails([0, 1]);
      // Wait a tick for state to update
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    // Check loading state
    expect(result.current.loading.has(0)).toBe(true);
    expect(result.current.loading.has(1)).toBe(true);

    // Resolve the promise
    await act(async () => {
      resolveInvoke!([
        {
          frame_index: 0,
          thumbnail_data: "data",
          width: 320,
          height: 180,
          success: true,
        },
        {
          frame_index: 1,
          thumbnail_data: "data",
          width: 320,
          height: 180,
          success: true,
        },
      ]);
      await loadPromise!;
    });

    // Should not be loading anymore
    expect(result.current.loading.has(0)).toBe(false);
    expect(result.current.loading.has(1)).toBe(false);
  });

  it("should clear cache", () => {
    const { result } = renderHook(() => useThumbnail());

    // Load some thumbnails first
    act(() => {
      result.current.clearCache();
    });

    expect(result.current.thumbnails.size).toBe(0);
  });

  it("should handle loading errors gracefully", async () => {
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    mockInvoke.mockRejectedValue(new Error("Failed to load"));

    const { result } = renderHook(() => useThumbnail());

    await act(async () => {
      await result.current.loadThumbnails([0]);
    });

    // Thumbnail should not be added to cache
    expect(result.current.thumbnails.has(0)).toBe(false);
    expect(result.current.loading.has(0)).toBe(false);

    consoleSpy.mockRestore();
  });

  it("should not reload already cached thumbnails", async () => {
    mockInvoke.mockResolvedValue([
      {
        frame_index: 0,
        thumbnail_data: "data",
        width: 320,
        height: 180,
        success: true,
      },
    ]);

    const { result } = renderHook(() => useThumbnail());

    // Load once
    await act(async () => {
      await result.current.loadThumbnails([0]);
    });

    expect(mockInvoke).toHaveBeenCalledTimes(1);

    // Load again - should not invoke again since already cached
    await act(async () => {
      await result.current.loadThumbnails([0]);
    });

    // Should not call invoke again for cached thumbnails
    expect(mockInvoke).toHaveBeenCalledTimes(1);
  });

  it("should not reload currently loading thumbnails", async () => {
    let resolveInvoke: (value: ThumbnailResult[]) => void;
    mockInvoke.mockImplementation(() => {
      return new Promise((resolve) => {
        resolveInvoke = resolve;
      });
    });

    const { result } = renderHook(() => useThumbnail());

    // Start first load
    let loadPromise1: Promise<void>;
    await act(async () => {
      loadPromise1 = result.current.loadThumbnails([0]);
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    // Try to load again while first is still loading
    let loadPromise2: Promise<void>;
    await act(async () => {
      loadPromise2 = result.current.loadThumbnails([0]);
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    // Should only invoke once (the second call should be filtered out)
    expect(mockInvoke).toHaveBeenCalledTimes(1);

    await act(async () => {
      resolveInvoke!([
        {
          frame_index: 0,
          thumbnail_data: "data",
          width: 320,
          height: 180,
          success: true,
        },
      ]);
      await Promise.all([loadPromise1!, loadPromise2!]);
    });
  });

  it("should use stable callbacks (useCallback optimization)", () => {
    const { result, rerender } = renderHook(() => useThumbnail());

    const loadThumbnailsRef = result.current.loadThumbnails;
    const clearCacheRef = result.current.clearCache;
    const hasRef = result.current.has;
    const getRef = result.current.get;

    rerender();

    // Callbacks should be stable
    expect(result.current.loadThumbnails).toBe(loadThumbnailsRef);
    expect(result.current.clearCache).toBe(clearCacheRef);
    expect(result.current.has).toBe(hasRef);
    expect(result.current.get).toBe(getRef);
  });

  it("should handle empty indices array", async () => {
    const { result } = renderHook(() => useThumbnail());

    await act(async () => {
      await result.current.loadThumbnails([]);
    });

    // Should not invoke
    expect(mockInvoke).not.toHaveBeenCalled();
    expect(result.current.thumbnails.size).toBe(0);
  });

  it("should handle large batch of thumbnails", async () => {
    const indices = Array.from({ length: 100 }, (_, i) => i);
    const mockData: ThumbnailResult[] = indices.map((i) => ({
      frame_index: i,
      thumbnail_data: `data-${i}`,
      width: 320,
      height: 180,
      success: true,
    }));

    mockInvoke.mockResolvedValue(mockData);

    const { result } = renderHook(() => useThumbnail());

    await act(async () => {
      await result.current.loadThumbnails(indices);
    });

    expect(result.current.thumbnails.size).toBe(100);
    expect(result.current.thumbnails.get(0)).toContain("data-0");
    expect(result.current.thumbnails.get(99)).toContain("data-99");
  });
});

describe("useThumbnail has() method", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should return false for non-existent thumbnail", () => {
    const { result } = renderHook(() => useThumbnail());

    expect(result.current.has(0)).toBe(false);
    expect(result.current.has(999)).toBe(false);
  });

  it("should return true for cached thumbnail", async () => {
    mockInvoke.mockResolvedValue([
      {
        frame_index: 0,
        thumbnail_data: "data",
        width: 320,
        height: 180,
        success: true,
      },
    ]);

    const { result } = renderHook(() => useThumbnail());

    await act(async () => {
      await result.current.loadThumbnails([0]);
    });

    expect(result.current.has(0)).toBe(true);
    expect(result.current.has(1)).toBe(false);
  });
});

describe("useThumbnail get() method", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should return undefined for non-existent thumbnail", () => {
    const { result } = renderHook(() => useThumbnail());

    expect(result.current.get(0)).toBeUndefined();
    expect(result.current.get(999)).toBeUndefined();
  });

  it("should return data URL for cached thumbnail", async () => {
    mockInvoke.mockResolvedValue([
      {
        frame_index: 5,
        thumbnail_data: "iVBORw0KGgo",
        width: 320,
        height: 180,
        success: true,
      },
    ]);

    const { result } = renderHook(() => useThumbnail());

    await act(async () => {
      await result.current.loadThumbnails([5]);
    });

    const thumbnail = result.current.get(5);
    expect(thumbnail).toBe("data:image/png;base64,iVBORw0KGgo");
    expect(thumbnail).toMatch(/^data:image\/png;base64,/);
  });
});

describe("useThumbnail data URL format", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should detect PNG format from magic bytes", async () => {
    mockInvoke.mockResolvedValue([
      {
        frame_index: 0,
        thumbnail_data: "iVBORw0KGgo",
        width: 320,
        height: 180,
        success: true,
      },
    ]);

    const { result } = renderHook(() => useThumbnail());

    await act(async () => {
      await result.current.loadThumbnails([0]);
    });

    expect(result.current.get(0)).toBe("data:image/png;base64,iVBORw0KGgo");
  });

  it("should detect SVG format", async () => {
    mockInvoke.mockResolvedValue([
      {
        frame_index: 0,
        thumbnail_data: "<svg>test</svg>",
        width: 320,
        height: 180,
        success: true,
      },
    ]);

    const { result } = renderHook(() => useThumbnail());

    await act(async () => {
      await result.current.loadThumbnails([0]);
    });

    expect(result.current.get(0)).toBe(
      "data:image/svg+xml;base64,<svg>test</svg>",
    );
  });

  it("should default to PNG for unknown format", async () => {
    mockInvoke.mockResolvedValue([
      {
        frame_index: 0,
        thumbnail_data: "randomdata",
        width: 320,
        height: 180,
        success: true,
      },
    ]);

    const { result } = renderHook(() => useThumbnail());

    await act(async () => {
      await result.current.loadThumbnails([0]);
    });

    expect(result.current.get(0)).toBe("data:image/png;base64,randomdata");
  });
});

describe("useThumbnail error handling", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should handle failed thumbnail results", async () => {
    mockInvoke.mockResolvedValue([
      {
        frame_index: 0,
        thumbnail_data: "data",
        width: 320,
        height: 180,
        success: true,
      },
      {
        frame_index: 1,
        thumbnail_data: "",
        width: 0,
        height: 0,
        success: false,
        error: "Failed to render",
      },
      {
        frame_index: 2,
        thumbnail_data: "data2",
        width: 320,
        height: 180,
        success: true,
      },
    ]);

    const { result } = renderHook(() => useThumbnail());

    await act(async () => {
      await result.current.loadThumbnails([0, 1, 2]);
    });

    // Should have successful thumbnails
    expect(result.current.has(0)).toBe(true);
    expect(result.current.has(2)).toBe(true);

    // Should not have failed thumbnail
    expect(result.current.has(1)).toBe(false);
  });

  it("should handle thumbnails with missing data", async () => {
    mockInvoke.mockResolvedValue([
      {
        frame_index: 0,
        thumbnail_data: "data",
        width: 320,
        height: 180,
        success: true,
      },
      {
        frame_index: 1,
        thumbnail_data: "",
        width: 320,
        height: 180,
        success: true,
      }, // Missing data
    ]);

    const { result } = renderHook(() => useThumbnail());

    await act(async () => {
      await result.current.loadThumbnails([0, 1]);
    });

    // Only the one with data should be cached
    expect(result.current.has(0)).toBe(true);
    expect(result.current.has(1)).toBe(false);
  });

  it("should handle network errors with retry", async () => {
    mockInvoke
      .mockRejectedValueOnce(new Error("Network error"))
      .mockResolvedValue([
        {
          frame_index: 0,
          thumbnail_data: "data",
          width: 320,
          height: 180,
          success: true,
        },
      ]);

    const { result } = renderHook(() => useThumbnail());

    // First attempt fails
    await act(async () => {
      await result.current.loadThumbnails([0]);
    });

    expect(result.current.has(0)).toBe(false);

    // Second attempt succeeds
    await act(async () => {
      await result.current.loadThumbnails([0]);
    });

    expect(result.current.has(0)).toBe(true);
  });
});

describe("useThumbnail cache behavior", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should support concurrent load requests for different frames", async () => {
    mockInvoke.mockResolvedValue([
      {
        frame_index: 0,
        thumbnail_data: "data0",
        width: 320,
        height: 180,
        success: true,
      },
      {
        frame_index: 1,
        thumbnail_data: "data1",
        width: 320,
        height: 180,
        success: true,
      },
    ]);

    const { result } = renderHook(() => useThumbnail());

    await act(async () => {
      await Promise.all([
        result.current.loadThumbnails([0]),
        result.current.loadThumbnails([1]),
      ]);
    });

    expect(result.current.thumbnails.size).toBe(2);
  });

  it("should handle loading same frame from multiple concurrent calls", async () => {
    let resolveInvoke: (value: ThumbnailResult[]) => void;
    mockInvoke.mockImplementation(() => {
      return new Promise((resolve) => {
        resolveInvoke = resolve;
      });
    });

    const { result } = renderHook(() => useThumbnail());

    // Start first load
    let loadPromise1: Promise<void>;
    await act(async () => {
      loadPromise1 = result.current.loadThumbnails([0]);
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    // Start second and third loads while first is still loading
    let loadPromise2: Promise<void>;
    let loadPromise3: Promise<void>;
    await act(async () => {
      loadPromise2 = result.current.loadThumbnails([0]);
      loadPromise3 = result.current.loadThumbnails([0]);
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    // Should only invoke once (the first call, subsequent ones are filtered out)
    expect(mockInvoke).toHaveBeenCalledTimes(1);

    await act(async () => {
      resolveInvoke!([
        {
          frame_index: 0,
          thumbnail_data: "data",
          width: 320,
          height: 180,
          success: true,
        },
      ]);
      await Promise.all([loadPromise1!, loadPromise2!, loadPromise3!]);
    });

    // Should have exactly one thumbnail
    expect(result.current.thumbnails.size).toBe(1);
  });

  it("should handle selective loading (only uncached)", async () => {
    mockInvoke.mockResolvedValue([
      {
        frame_index: 0,
        thumbnail_data: "data0",
        width: 320,
        height: 180,
        success: true,
      },
      {
        frame_index: 1,
        thumbnail_data: "data1",
        width: 320,
        height: 180,
        success: true,
      },
    ]);

    const { result } = renderHook(() => useThumbnail());

    // Load initial thumbnails
    await act(async () => {
      await result.current.loadThumbnails([0, 1]);
    });

    mockInvoke.mockClear();
    mockInvoke.mockResolvedValue([
      {
        frame_index: 2,
        thumbnail_data: "data2",
        width: 320,
        height: 180,
        success: true,
      },
    ]);

    // Load mix of cached and uncached
    await act(async () => {
      await result.current.loadThumbnails([0, 1, 2]);
    });

    // Should only load the uncached one
    expect(mockInvoke).toHaveBeenCalledWith("get_thumbnails", {
      frameIndices: [2],
    });
  });
});

describe("useThumbnail edge cases", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should handle negative frame indices", async () => {
    mockInvoke.mockResolvedValue([
      {
        frame_index: -1,
        thumbnail_data: "data",
        width: 320,
        height: 180,
        success: true,
      },
    ]);

    const { result } = renderHook(() => useThumbnail());

    await act(async () => {
      await result.current.loadThumbnails([-1]);
    });

    expect(result.current.has(-1)).toBe(true);
  });

  it("should handle very large frame indices", async () => {
    mockInvoke.mockResolvedValue([
      {
        frame_index: 999999,
        thumbnail_data: "data",
        width: 320,
        height: 180,
        success: true,
      },
    ]);

    const { result } = renderHook(() => useThumbnail());

    await act(async () => {
      await result.current.loadThumbnails([999999]);
    });

    expect(result.current.has(999999)).toBe(true);
  });

  it("should handle duplicate indices in request", async () => {
    mockInvoke.mockResolvedValue([
      {
        frame_index: 0,
        thumbnail_data: "data",
        width: 320,
        height: 180,
        success: true,
      },
    ]);

    const { result } = renderHook(() => useThumbnail());

    await act(async () => {
      await result.current.loadThumbnails([0, 0, 0, 0]);
    });

    // Should still only have one thumbnail
    expect(result.current.thumbnails.size).toBe(1);
    expect(mockInvoke).toHaveBeenCalledTimes(1);
  });

  it("should maintain cache after clearing and reloading", async () => {
    mockInvoke.mockResolvedValue([
      {
        frame_index: 0,
        thumbnail_data: "data",
        width: 320,
        height: 180,
        success: true,
      },
    ]);

    const { result } = renderHook(() => useThumbnail());

    // Load
    await act(async () => {
      await result.current.loadThumbnails([0]);
    });
    expect(result.current.thumbnails.size).toBe(1);

    // Clear
    act(() => {
      result.current.clearCache();
    });
    expect(result.current.thumbnails.size).toBe(0);

    // Reload
    await act(async () => {
      await result.current.loadThumbnails([0]);
    });
    expect(result.current.thumbnails.size).toBe(1);
  });
});

describe("useThumbnail loading state management", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should correctly update loading state through lifecycle", async () => {
    let resolveInvoke: (value: ThumbnailResult[]) => void;
    mockInvoke.mockImplementation(() => {
      return new Promise((resolve) => {
        resolveInvoke = resolve;
      });
    });

    const { result } = renderHook(() => useThumbnail());

    // Initial state
    expect(result.current.loading.size).toBe(0);

    // Start loading
    let loadPromise: Promise<void>;
    await act(async () => {
      loadPromise = result.current.loadThumbnails([0, 1, 2]);
      // Wait for state to update
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    // During loading
    expect(result.current.loading.has(0)).toBe(true);
    expect(result.current.loading.has(1)).toBe(true);
    expect(result.current.loading.has(2)).toBe(true);
    expect(result.current.loading.size).toBe(3);

    // Finish loading
    await act(async () => {
      resolveInvoke!([
        {
          frame_index: 0,
          thumbnail_data: "data0",
          width: 320,
          height: 180,
          success: true,
        },
        {
          frame_index: 1,
          thumbnail_data: "data1",
          width: 320,
          height: 180,
          success: true,
        },
        {
          frame_index: 2,
          thumbnail_data: "data2",
          width: 320,
          height: 180,
          success: true,
        },
      ]);
      await loadPromise!;
    });

    // After loading
    expect(result.current.loading.size).toBe(0);
  });

  it("should clear loading state even on error", async () => {
    mockInvoke.mockRejectedValue(new Error("Load failed"));

    const { result } = renderHook(() => useThumbnail());

    await act(async () => {
      await result.current.loadThumbnails([0, 1]);
    });

    // Loading state should be cleared
    expect(result.current.loading.has(0)).toBe(false);
    expect(result.current.loading.has(1)).toBe(false);
  });
});
