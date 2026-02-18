/**
 * Thumbnail Context Tests
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { ThumbnailProvider, useThumbnails } from "../ThumbnailContext";
import { invoke } from "@tauri-apps/api/core";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockInvoke = invoke as vi.MockedFunction<typeof invoke>;

describe("ThumbnailContext", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should provide default empty state", () => {
    const { result } = renderHook(() => useThumbnails(), {
      wrapper: ({ children }) => (
        <ThumbnailProvider>{children}</ThumbnailProvider>
      ),
    });

    expect(result.current.thumbnails).toBeInstanceOf(Map);
    expect(result.current.thumbnails.size).toBe(0);
    expect(result.current.loading).toBeInstanceOf(Set);
    expect(result.current.loading.size).toBe(0);
  });

  it("should load thumbnails successfully", async () => {
    const mockResults = [
      {
        frame_index: 0,
        thumbnail_data: "base64data",
        width: 100,
        height: 56,
        success: true,
      },
      {
        frame_index: 1,
        thumbnail_data: "base64data2",
        width: 100,
        height: 56,
        success: true,
      },
    ];
    mockInvoke.mockResolvedValue(mockResults);

    const { result } = renderHook(() => useThumbnails(), {
      wrapper: ({ children }) => (
        <ThumbnailProvider>{children}</ThumbnailProvider>
      ),
    });

    await act(async () => {
      await result.current.loadThumbnails([0, 1]);
    });

    expect(result.current.thumbnails.get(0)).toBeTruthy();
    expect(result.current.thumbnails.get(1)).toBeTruthy();
    expect(result.current.loading.size).toBe(0);
  });

  it("should mark thumbnails as loading during load", async () => {
    mockInvoke.mockImplementation(
      () =>
        new Promise((resolve) =>
          setTimeout(
            () =>
              resolve([
                {
                  frame_index: 0,
                  thumbnail_data: "data",
                  width: 100,
                  height: 56,
                  success: true,
                },
              ]),
            100,
          ),
        ),
    );

    const { result } = renderHook(() => useThumbnails(), {
      wrapper: ({ children }) => (
        <ThumbnailProvider>{children}</ThumbnailProvider>
      ),
    });

    act(() => {
      result.current.loadThumbnails([0]);
    });

    expect(result.current.loading.has(0)).toBe(true);

    await waitFor(() => {
      expect(result.current.loading.has(0)).toBe(false);
    });
  });

  it("should not load already cached thumbnails", async () => {
    const mockResults = [
      {
        frame_index: 0,
        thumbnail_data: "data",
        width: 100,
        height: 56,
        success: true,
      },
    ];
    mockInvoke.mockResolvedValue(mockResults);

    const { result } = renderHook(() => useThumbnails(), {
      wrapper: ({ children }) => (
        <ThumbnailProvider>{children}</ThumbnailProvider>
      ),
    });

    await act(async () => {
      await result.current.loadThumbnails([0]);
    });

    const callCount = mockInvoke.mock.calls.length;

    await act(async () => {
      await result.current.loadThumbnails([0]);
    });

    expect(mockInvoke.mock.calls.length).toBe(callCount);
  });

  it("should not load currently loading thumbnails", async () => {
    let resolveInvoke: (value: unknown) => void;
    mockInvoke.mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveInvoke = resolve;
        }),
    );

    const { result } = renderHook(() => useThumbnails(), {
      wrapper: ({ children }) => (
        <ThumbnailProvider>{children}</ThumbnailProvider>
      ),
    });

    act(() => {
      result.current.loadThumbnails([0]);
    });

    act(() => {
      result.current.loadThumbnails([0, 1]);
    });

    // Should only call invoke once for frame 0
    expect(mockInvoke.mock.calls.length).toBe(1);

    resolveInvoke!([
      {
        frame_index: 0,
        thumbnail_data: "data",
        width: 100,
        height: 56,
        success: true,
      },
    ]);
  });

  it("should get thumbnail for frame", async () => {
    const mockResults = [
      {
        frame_index: 5,
        thumbnail_data: "base64data",
        width: 100,
        height: 56,
        success: true,
      },
    ];
    mockInvoke.mockResolvedValue(mockResults);

    const { result } = renderHook(() => useThumbnails(), {
      wrapper: ({ children }) => (
        <ThumbnailProvider>{children}</ThumbnailProvider>
      ),
    });

    await act(async () => {
      await result.current.loadThumbnails([5]);
    });

    expect(result.current.getThumbnail(5)).toBeTruthy();
    expect(result.current.getThumbnail(999)).toBeUndefined();
  });

  it("should check if thumbnail is loading", async () => {
    mockInvoke.mockImplementation(
      () => new Promise((resolve) => setTimeout(() => resolve([]), 100)),
    );

    const { result } = renderHook(() => useThumbnails(), {
      wrapper: ({ children }) => (
        <ThumbnailProvider>{children}</ThumbnailProvider>
      ),
    });

    act(() => {
      result.current.loadThumbnails([0]);
    });

    expect(result.current.isLoading(0)).toBe(true);
    expect(result.current.isLoading(1)).toBe(false);
  });

  it("should clear all cached thumbnails", async () => {
    const mockResults = [
      {
        frame_index: 0,
        thumbnail_data: "data",
        width: 100,
        height: 56,
        success: true,
      },
      {
        frame_index: 1,
        thumbnail_data: "data2",
        width: 100,
        height: 56,
        success: true,
      },
    ];
    mockInvoke.mockResolvedValue(mockResults);

    const { result } = renderHook(() => useThumbnails(), {
      wrapper: ({ children }) => (
        <ThumbnailProvider>{children}</ThumbnailProvider>
      ),
    });

    await act(async () => {
      await result.current.loadThumbnails([0, 1]);
    });

    expect(result.current.thumbnails.size).toBe(2);

    act(() => {
      result.current.clearCache();
    });

    expect(result.current.thumbnails.size).toBe(0);
  });

  it("should preload range of thumbnails", async () => {
    const mockResults = Array.from({ length: 10 }, (_, i) => ({
      frame_index: i,
      thumbnail_data: `data${i}`,
      width: 100,
      height: 56,
      success: true,
    }));
    mockInvoke.mockResolvedValue(mockResults);

    const { result } = renderHook(() => useThumbnails(), {
      wrapper: ({ children }) => (
        <ThumbnailProvider>{children}</ThumbnailProvider>
      ),
    });

    await act(async () => {
      await result.current.preloadRange(0, 10);
    });

    expect(mockInvoke).toHaveBeenCalledWith("get_thumbnails", {
      frameIndices: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
    });
  });

  it("should handle failed thumbnail loads", async () => {
    mockInvoke.mockResolvedValue([
      {
        frame_index: 0,
        thumbnail_data: "data",
        width: 100,
        height: 56,
        success: true,
      },
      { frame_index: 1, success: false, error: "Failed to load" },
    ]);

    const { result } = renderHook(() => useThumbnails(), {
      wrapper: ({ children }) => (
        <ThumbnailProvider>{children}</ThumbnailProvider>
      ),
    });

    await act(async () => {
      await result.current.loadThumbnails([0, 1]);
    });

    expect(result.current.thumbnails.get(0)).toBeTruthy();
    expect(result.current.thumbnails.get(1)).toBeUndefined();
  });

  it("should handle empty results", async () => {
    mockInvoke.mockResolvedValue([]);

    const { result } = renderHook(() => useThumbnails(), {
      wrapper: ({ children }) => (
        <ThumbnailProvider>{children}</ThumbnailProvider>
      ),
    });

    await act(async () => {
      await result.current.loadThumbnails([0, 1]);
    });

    expect(result.current.thumbnails.size).toBe(0);
  });

  it("should convert SVG data to data URL", async () => {
    const svgData = "<svg>test</svg>";
    mockInvoke.mockResolvedValue([
      {
        frame_index: 0,
        thumbnail_data: svgData,
        width: 100,
        height: 56,
        success: true,
      },
    ]);

    const { result } = renderHook(() => useThumbnails(), {
      wrapper: ({ children }) => (
        <ThumbnailProvider>{children}</ThumbnailProvider>
      ),
    });

    await act(async () => {
      await result.current.loadThumbnails([0]);
    });

    const thumbnail = result.current.getThumbnail(0);
    expect(thumbnail).toMatch(/^data:image\/svg\+xml;base64,/);
  });

  it("should convert PNG data to data URL", async () => {
    const pngData =
      "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
    mockInvoke.mockResolvedValue([
      {
        frame_index: 0,
        thumbnail_data: pngData,
        width: 100,
        height: 56,
        success: true,
      },
    ]);

    const { result } = renderHook(() => useThumbnails(), {
      wrapper: ({ children }) => (
        <ThumbnailProvider>{children}</ThumbnailProvider>
      ),
    });

    await act(async () => {
      await result.current.loadThumbnails([0]);
    });

    const thumbnail = result.current.getThumbnail(0);
    expect(thumbnail).toMatch(/^data:image\/png;base64,/);
  });

  it("should handle invoke errors gracefully", async () => {
    mockInvoke.mockRejectedValue(new Error("RPC error"));

    const { result } = renderHook(() => useThumbnails(), {
      wrapper: ({ children }) => (
        <ThumbnailProvider>{children}</ThumbnailProvider>
      ),
    });

    await act(async () => {
      await result.current.loadThumbnails([0]);
    });

    expect(result.current.loading.size).toBe(0);
  });

  it("should throw error when useThumbnails is used outside provider", () => {
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});

    expect(() => {
      renderHook(() => useThumbnails());
    }).toThrow("useThumbnails must be used within a ThumbnailProvider");

    consoleSpy.mockRestore();
  });

  it("should provide context to all children", () => {
    const wrapper = ({ children }: { children: React.ReactNode }) => (
      <ThumbnailProvider>{children}</ThumbnailProvider>
    );

    const { result: result1 } = renderHook(() => useThumbnails(), { wrapper });
    const { result: result2 } = renderHook(() => useThumbnails(), { wrapper });

    expect(result1.current.thumbnails).toBeInstanceOf(Map);
    expect(result2.current.thumbnails).toBeInstanceOf(Map);
  });

  it("should filter out already loaded thumbnails from request", async () => {
    const mockResults = [
      {
        frame_index: 0,
        thumbnail_data: "data",
        width: 100,
        height: 56,
        success: true,
      },
    ];
    mockInvoke.mockResolvedValue(mockResults);

    const { result } = renderHook(() => useThumbnails(), {
      wrapper: ({ children }) => (
        <ThumbnailProvider>{children}</ThumbnailProvider>
      ),
    });

    await act(async () => {
      await result.current.loadThumbnails([0, 1, 2]);
    });

    const firstCallArgs = mockInvoke.mock.calls[0][1];
    expect(firstCallArgs.frameIndices).toEqual([0, 1, 2]);

    mockInvoke.mockResolvedValue([
      {
        frame_index: 1,
        thumbnail_data: "data2",
        width: 100,
        height: 56,
        success: true,
      },
    ]);

    await act(async () => {
      await result.current.loadThumbnails([0, 1, 2, 3]);
    });

    const secondCallArgs = mockInvoke.mock.calls[1][1];
    // Should only request 2 and 3 since 0 and 1 are already loaded
    expect(secondCallArgs.frameIndices).toEqual([2, 3]);
  });

  it("should handle thumbnails without data", async () => {
    mockInvoke.mockResolvedValue([
      {
        frame_index: 0,
        thumbnail_data: "",
        width: 100,
        height: 56,
        success: true,
      },
    ]);

    const { result } = renderHook(() => useThumbnails(), {
      wrapper: ({ children }) => (
        <ThumbnailProvider>{children}</ThumbnailProvider>
      ),
    });

    await act(async () => {
      await result.current.loadThumbnails([0]);
    });

    expect(result.current.thumbnails.get(0)).toBeUndefined();
  });
});
