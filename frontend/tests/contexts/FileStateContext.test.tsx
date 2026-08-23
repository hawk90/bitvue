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
  BridgeTimeline,
  FramesChunkResult,
  StreamInfoResult,
} from "@/services/electronBridgeService";

const { indexStream, getFramesChunk, getStreamInfo, getTimeline } = vi.hoisted(
  () => ({
    indexStream: vi.fn(),
    getFramesChunk: vi.fn(),
    getStreamInfo: vi.fn(),
    getTimeline: vi.fn(),
  }),
);

vi.mock("@/services/electronBridgeService", async (importOriginal) => {
  // extractDiagnostics is a pure function (no bridge dependency) -- use the real
  // implementation rather than re-mocking it, same reasoning as YuvViewerPanel.test.tsx's
  // Tauri-mock comment for keeping non-bridge logic real.
  const actual =
    await importOriginal<typeof import("@/services/electronBridgeService")>();
  return {
    ...actual,
    indexStream,
    getFramesChunk,
    getStreamInfo,
    getTimeline,
  };
});

function timeline(
  frames: BridgeTimeline["frames"],
  ptsQuality: BridgeTimeline["pts_quality"] = "Ok",
): BridgeTimeline {
  return {
    stream_id: "A",
    frames,
    current_frame: null,
    scrub_mode: "Idle",
    viewport: [0, 0],
    vertical_viewport: [0, 0],
    pts_quality: ptsQuality,
  };
}

