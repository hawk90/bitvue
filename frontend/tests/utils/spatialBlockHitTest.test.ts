/**
 * spatialBlockHitTest tests (INT-01: Player click→spatialBlock select)
 */

import { describe, it, expect } from "vitest";
import { resolveSpatialBlockAtPoint } from "@/utils/spatialBlockHitTest";
import type { FrameInfo } from "@/types/video";
import { PartitionType } from "@/types/video";

function frame(overrides: Partial<FrameInfo> = {}): FrameInfo {
  return {
    frame_index: 0,
    frame_type: "I",
    size: 1000,
    ...overrides,
  };
}

describe("resolveSpatialBlockAtPoint", () => {
  it("returns null when neither partition_grid nor qp_grid is present", () => {
    expect(resolveSpatialBlockAtPoint(10, 10, frame())).toBeNull();
  });

  it("returns null for negative coordinates", () => {
    const f = frame({
      qp_grid: {
        grid_w: 4,
        grid_h: 4,
        block_w: 16,
        block_h: 16,
        qp: new Array(16).fill(20),
        qp_min: 20,
        qp_max: 20,
      },
    });
    expect(resolveSpatialBlockAtPoint(-1, 10, f)).toBeNull();
    expect(resolveSpatialBlockAtPoint(10, -1, f)).toBeNull();
  });

  describe("partition_grid (variable-size leaf blocks)", () => {
    it("finds the leaf block containing the point", () => {
      const f = frame({
        partition_grid: {
          coded_width: 128,
          coded_height: 128,
          sb_size: 64,
          blocks: [
            {
              x: 0,
              y: 0,
              width: 32,
              height: 32,
              partition: PartitionType.None,
              depth: 1,
            },
            {
              x: 32,
              y: 0,
              width: 64,
              height: 64,
              partition: PartitionType.None,
              depth: 0,
            },
          ],
        },
      });

      expect(resolveSpatialBlockAtPoint(10, 10, f)).toEqual({
        x: 0,
        y: 0,
        w: 32,
        h: 32,
      });
      expect(resolveSpatialBlockAtPoint(50, 50, f)).toEqual({
        x: 32,
        y: 0,
        w: 64,
        h: 64,
      });
    });

    it("returns null when the point falls outside every leaf block", () => {
      const f = frame({
        partition_grid: {
          coded_width: 128,
          coded_height: 128,
          sb_size: 64,
          blocks: [
            {
              x: 0,
              y: 0,
              width: 32,
              height: 32,
              partition: PartitionType.None,
              depth: 1,
            },
          ],
        },
      });
      expect(resolveSpatialBlockAtPoint(100, 100, f)).toBeNull();
    });

    it("treats block bounds as [x, x+width) -- exclusive on the far edge", () => {
      const f = frame({
        partition_grid: {
          coded_width: 32,
          coded_height: 32,
          sb_size: 32,
          blocks: [
            {
              x: 0,
              y: 0,
              width: 32,
              height: 32,
              partition: PartitionType.None,
              depth: 0,
            },
          ],
        },
      });
      expect(resolveSpatialBlockAtPoint(31, 31, f)).not.toBeNull();
      expect(resolveSpatialBlockAtPoint(32, 32, f)).toBeNull();
    });

    it("is preferred over qp_grid when both are present", () => {
      const f = frame({
        partition_grid: {
          coded_width: 64,
          coded_height: 64,
          sb_size: 64,
          blocks: [
            {
              x: 0,
              y: 0,
              width: 64,
              height: 64,
              partition: PartitionType.None,
              depth: 0,
            },
          ],
        },
        qp_grid: {
          grid_w: 4,
          grid_h: 4,
          block_w: 16,
          block_h: 16,
          qp: new Array(16).fill(20),
          qp_min: 20,
          qp_max: 20,
        },
      });
      // A qp_grid-only lookup at (10,10) would return a 16x16 cell -- confirms partition_grid wins.
      expect(resolveSpatialBlockAtPoint(10, 10, f)).toEqual({
        x: 0,
        y: 0,
        w: 64,
        h: 64,
      });
    });
  });

  describe("qp_grid fallback (fixed-size grid)", () => {
    it("computes the fixed cell containing the point", () => {
      const f = frame({
        qp_grid: {
          grid_w: 4,
          grid_h: 4,
          block_w: 16,
          block_h: 16,
          qp: new Array(16).fill(20),
          qp_min: 20,
          qp_max: 20,
        },
      });
      expect(resolveSpatialBlockAtPoint(20, 5, f)).toEqual({
        x: 16,
        y: 0,
        w: 16,
        h: 16,
      });
    });

    it("returns null when the point is outside the grid's bounds", () => {
      const f = frame({
        qp_grid: {
          grid_w: 2,
          grid_h: 2,
          block_w: 16,
          block_h: 16,
          qp: [20, 20, 20, 20],
          qp_min: 20,
          qp_max: 20,
        },
      });
      expect(resolveSpatialBlockAtPoint(100, 100, f)).toBeNull();
    });

    it("returns null instead of dividing by zero when block_w/block_h are 0", () => {
      const f = frame({
        qp_grid: {
          grid_w: 0,
          grid_h: 0,
          block_w: 0,
          block_h: 0,
          qp: [],
          qp_min: 0,
          qp_max: 0,
        },
      });
      expect(resolveSpatialBlockAtPoint(10, 10, f)).toBeNull();
    });
  });
});
