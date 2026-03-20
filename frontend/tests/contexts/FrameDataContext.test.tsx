/**
 * FrameDataContext Tests
 * Tests frame data context provider including stats calculation,
 * worker usage, and cleanup behaviour.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import React from "react";
import { FrameDataProvider, useFrameData } from "@/contexts/FrameDataContext";
import type { FrameInfo } from "@/types/video";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const wrapper = ({ children }: { children: React.ReactNode }) => (
  <FrameDataProvider>{children}</FrameDataProvider>
);

function makeFrame(
  index: number,
  type: "I" | "P" | "B",
  size: number,
  keyFrame?: boolean,
): FrameInfo {
  return {
    frame_index: index,
    frame_type: type,
    size,
    poc: index,
    key_frame: keyFrame,
  };
}

// ---------------------------------------------------------------------------
// Worker mock helpers
// ---------------------------------------------------------------------------

type MockWorkerInstance = {
  postMessage: ReturnType<typeof vi.fn>;
  terminate: ReturnType<typeof vi.fn>;
  onmessage: ((e: MessageEvent) => void) | null;
  onerror: ((e: ErrorEvent) => void) | null;
};

let workerInstances: MockWorkerInstance[] = [];

function setupWorkerMock(autoReply = false) {
  workerInstances = [];

  global.Worker = vi.fn().mockImplementation(() => {
    const instance: MockWorkerInstance = {
      postMessage: vi.fn((data) => {
        if (autoReply && instance.onmessage) {
          // Simulate the worker posting stats back synchronously
          const frames = data as FrameInfo[];
          const stats = {
            totalFrames: frames.length,
            frameTypes: frames.reduce(
              (acc, f) => {
                acc[f.frame_type] = (acc[f.frame_type] || 0) + 1;
                return acc;
              },
              {} as Record<string, number>,
            ),
            totalSize: frames.reduce((s, f) => s + f.size, 0),
            avgSize:
              frames.length > 0
                ? frames.reduce((s, f) => s + f.size, 0) / frames.length
                : 0,
            keyFrames: frames.filter((f) => f.key_frame).length,
          };
          instance.onmessage(new MessageEvent("message", { data: stats }));
        }
      }),
      terminate: vi.fn(),
      onmessage: null,
      onerror: null,
    };
    workerInstances.push(instance);
    return instance;
  }) as unknown as typeof Worker;
}

function restoreWorker() {
  // Restore to undefined to avoid leaking into other tests
  // (jsdom does not have a built-in Worker)
  delete (global as Record<string, unknown>).Worker;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("FrameDataContext initial state", () => {
  it("provides empty frames array initially", () => {
    const { result } = renderHook(() => useFrameData(), { wrapper });

    expect(result.current.frames).toEqual([]);
  });

  it("getFrameStats returns all-zero stats initially", () => {
    const { result } = renderHook(() => useFrameData(), { wrapper });

    const stats = result.current.getFrameStats();
    expect(stats.totalFrames).toBe(0);
    expect(stats.totalSize).toBe(0);
    expect(stats.avgSize).toBe(0);
    expect(stats.keyFrames).toBe(0);
    expect(stats.frameTypes).toEqual({});
  });
});

describe("FrameDataContext setFrames and sync stats", () => {
  it("updating frames recalculates totalFrames", async () => {
    const { result } = renderHook(() => useFrameData(), { wrapper });

    const frames = [makeFrame(0, "I", 50000, true), makeFrame(1, "P", 30000)];

    act(() => {
      result.current.setFrames(frames);
    });

    await waitFor(() => {
      expect(result.current.getFrameStats().totalFrames).toBe(2);
    });
  });

  it("correctly totals frame sizes", async () => {
    const { result } = renderHook(() => useFrameData(), { wrapper });

    const frames = [
      makeFrame(0, "I", 10000),
      makeFrame(1, "P", 5000),
      makeFrame(2, "B", 2000),
    ];

    act(() => {
      result.current.setFrames(frames);
    });

    await waitFor(() => {
      expect(result.current.getFrameStats().totalSize).toBe(17000);
    });
  });

  it("computes average size correctly", async () => {
    const { result } = renderHook(() => useFrameData(), { wrapper });

    const frames = [makeFrame(0, "I", 10000), makeFrame(1, "P", 20000)];

    act(() => {
      result.current.setFrames(frames);
    });

    await waitFor(() => {
      expect(result.current.getFrameStats().avgSize).toBe(15000);
    });
  });

  it("stats for empty array have no division by zero", async () => {
    const { result } = renderHook(() => useFrameData(), { wrapper });

    act(() => {
      result.current.setFrames([]);
    });

    await waitFor(() => {
      const stats = result.current.getFrameStats();
      expect(stats.avgSize).toBe(0);
      expect(stats.totalFrames).toBe(0);
    });
  });

  it("single frame produces correct stats", async () => {
    const { result } = renderHook(() => useFrameData(), { wrapper });

    act(() => {
      result.current.setFrames([makeFrame(0, "I", 42000, true)]);
    });

    await waitFor(() => {
      const stats = result.current.getFrameStats();
      expect(stats.totalFrames).toBe(1);
      expect(stats.avgSize).toBe(42000);
      expect(stats.keyFrames).toBe(1);
    });
  });

  it("counts I/P/B frame types correctly", async () => {
    const { result } = renderHook(() => useFrameData(), { wrapper });

    const frames = [
      makeFrame(0, "I", 5000, true),
      makeFrame(1, "P", 3000),
      makeFrame(2, "P", 3000),
      makeFrame(3, "B", 2000),
      makeFrame(4, "B", 2000),
      makeFrame(5, "B", 2000),
    ];

    act(() => {
      result.current.setFrames(frames);
    });

    await waitFor(() => {
      const { frameTypes } = result.current.getFrameStats();
      expect(frameTypes["I"]).toBe(1);
      expect(frameTypes["P"]).toBe(2);
      expect(frameTypes["B"]).toBe(3);
    });
  });

  it("counts only key_frame:true frames as keyFrames", async () => {
    const { result } = renderHook(() => useFrameData(), { wrapper });

    const frames = [
      makeFrame(0, "I", 5000, true),
      makeFrame(1, "P", 3000, false),
      makeFrame(2, "I", 5000, true),
      makeFrame(3, "P", 3000), // key_frame undefined → falsy
    ];

    act(() => {
      result.current.setFrames(frames);
    });

    await waitFor(() => {
      expect(result.current.getFrameStats().keyFrames).toBe(2);
    });
  });

  it("getFrameStats returns the current stats value", async () => {
    const { result } = renderHook(() => useFrameData(), { wrapper });

    act(() => {
      result.current.setFrames([makeFrame(0, "I", 9000, true)]);
    });

    await waitFor(() => {
      const stats = result.current.getFrameStats();
      expect(stats.totalFrames).toBe(1);
    });
  });

  it("multiple setFrames calls each recalculate stats", async () => {
    const { result } = renderHook(() => useFrameData(), { wrapper });

    act(() => {
      result.current.setFrames([makeFrame(0, "I", 1000)]);
    });

    await waitFor(() => {
      expect(result.current.getFrameStats().totalFrames).toBe(1);
    });

    act(() => {
      result.current.setFrames([
        makeFrame(0, "I", 1000),
        makeFrame(1, "P", 2000),
        makeFrame(2, "B", 3000),
      ]);
    });

    await waitFor(() => {
      expect(result.current.getFrameStats().totalFrames).toBe(3);
    });
  });
});

describe("FrameDataContext worker usage for large arrays", () => {
  beforeEach(() => {
    setupWorkerMock(true /* autoReply */);
  });

  afterEach(() => {
    restoreWorker();
    vi.restoreAllMocks();
  });

  it("creates a Worker for arrays >= 100 frames", async () => {
    const { result } = renderHook(() => useFrameData(), { wrapper });

    // Build an array of exactly 100 frames to exceed the WORKER_THRESHOLD
    const frames = Array.from({ length: 100 }, (_, i) =>
      makeFrame(i, "P", 1000),
    );

    act(() => {
      result.current.setFrames(frames);
    });

    await waitFor(() => {
      expect(global.Worker).toHaveBeenCalled();
    });
  });

  it("does NOT create a Worker for arrays below the threshold", async () => {
    const { result } = renderHook(() => useFrameData(), { wrapper });

    const frames = Array.from({ length: 10 }, (_, i) =>
      makeFrame(i, "P", 1000),
    );

    act(() => {
      result.current.setFrames(frames);
    });

    await waitFor(() => {
      expect(result.current.getFrameStats().totalFrames).toBe(10);
    });

    expect(global.Worker).not.toHaveBeenCalled();
  });

  it("worker.terminate() is called when component unmounts with active worker", async () => {
    const { result, unmount } = renderHook(() => useFrameData(), { wrapper });

    const frames = Array.from({ length: 100 }, (_, i) =>
      makeFrame(i, "P", 1000),
    );

    act(() => {
      result.current.setFrames(frames);
    });

    await waitFor(() => {
      expect(workerInstances.length).toBeGreaterThan(0);
    });

    unmount();

    // The cleanup effect should have called terminate on the worker
    const lastWorker = workerInstances[workerInstances.length - 1];
    expect(lastWorker.terminate).toHaveBeenCalled();
  });
});

