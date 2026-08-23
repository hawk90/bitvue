/**
 * Spatial block hit-testing (INT-01: Player click→spatialBlock select).
 *
 * Finds the coding-unit block at a given frame-native pixel coordinate. Pure and DOM-free so it
 * can be unit-tested without a real canvas -- `VideoCanvas.tsx` handles the client-coordinate ->
 * frame-coordinate conversion (via `getBoundingClientRect()`) separately and passes plain numbers
 * in here.
 */

import type { FrameInfo } from "../types/video";

export interface SpatialBlockRect {
  x: number;
  y: number;
  w: number;
  h: number;
}

/**
 * Prefers `partition_grid`'s real leaf blocks (variable size -- the true partition-tree leaves)
 * over the fixed-size `qp_grid`, since only the former reflects actual block boundaries when
 * partitions aren't uniform (which is the common case). Falls back to `qp_grid`'s fixed grid when
 * no partition data is available for this frame/mode. Returns `null` (never a fabricated guess)
 * when neither grid is available or the point falls outside both.
 */
export function resolveSpatialBlockAtPoint(
  frameX: number,
  frameY: number,
  frame: FrameInfo,
): SpatialBlockRect | null {
  if (frameX < 0 || frameY < 0) return null;

  const partitionGrid = frame.partition_grid;
  if (partitionGrid) {
    const block = partitionGrid.blocks.find(
      (b) =>
        frameX >= b.x &&
        frameX < b.x + b.width &&
        frameY >= b.y &&
        frameY < b.y + b.height,
    );
    if (block) {
      return { x: block.x, y: block.y, w: block.width, h: block.height };
    }
  }

  const qpGrid = frame.qp_grid;
  if (qpGrid && qpGrid.block_w > 0 && qpGrid.block_h > 0) {
    const col = Math.floor(frameX / qpGrid.block_w);
    const row = Math.floor(frameY / qpGrid.block_h);
    if (col < 0 || col >= qpGrid.grid_w || row < 0 || row >= qpGrid.grid_h) {
      return null;
    }
    return {
      x: col * qpGrid.block_w,
      y: row * qpGrid.block_h,
      w: qpGrid.block_w,
      h: qpGrid.block_h,
    };
  }

  return null;
}
