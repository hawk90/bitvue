/**
 * Advanced Visualization Renderers - Intra Prediction Modes
 *
 * F11: Intra Prediction Modes visualization
 */

import type { PredictionModeGrid } from "../../../types/video";

/**
 * Color maps for intra prediction modes
 * Based on HEVC/VVC intra mode angles
 */
export const INTRA_MODE_COLORS: Record<number, string> = [
  "#000000", // 0: Planar
  "#ff0000", // 1: DC
  "#00ff00", // 2: Angular 2
  "#0000ff", // 3: Angular 3
  "#ffff00", // 4: Angular 4
  "#ff00ff", // 5: Angular 5
  "#00ffff", // 6: Angular 6
  "#ff8000", // 7: Angular 7
  "#8000ff", // 8: Angular 8
  "#ff0080", // 9: Angular 9
  "#80ff00", // 10: Angular 10
  "#0080ff", // 11: Angular 11
  "#80ff80", // 12: Angular 12
  "#ff8080", // 13: Angular 13
  "#80ffff", // 14: Angular 14
  "#ffff80", // 15: Angular 15
  "#ff80ff", // 16: Angular 16
  "#80ff00", // 17: Angular 17
  "#00ff80", // 18: Angular 18
  "#0080ff", // 19: Angular 19
  "#ff8000", // 20: Angular 20
  "#8000ff", // 21: Angular 21
  "#ff0080", // 22: Angular 22
  "#80ff80", // 23: Angular 23
  "#80ff00", // 24: Angular 24
  "#00ff80", // 25: Angular 25
  "#008080", // 26: Angular 26
  "#808000", // 27: Angular 27
  "#008080", // 28: Angular 28
  "#800080", // 29: Angular 29
  "#808080", // 30: Angular 30
  "#ff8080", // 31: Angular 31
  "#80ff80", // 32: Angular 32
  "#8080ff", // 33: Angular 33
  "#ff80ff", // 34: Angular 34
];

/**
 * Get intra mode color
 */
export function getIntraModeColor(mode: number | null): string {
  if (mode === null || mode < 0 || mode >= INTRA_MODE_COLORS.length) {
    return "#333333";
  }
  return INTRA_MODE_COLORS[mode];
}

/**
 * Render intra prediction mode overlay
 */
export function renderIntraModes(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  grid: PredictionModeGrid,
  blockSize: number = 16,
): void {
  const { grid_w, grid_h, modes } = grid;
  const blockW = width / grid_w;
  const blockH = height / grid_h;

  for (let y = 0; y < grid_h; y++) {
    for (let x = 0; x < grid_w; x++) {
      const idx = y * grid_w + x;
      const mode = modes[idx];

      if (mode !== null) {
        const bx = Math.floor(x * blockW);
        const by = Math.floor(y * blockH);
        const bw = Math.ceil(blockW);
        const bh = Math.ceil(blockH);

        ctx.fillStyle = getIntraModeColor(mode);
        ctx.fillRect(bx, by, bw, bh);

        // Draw mode number for larger blocks
        if (bw >= 32 && bh >= 16) {
          ctx.fillStyle = mode > 20 ? "#000" : "#fff";
          ctx.font = "10px monospace";
          ctx.fillText(mode.toString(), bx + 2, by + 12);
        }
      }
    }
  }
}