describe("FrameDataContext worker error fallback", () => {
  beforeEach(() => {
    // Set up worker mock that fires onerror synchronously on postMessage
    workerInstances = [];

    global.Worker = vi.fn().mockImplementation(() => {
      const instance: MockWorkerInstance = {
        postMessage: vi.fn(() => {
          if (instance.onerror) {
            instance.onerror(
              new ErrorEvent("error", { message: "Worker crash" }),
            );
          }
        }),
        terminate: vi.fn(),
        onmessage: null,
        onerror: null,
      };
      workerInstances.push(instance);
      return instance;
    }) as unknown as typeof Worker;
  });

  afterEach(() => {
    restoreWorker();
    vi.restoreAllMocks();
  });

  it("falls back to sync calculation when worker fires onerror", async () => {
    const { result } = renderHook(() => useFrameData(), { wrapper });

    const frames = Array.from({ length: 100 }, (_, i) =>
      makeFrame(i, i % 3 === 0 ? "I" : "P", 1000, i % 3 === 0),
    );

    act(() => {
      result.current.setFrames(frames);
    });

    // Even though the worker errored, sync fallback should have calculated stats
    await waitFor(() => {
      expect(result.current.getFrameStats().totalFrames).toBe(100);
    });
  });
});

describe("FrameDataContext useFrameData hook", () => {
  it("throws when used outside of FrameDataProvider", () => {
    // Suppress the React error boundary console output
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});

    expect(() => {
      renderHook(() => useFrameData());
    }).toThrow("useFrameData must be used within a FrameDataProvider");

    spy.mockRestore();
  });
});
