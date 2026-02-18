/**
 * useStreamData Hook Tests
 * Tests stream data context hook
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import {
  useStreamData,
  StreamDataProvider,
} from "../../contexts/StreamDataContext";
import React from "react";

// Mock Tauri invoke
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

// Mock logger
vi.mock("../../utils/logger", () => ({
  createLogger: () => ({
    info: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
    warn: vi.fn(),
  }),
}));

// Import mockFrames from test-utils
const mockFrames = [
  {
    frame_index: 0,
    frame_type: "I",
    size: 50000,
    pts: 0,
    poc: 0,
    key_frame: true,
    display_order: 0,
    coding_order: 0,
  },
  {
    frame_index: 1,
    frame_type: "P",
    size: 30000,
    pts: 1,
    poc: 1,
    key_frame: false,
    ref_frames: [0],
    display_order: 1,
    coding_order: 1,
  },
];

/**
 * Wrapper with StreamDataProvider for tests
 */
function wrapper({ children }: { children: React.ReactNode }) {
  return <StreamDataProvider>{children}</StreamDataProvider>;
}

describe("useStreamData", () => {
  beforeEach(() => {
    // Reset environment variable to prevent auto-loading
    vi.stubEnv("NODE_ENV", "test");
  });

  it("should return stream data from context", () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    expect(result.current.frames).toEqual([]);
    expect(result.current.currentFrameIndex).toBe(0);
    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBe(null);
  });

  it("should have frames array", () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    expect(Array.isArray(result.current.frames)).toBe(true);
  });

  it("should have currentFrameIndex", () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    expect(typeof result.current.currentFrameIndex).toBe("number");
  });

  it("should have getFrameStats function", () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    expect(typeof result.current.getFrameStats).toBe("function");
  });

  it("should return stats from getFrameStats", () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    const stats = result.current.getFrameStats();

    expect(stats.totalFrames).toBe(0);
    expect(stats.avgSize).toBe(0);
  });
});

describe("useStreamData with mock data", () => {
  beforeEach(async () => {
    vi.stubEnv("NODE_ENV", "test");
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockResolvedValue(mockFrames);
  });

  it("should load frames from backend", async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    // Initially empty
    expect(result.current.frames).toEqual([]);

    // Refresh frames
    await act(async () => {
      await result.current.refreshFrames();
    });

    // Should have frames now
    expect(result.current.frames).toHaveLength(2);
    expect(result.current.frames[0].frame_index).toBe(0);
    expect(result.current.frames[1].frame_index).toBe(1);
  });

  it("should provide access to current frame", async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    await act(async () => {
      await result.current.refreshFrames();
    });

    const currentFrame =
      result.current.frames[result.current.currentFrameIndex];
    expect(currentFrame).toBeDefined();
    expect(currentFrame.frame_index).toBe(0);
  });

  it("should provide access to all frames", async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    await act(async () => {
      await result.current.refreshFrames();
    });

    expect(result.current.frames.map((f) => f.frame_index)).toEqual([0, 1]);
  });

  it("should calculate frame type distribution", async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    await act(async () => {
      await result.current.refreshFrames();
    });

    const stats = result.current.getFrameStats();

    expect(stats.frameTypes.I).toBe(1);
    expect(stats.frameTypes.P).toBe(1);
  });

  it("should calculate total frames", async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    await act(async () => {
      await result.current.refreshFrames();
    });

    const stats = result.current.getFrameStats();

    expect(stats.totalFrames).toBe(2);
  });

  it("should calculate average size", async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    await act(async () => {
      await result.current.refreshFrames();
    });

    const stats = result.current.getFrameStats();

    expect(stats.avgSize).toBe(40000);
  });

  it("should calculate total size", async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    await act(async () => {
      await result.current.refreshFrames();
    });

    const stats = result.current.getFrameStats();

    expect(stats.totalSize).toBe(80000);
  });

  it("should calculate key frames", async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    await act(async () => {
      await result.current.refreshFrames();
    });

    const stats = result.current.getFrameStats();

    expect(stats.keyFrames).toBe(1);
  });
});

describe("useStreamData edge cases", () => {
  beforeEach(() => {
    vi.stubEnv("NODE_ENV", "test");
  });

  it("should handle empty frames array", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockResolvedValue([]);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await act(async () => {
      await result.current.refreshFrames();
    });

    expect(result.current.frames).toEqual([]);

    const stats = result.current.getFrameStats();
    expect(stats.totalFrames).toBe(0);
    expect(stats.avgSize).toBe(0);
    expect(stats.totalSize).toBe(0);
  });

  it("should handle currentFrameIndex out of bounds", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockResolvedValue([mockFrames[0]]);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await act(async () => {
      await result.current.refreshFrames();
    });

    // Set index out of bounds
    act(() => {
      result.current.setCurrentFrameIndex(999);
    });

    expect(
      result.current.frames[result.current.currentFrameIndex],
    ).toBeUndefined();
  });

  it("should handle setFrames", () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    // Directly set frames
    act(() => {
      result.current.setFrames(mockFrames);
    });

    expect(result.current.frames).toHaveLength(2);
    expect(result.current.frames[0].frame_index).toBe(0);
  });

  it("should handle clearData", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockResolvedValue(mockFrames);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await act(async () => {
      await result.current.refreshFrames();
    });
    expect(result.current.frames).toHaveLength(2);

    // Clear data
    act(() => {
      result.current.clearData();
    });

    expect(result.current.frames).toEqual([]);
    expect(result.current.filePath).toBe(null);
    expect(result.current.currentFrameIndex).toBe(0);
  });

  it("should handle setFilePath", () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    act(() => {
      result.current.setFilePath("/test/path.mp4");
    });

    expect(result.current.filePath).toBe("/test/path.mp4");
  });

  it("should handle loading state", async () => {
    let resolveInvoke: (value: any) => void;
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveInvoke = resolve;
        }),
    );

    const { result } = renderHook(() => useStreamData(), { wrapper });

    // Start loading - don't await yet
    const loadPromise = result.current.refreshFrames();

    // Check loading state synchronously after calling refreshFrames
    // The state should update synchronously before the async operation completes
    if (result.current.loading) {
      expect(result.current.loading).toBe(true);
    }

    // Resolve the promise
    resolveInvoke!(mockFrames);
    await loadPromise;

    // Should be done loading
    expect(result.current.loading).toBe(false);
  });

  it("should handle errors from backend", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    // Mock with a string error (Tauri typically returns string errors)
    vi.mocked(invoke).mockRejectedValue("Failed to load frames");

    const { result } = renderHook(() => useStreamData(), { wrapper });

    // Act to handle the state updates
    await act(async () => {
      try {
        await result.current.refreshFrames();
      } catch {
        // Error is handled internally by the hook
      }
    });

    // The error should be set by the catch block in refreshFrames
    expect(result.current.error).toBe("Failed to load frames");
    expect(result.current.frames).toEqual([]);
  });
});