function timelineFrame(
  overrides: Partial<BridgeTimeline["frames"][number]>,
): BridgeTimeline["frames"][number] {
  return {
    display_idx: 0,
    size_bytes: 100,
    frame_type: "P",
    marker: "None",
    pts: null,
    dts: null,
    is_selected: false,
    ...overrides,
  };
}

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
    getStreamInfo.mockResolvedValue({ indexed: false, container: null });
    getTimeline.mockResolvedValue(timeline([]));
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

  it("refreshFrames calls getStreamInfo after indexing and stores the real container width/height/codec/bitDepth -- SelectionInfoPanel's 'Video Properties' section used to have no real data source at all and always rendered hardcoded 1920x1080/AV1 placeholders", async () => {
    getFramesChunk.mockResolvedValueOnce(
      chunk([unit({ frame_index: 0, frame_type: "I" })], 1),
    );
    getStreamInfo.mockResolvedValueOnce({
      indexed: true,
      container: {
        format: "IVF",
        codec: "AV1",
        track_count: 1,
        duration_ms: 10000,
        bitrate_bps: 177000,
        width: 320,
        height: 240,
        bit_depth: 8,
      },
    } satisfies StreamInfoResult);

    const { result } = renderHook(
      () => ({ file: useFileState(), data: useFrameData() }),
      { wrapper },
    );

    await act(async () => {
      await result.current.file.refreshFrames();
    });

    expect(getStreamInfo).toHaveBeenCalledWith("A");
    await waitFor(() =>
      expect(result.current.data.streamInfo).toEqual({
        width: 320,
        height: 240,
        codec: "AV1",
        bitDepth: 8,
        // Merged in afterward by applyDisplayOrder once get_timeline resolves (EDGE-03) --
        // "Ok" here because beforeEach's default getTimeline mock returns pts_quality: "Ok".
        ptsQuality: "Ok",
      }),
    );
  });

  it("refreshFrames leaves streamInfo null when the stream isn't indexed yet (getStreamInfo returns indexed:false)", async () => {
    getFramesChunk.mockResolvedValueOnce(
      chunk([unit({ frame_index: 0, frame_type: "I" })], 1),
    );
    // beforeEach's default already returns {indexed: false, container: null}

    const { result } = renderHook(
      () => ({ file: useFileState(), data: useFrameData() }),
      { wrapper },
    );

    await act(async () => {
      await result.current.file.refreshFrames();
    });

    await waitFor(() => expect(result.current.data.frames).toHaveLength(1));
    expect(result.current.data.streamInfo).toBeNull();
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
            offset: 16147,
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
    // Real, correctly-derived field -- was silently dropped before FrameInfo grew an offset
    // field (StreamTreePanel.tsx was hardcoding 0 for every displayed frame as a result).
    expect(frame.offset).toBe(16147);
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

  // CTX-02 fix: display_order/coding_order were always left undefined (docs/PARITY_CHECKLIST.md
  // CTX-02) -- these cover the real PTS-join against get_timeline that now populates them.
  it("populates display_order/coding_order by joining frames against get_timeline's PTS-sorted display index (reordered stream: decode order I,P,B but display order I,B,P)", async () => {
    getFramesChunk.mockResolvedValueOnce(
      chunk(
        [
          unit({ frame_index: 0, frame_type: "I", pts: 0 }),
          unit({ frame_index: 1, frame_type: "P", pts: 2000 }),
          unit({ frame_index: 2, frame_type: "B", pts: 1000 }),
        ],
        3,
      ),
    );
    getTimeline.mockResolvedValueOnce(
      timeline([
        timelineFrame({ display_idx: 0, pts: 0 }),
        timelineFrame({ display_idx: 1, pts: 1000 }),
        timelineFrame({ display_idx: 2, pts: 2000 }),
      ]),
    );

    const { result } = renderHook(
      () => ({ file: useFileState(), data: useFrameData() }),
      { wrapper },
    );
    await act(async () => {
      await result.current.file.refreshFrames();
    });

    await waitFor(() => expect(result.current.data.frames).toHaveLength(3));
    const [frameI, frameP, frameB] = result.current.data.frames;
    // decode_order stays array position (backend frame_index) -- unchanged, still what every
    // getDecodedFrameYuv/getFrameAnalysis call keys on.
    expect(frameI.coding_order).toBe(0);
    expect(frameP.coding_order).toBe(1);
    expect(frameB.coding_order).toBe(2);
    // display_order is the real PTS-sorted position -- P (pts=2000) is last, B (pts=1000) is
    // before it, despite B being decoded after P.
    expect(frameI.display_order).toBe(0);
    expect(frameP.display_order).toBe(2);
    expect(frameB.display_order).toBe(1);
  });

  it("leaves display_order/coding_order undefined for a frame whose PTS is missing or duplicated in the timeline response, instead of fabricating a guess", async () => {
    getFramesChunk.mockResolvedValueOnce(
      chunk(
        [
          unit({ frame_index: 0, frame_type: "I", pts: 0 }),
          unit({ frame_index: 1, frame_type: "P", pts: 500 }),
          unit({ frame_index: 2, frame_type: "P", pts: null }),
        ],
        3,
      ),
    );
    // pts=500 appears twice in the timeline (ambiguous PTS, PtsQuality::Bad) -- must not guess.
    getTimeline.mockResolvedValueOnce(
      timeline([
        timelineFrame({ display_idx: 0, pts: 0 }),
        timelineFrame({ display_idx: 1, pts: 500 }),
        timelineFrame({ display_idx: 2, pts: 500 }),
      ]),
    );

    const { result } = renderHook(
      () => ({ file: useFileState(), data: useFrameData() }),
      { wrapper },
    );
    await act(async () => {
      await result.current.file.refreshFrames();
    });

    await waitFor(() => expect(result.current.data.frames).toHaveLength(3));
    const [frameI, frameAmbiguous, frameNoPts] = result.current.data.frames;
    expect(frameI.display_order).toBe(0);
    expect(frameAmbiguous.display_order).toBeUndefined();
    expect(frameAmbiguous.coding_order).toBeUndefined();
    expect(frameNoPts.display_order).toBeUndefined();
    expect(frameNoPts.coding_order).toBeUndefined();
  });

  it("does not fail refreshFrames when get_timeline rejects (e.g. non-AV1 codec) -- display_order stays undefined, decode-order frame data is unaffected", async () => {
    getFramesChunk.mockResolvedValueOnce(
      chunk([unit({ frame_index: 0, frame_type: "I", pts: 0 })], 1),
    );
    getTimeline.mockRejectedValueOnce(new Error("codec not supported"));

    const { result } = renderHook(
      () => ({ file: useFileState(), data: useFrameData() }),
      { wrapper },
    );
    await act(async () => {
      await result.current.file.refreshFrames();
    });

    await waitFor(() => expect(result.current.data.frames).toHaveLength(1));
    expect(result.current.file.error).toBeNull();
    expect(result.current.data.frames[0].display_order).toBeUndefined();
  });

  // EDGE-03: PtsQuality (Ok/Warn/Bad), a stream-wide verdict from the same get_timeline call
  // display_order/coding_order already join against, surfaced into FrameDataContext's streamInfo.
  it.each(["Ok", "Warn", "Bad"] as const)(
    "surfaces get_timeline's pts_quality (%s) into streamInfo.ptsQuality",
    async (ptsQuality) => {
      getFramesChunk.mockResolvedValueOnce(
        chunk([unit({ frame_index: 0, frame_type: "I", pts: 0 })], 1),
      );
      getStreamInfo.mockResolvedValueOnce({
        indexed: true,
        container: {
          format: "IVF",
          codec: "AV1",
          track_count: 1,
          duration_ms: 10000,
          bitrate_bps: 177000,
          width: 320,
          height: 240,
          bit_depth: 8,
        },
      } satisfies StreamInfoResult);
      getTimeline.mockResolvedValueOnce(
        timeline([timelineFrame({ display_idx: 0, pts: 0 })], ptsQuality),
      );

      const { result } = renderHook(
        () => ({ file: useFileState(), data: useFrameData() }),
        { wrapper },
      );
      await act(async () => {
        await result.current.file.refreshFrames();
      });

      await waitFor(() =>
        expect(result.current.data.streamInfo?.ptsQuality).toBe(ptsQuality),
      );
    },
  );

  it("leaves streamInfo.ptsQuality null when get_timeline rejects, without discarding the real container info", async () => {
    getFramesChunk.mockResolvedValueOnce(
      chunk([unit({ frame_index: 0, frame_type: "I", pts: 0 })], 1),
    );
    getStreamInfo.mockResolvedValueOnce({
      indexed: true,
      container: {
        format: "IVF",
        codec: "AV1",
        track_count: 1,
        duration_ms: 10000,
        bitrate_bps: 177000,
        width: 320,
        height: 240,
        bit_depth: 8,
      },
    } satisfies StreamInfoResult);
    getTimeline.mockRejectedValueOnce(new Error("codec not supported"));

    const { result } = renderHook(
      () => ({ file: useFileState(), data: useFrameData() }),
      { wrapper },
    );
    await act(async () => {
      await result.current.file.refreshFrames();
    });

    await waitFor(() =>
      expect(result.current.data.streamInfo).toEqual({
        width: 320,
        height: 240,
        codec: "AV1",
        bitDepth: 8,
        ptsQuality: null,
      }),
    );
  });
});
