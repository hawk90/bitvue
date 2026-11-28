import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  getAllGraphFrames,
  unitNodeToGraphFrame,
  GRAPH_FRAMES_CHUNK_SIZE,
} from "@/services/bridge/frameList";
import type { BridgeUnitNode } from "@/services/bridge/stream";

const { getFramesChunk } = vi.hoisted(() => ({ getFramesChunk: vi.fn() }));

vi.mock("@/services/bridge/stream", () => ({ getFramesChunk }));

const unit = (over: Partial<BridgeUnitNode> = {}): BridgeUnitNode => ({
  key: null,
  unit_type: "Frame",
  offset: 0,
  size: 100,
  frame_index: 0,
  frame_type: "I",
  pts: 0,
  dts: null,
  display_name: "Frame",
  children: [],
  qp_avg: null,
  mv_grid: null,
  temporal_id: null,
  ref_frames: null,
  ref_slots: null,
  ...over,
});

describe("unitNodeToGraphFrame", () => {
  it("keeps the fields the graph panels read, null where the sidecar said nothing", () => {
    expect(
      unitNodeToGraphFrame(
        unit({
          frame_index: 4,
          frame_type: "P",
          size: 321,
          pts: 8,
          ref_frames: [2, 3],
          temporal_id: 1,
          qp_avg: 90,
        }),
      ),
    ).toEqual({
      frame_index: 4,
      frame_type: "P",
      size: 321,
      pts: 8,
      ref_frames: [2, 3],
      temporal_id: 1,
      qp: 90,
    });
    expect(unitNodeToGraphFrame(unit({ pts: null }))?.pts).toBeNull();
    // A unit with no QP (a repeated frame) stays null, never 0
    expect(unitNodeToGraphFrame(unit({ qp_avg: null }))?.qp).toBeNull();
  });

  it("labels an unknown frame type '?' and drops a unit with no frame_index", () => {
    expect(unitNodeToGraphFrame(unit({ frame_type: null }))?.frame_type).toBe(
      "?",
    );
    expect(unitNodeToGraphFrame(unit({ frame_index: null }))).toBeNull();
  });
});

describe("getAllGraphFrames", () => {
  beforeEach(() => {
    getFramesChunk.mockReset();
  });

  it("is empty, after one call, for a stream that is not indexed", async () => {
    getFramesChunk.mockResolvedValue({
      indexed: false,
      units: [],
      total_count: 0,
    });
    expect(await getAllGraphFrames("A")).toEqual([]);
    expect(getFramesChunk).toHaveBeenCalledTimes(1);
  });

  it("walks chunks until total_count and keeps decode order", async () => {
    const total = GRAPH_FRAMES_CHUNK_SIZE * 2 + 7;
    const all = Array.from({ length: total }, (_, i) =>
      unit({ frame_index: i }),
    );
    getFramesChunk.mockImplementation(
      async (_s: string, offset: number, limit: number) => ({
        indexed: true,
        units: all.slice(offset, offset + limit),
        total_count: total,
      }),
    );
    const frames = await getAllGraphFrames("B");
    expect(frames.map((f) => f.frame_index)).toEqual(all.map((_, i) => i));
    expect(getFramesChunk).toHaveBeenCalledTimes(3);
    expect(getFramesChunk.mock.calls[0][0]).toBe("B");
  });

  it("stops on an empty chunk even if total_count claims more", async () => {
    getFramesChunk
      .mockResolvedValueOnce({
        indexed: true,
        units: [unit()],
        total_count: 5,
      })
      .mockResolvedValueOnce({ indexed: true, units: [], total_count: 5 });
    expect(await getAllGraphFrames("A")).toHaveLength(1);
    expect(getFramesChunk).toHaveBeenCalledTimes(2);
  });

  it("stops between chunks once aborted", async () => {
    const controller = new AbortController();
    getFramesChunk.mockImplementation(async () => {
      controller.abort();
      return { indexed: true, units: [unit()], total_count: 1000 };
    });
    await getAllGraphFrames("A", controller.signal);
    expect(getFramesChunk).toHaveBeenCalledTimes(1);
  });

  it("rejects when a chunk fails", async () => {
    getFramesChunk.mockRejectedValue(new Error("boom"));
    await expect(getAllGraphFrames("A")).rejects.toThrow("boom");
  });
});
