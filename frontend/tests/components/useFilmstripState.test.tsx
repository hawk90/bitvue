/**
 * useFilmstripState Tests
 *
 * Covers the 2026-08-08 rewiring off Tauri's get_thumbnails invoke() to the Electron bridge's
 * getThumbnails (bitvue-sidecar's decode_bridge::get_thumbnails). No test previously exercised
 * this hook's real logic -- Filmstrip.test.tsx mocks the whole hook away.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { useFilmstripState } from "@/components/useFilmstripState";
import type { FrameInfo } from "@/types/video";
import type { BridgeThumbnailResult } from "@/services/electronBridgeService";

const { getThumbnails } = vi.hoisted(() => ({
  getThumbnails: vi.fn(),
}));

vi.mock("@/services/electronBridgeService", () => ({
  getThumbnails,
}));

function frame(index: number): FrameInfo {
  return { frame_index: index, frame_type: "P", size: 1000 };
}

function thumb(index: number): BridgeThumbnailResult {
  return {
    frame_index: index,
    thumbnail_data: `data:image/png;base64,frame${index}`,
    width: 120,
    height: 68,
    success: true,
  };
}

describe("useFilmstripState", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("auto-loads thumbnails for visible frames via the Electron bridge, stream A", async () => {
    getThumbnails.mockResolvedValue([thumb(0), thumb(1)]);
    const frames = [frame(0), frame(1)];

    const { result } = renderHook(() =>
      useFilmstripState({ frames, displayView: "thumbnails" }),
    );

    await waitFor(() =>
      expect(getThumbnails).toHaveBeenCalledWith("A", [0, 1]),
    );
    await waitFor(() => expect(result.current.thumbnails.size).toBe(2));
    expect(result.current.thumbnails.get(0)).toBe(
      "data:image/png;base64,frame0",
    );
  });

  it("does not call the bridge when displayView is not thumbnails", () => {
    renderHook(() =>
      useFilmstripState({ frames: [frame(0)], displayView: "list" }),
    );
    expect(getThumbnails).not.toHaveBeenCalled();
  });

  it("does not re-request an already-loaded or in-flight index", async () => {
    let resolveFirst!: (v: BridgeThumbnailResult[]) => void;
    getThumbnails.mockReturnValueOnce(
      new Promise((resolve) => {
        resolveFirst = resolve;
      }),
    );

    const { result, rerender } = renderHook(
      ({ frames }: { frames: FrameInfo[] }) =>
        useFilmstripState({ frames, displayView: "thumbnails" }),
      { initialProps: { frames: [frame(0)] } },
    );

    await waitFor(() => expect(getThumbnails).toHaveBeenCalledTimes(1));
    expect(result.current.loadingThumbnails.has(0)).toBe(true);

    // Re-render with the same frame while the request is still in flight -- the effect
    // re-runs (thumbnails/frames identity may change) but must not issue a duplicate request.
    rerender({ frames: [frame(0)] });
    expect(getThumbnails).toHaveBeenCalledTimes(1);

    await act(async () => {
      resolveFirst([thumb(0)]);
    });
    await waitFor(() => expect(result.current.thumbnails.get(0)).toBeDefined());

    rerender({ frames: [frame(0)] });
    expect(getThumbnails).toHaveBeenCalledTimes(1);
  });

  it("clears the loading flag and logs, without crashing, when the bridge call rejects", async () => {
    getThumbnails.mockRejectedValue(new Error("window.bitvue is unavailable"));

    // A stable `frames` reference, matching how a real caller would memoize it -- a fresh
    // array literal per render would re-trigger the auto-load effect on every state change
    // this hook itself causes (loadingThumbnails), unrelated to what this test is checking.
    const frames = [frame(0)];
    const { result } = renderHook(() =>
      useFilmstripState({ frames, displayView: "thumbnails" }),
    );

    await waitFor(() => expect(getThumbnails).toHaveBeenCalled());
    await waitFor(() => expect(result.current.loadingThumbnails.size).toBe(0));
    expect(result.current.thumbnails.size).toBe(0);
  });

  it("caps a request at THUMBNAIL_BATCH_SIZE frames", async () => {
    getThumbnails.mockResolvedValue([]);
    const frames = Array.from({ length: 80 }, (_, i) => frame(i));

    renderHook(() => useFilmstripState({ frames, displayView: "thumbnails" }));

    await waitFor(() => expect(getThumbnails).toHaveBeenCalled());
    const requestedIndices = getThumbnails.mock.calls[0][1] as number[];
    expect(requestedIndices.length).toBe(50); // THUMBNAIL_BATCH_SIZE
  });
});
