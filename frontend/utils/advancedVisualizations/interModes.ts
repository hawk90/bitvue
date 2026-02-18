/**
 * Advanced Visualization Renderers - Inter Prediction Modes
 *
 * F12: Inter Prediction Modes visualization
 */

import type { MVGrid } from "../../../types/video";
import { InterMode } from "./types";

/**
 * Get inter mode color
 */
export function getInterModeColor(mode: InterMode): string {
  switch (mode) {
    case InterMode.Skip:
      return "#4caf50"; // Green
    case InterMode.Merge:
      return "#2196f3"; // Blue
    case InterMode.MotionVector:
      return "#ff9800"; // Orange
    case InterMode.Intra:
      return "#f44336"; // Red
    default:
      return "#666666";
  }
}

/**
 * Render inter prediction mode overlay
 */
export function renderInterModes(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  mvGrid: MVGrid,
  blockSize: number = 16,
): void {
  const { grid_w, grid_h, mv_l0, mode } = mvGrid;
  const blockW = width / grid_w;
  const blockH = height / grid_h;

  for (let y = 0; y < grid_h; y++) {
    for (let x = 0; x < grid_w; x++) {
      const idx = y * grid_w + x;

      // Determine mode from MV data
      let interMode = InterMode.MotionVector;

      if (mode && mode[idx] !== undefined) {
        interMode = mode[idx];
      } else if (mv_l0[idx].dx_qpel === 0 && mv_l0[idx].dy_qpel === 0) {
        // Check if this is a skip or merge block
        interMode = InterMode.Merge;
      }

      const bx = Math.floor(x * blockW);
      const by = Math.floor(y * blockH);
      const bw = Math.ceil(blockW);
      const bh = Math.ceil(blockH);

      ctx.fillStyle = getInterModeColor(interMode);
      ctx.fillRect(bx, by, bw, bh);

      // Draw border
      ctx.strokeStyle = "rgba(0, 0, 0, 0.3)";
      ctx.lineWidth = 1;
      ctx.strokeRect(bx, by, bw, bh);
    }
  }
}

export { InterMode };
