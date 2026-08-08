/**
 * FileStateContext Tests
 *
 * Covers the 2026-08-08 rewiring of refreshFrames/loadMoreFrames to electronBridgeService
 * (bitvue-indexer's index_stream/get_frames_chunk) instead of Tauri's invoke("get_frames_chunk").
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { FrameDataProvider, useFrameData } from "@/contexts/FrameDataContext";
import { FileStateProvider, useFileState } from "@/contexts/FileStateContext";
import type {
  BridgeUnitNode,
  FramesChunkResult,
} from "@/services/electronBridgeService";

const { indexStream, getFramesChunk } = vi.hoisted(() => ({
  indexStream: vi.fn(),
  getFramesChunk: vi.fn(),
}));

vi.mock("@/services/electronBridgeService", () => ({
  indexStream,
  getFramesChunk,
}));

function wrapper({ children }: { children: ReactNode }) {
  return (
    <FrameDataProvider>
      <FileStateProvider>{children}</FileStateProvider>
    </FrameDataProvider>
  );
}

function unit(overrides: Partial<BridgeUnitNode>): BridgeUnitNode {
  return {
    key: null,
    unit_type: "FRAME",
    offset: 0,
    size: 100,
    frame_index: 0,
    frame_type: "P",
    pts: null,
    dts: null,
    display_name: "frame",
    children: [],
    qp_avg: null,
    mv_grid: null,
    temporal_id: null,
    ref_frames: null,
    ref_slots: null,
    ...overrides,
  };
}

function chunk(
  units: BridgeUnitNode[],
  total_count: number,
  indexed = true,
): FramesChunkResult {
  return { indexed, units, total_count };
}

describe("FileStateContext", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    indexStream.mockResolvedValue([{ type: "ModelUpdated" }]);
  });

  it("refreshFrames indexes the stream, then fetches and maps all frames in one page", async () => {
    getFramesChunk.mockResolvedValueOnce(
      chunk(
        [
          unit({ frame_index: 0, frame_type: "I" }),
          unit({ frame_index: 1, frame_type: "P" }),
        ],
        2,
      ),
    );

    const { result } = renderHook(
      () => ({ file: useFileState(), data: useFrameData() }),
      { wrapper },
    );

    await act(async () => {
      await result.current.file.refreshFrames();
    });

    expect(indexStream).toHaveBeenCalledWith("A");
    expect(getFramesChunk).toHaveBeenCalledWith("A", 0, 100);
    await waitFor(() => expect(result.current.data.frames).toHaveLength(2));
    expect(result.current.data.frames[0].key_frame).toBe(true);
    expect(result.current.data.frames[1].key_frame).toBe(false);
    expect(result.current.file.totalFrames).toBe(2);
    expect(result.current.file.hasMoreFrames).toBe(false);
  });

  it("refreshFrames pages through multiple chunks until total_count is reached", async () => {
    const page1 = Array.from({ length: 100 }, (_, i) =>
      unit({ frame_index: i }),
    );
    const page2 = Array.from({ length: 20 }, (_, i) =>
      unit({ frame_index: 100 + i }),
    );
    getFramesChunk
      .mockResolvedValueOnce(chunk(page1, 120))
      .mockResolvedValueOnce(chunk(page2, 120));

    const { result } = renderHook(
      () => ({ file: useFileState(), data: useFrameData() }),
      { wrapper },
    );

    await act(async () => {
      await result.current.file.refreshFrames();
    });

    expect(getFramesChunk).toHaveBeenNthCalledWith(1, "A", 0, 100);
    expect(getFramesChunk).toHaveBeenNthCalledWith(2, "A", 100, 100);
    await waitFor(() => expect(result.current.data.frames).toHaveLength(120));
  });

  it("does not fabricate fields the sidecar doesn't provide", async () => {
    getFramesChunk.mockResolvedValueOnce(
      chunk(
        [
          unit({
            frame_index: 0,
            frame_type: "I",
            pts: 12345,
            ref_frames: [0],
          }),
        ],
        1,
      ),
    );

    const { result } = renderHook(
      () => ({ file: useFileState(), data: useFrameData() }),
      { wrapper },
    );
    await act(async () => {
      await result.current.file.refreshFrames();
    });

    await waitFor(() => expect(result.current.data.frames).toHaveLength(1));
    const frame = result.current.data.frames[0];
    expect(frame.pts).toBe(12345);
    expect(frame.ref_frames).toEqual([0]);
    expect(frame.poc).toBeUndefined();
    expect(frame.thumbnail).toBeUndefined();
  });

  it("surfaces an error via `error` when the bridge call rejects (e.g. non-IVF stream)", async () => {
    getFramesChunk.mockRejectedValueOnce(
      new Error("window.bitvue is unavailable"),
    );

    const { result } = renderHook(() => useFileState(), { wrapper });
    await act(async () => {
      await result.current.refreshFrames();
    });

    expect(result.current.error).toContain("window.bitvue is unavailable");
  });

  it("loadMoreFrames is a no-op when hasMoreFrames is false", async () => {
    // refreshFrames unconditionally resets hasMoreFrames to false once its own internal loop
    // finishes (pre-existing behavior, unrelated to this migration -- it already eagerly pages
    // through everything itself). That means loadMoreFrames's guard is the only thing reachable
    // through this context's public API in practice; verify it actually guards.
    getFramesChunk.mockResolvedValueOnce(chunk([unit({ frame_index: 0 })], 1));

    const { result } = renderHook(() => useFileState(), { wrapper });
    await act(async () => {
      await result.current.refreshFrames();
    });
    expect(result.current.hasMoreFrames).toBe(false);

    getFramesChunk.mockClear();
    let more: unknown[] = [];
    await act(async () => {
      more = await result.current.loadMoreFrames();
    });

    expect(getFramesChunk).not.toHaveBeenCalled();
    expect(more).toEqual([]);
  });
});
