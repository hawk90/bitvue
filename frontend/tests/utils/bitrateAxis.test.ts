import { describe, it, expect } from "vitest";
import {
  axisTickPositions,
  formatAxisTick,
  orderFramesForAxis,
} from "@/utils/bitrateAxis";
import type { GraphFrame } from "@/services/electronBridgeService";

const frame = (frame_index: number, pts: number | null): GraphFrame => ({
  frame_index,
  frame_type: "P",
  size: 100,
  pts,
  ref_frames: null,
  temporal_id: null,
});

describe("orderFramesForAxis", () => {
  it("sorts decode order into display order by PTS", () => {
    // decode order I P B B : display order is I B B P
    const decode = [frame(0, 0), frame(1, 3), frame(2, 1), frame(3, 2)];
    const { frames, order } = orderFramesForAxis(decode);
    expect(order).toBe("display");
    expect(frames.map((f) => f.frame_index)).toEqual([0, 2, 3, 1]);
    // input is not mutated
    expect(decode.map((f) => f.frame_index)).toEqual([0, 1, 2, 3]);
  });

  it("keeps decode order when any PTS is missing", () => {
    const decode = [frame(0, 0), frame(1, null), frame(2, 1)];
    const { frames, order } = orderFramesForAxis(decode);
    expect(order).toBe("decode");
    expect(frames).toBe(decode);
  });

  it("keeps decode order when a PTS repeats", () => {
    const decode = [frame(0, 0), frame(1, 5), frame(2, 5)];
    expect(orderFramesForAxis(decode).order).toBe("decode");
  });

  it("handles an empty stream", () => {
    expect(orderFramesForAxis([])).toEqual({ frames: [], order: "display" });
  });
});

describe("axisTickPositions", () => {
  it("is empty for no bars and a single tick for one bar", () => {
    expect(axisTickPositions(0)).toEqual([]);
    expect(axisTickPositions(1)).toEqual([0]);
  });

  it("labels first and last bar and spaces the rest evenly", () => {
    expect(axisTickPositions(101)).toEqual([0, 25, 50, 75, 100]);
  });

  it("never repeats a position on a short stream", () => {
    expect(axisTickPositions(2)).toEqual([0, 1]);
    expect(axisTickPositions(3)).toEqual([0, 1, 2]);
  });
});

describe("formatAxisTick", () => {
  it("uses frame numbers without a frame rate", () => {
    expect(formatAxisTick(12, null)).toBe("#12");
    expect(formatAxisTick(12, undefined)).toBe("#12");
    expect(formatAxisTick(12, 0)).toBe("#12");
  });

  it("uses seconds with a positive frame rate", () => {
    expect(formatAxisTick(60, 30)).toBe("2.00s");
    expect(formatAxisTick(0, 24)).toBe("0.00s");
  });
});
