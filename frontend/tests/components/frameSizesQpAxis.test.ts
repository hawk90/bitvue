import { describe, it, expect } from "vitest";
import { qpAxisMax } from "../../components/Filmstrip/views/FrameSizesView";
import type { FrameInfo } from "../../types/video";

const f = (qp: number): FrameInfo =>
  ({
    frame_index: 0,
    frame_type: "P",
    size: 1000,
    qp_grid: { qp_min: qp },
  }) as unknown as FrameInfo;

describe("qpAxisMax", () => {
  it("keeps 0-51 for H.264/HEVC-range QPs", () => {
    expect(qpAxisMax([f(20), f(51)], 1000)).toBe(51);
  });
  it("widens to 255 when an AV1 QP exceeds 51", () => {
    expect(qpAxisMax([f(14), f(85), f(92)], 1000)).toBe(255);
  });
  it("defaults to 51 with no frames", () => {
    expect(qpAxisMax([], 1)).toBe(51);
  });
});
