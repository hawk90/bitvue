/**
 * AV1 Heat Map Renderer
 *
 * Info overlay: where the bits went. Each cell is colored by the entropy-coded bits per pixel the
 * encoder spent there, measured from the arithmetic decoder (`frame.bit_grid`, partition cost
 * included) -- not motion-vector magnitude (#458). Warmer = more bits per pixel.
 */

import type { OverlayRendererProps } from "../types";
import { jetColor } from "./Av1EfficiencyMapRenderer";

export function Av1HeatMapOverlay({
  ctx,
  width,
  height,
  frame,
}: OverlayRendererProps): void {
  const grid = frame.bit_grid;
  if (!grid) return;

  const { grid_w, grid_h, bits_per_pixel } = grid;
  const cellW = width / grid_w;
  const cellH = height / grid_h;
  let max = 0;
  for (const v of bits_per_pixel) if (v > max) max = v;
  const range = Math.max(1e-9, max);

  for (let row = 0; row < grid_h; row++) {
    for (let col = 0; col < grid_w; col++) {
      const v = bits_per_pixel[row * grid_w + col] ?? 0;
      if (v <= 0) continue; // nothing coded in this cell
      ctx.fillStyle = jetColor(v / range);
      ctx.fillRect(col * cellW, row * cellH, cellW, cellH);
    }
  }

  const barX = 10;
  const barY = height - 22;
  const barW = 160;
  const grad = ctx.createLinearGradient(barX, 0, barX + barW, 0);
  grad.addColorStop(0, "rgba(0,0,200,0.8)");
  grad.addColorStop(0.25, "rgba(0,200,200,0.8)");
  grad.addColorStop(0.5, "rgba(0,200,0,0.8)");
  grad.addColorStop(0.75, "rgba(200,200,0,0.8)");
  grad.addColorStop(1, "rgba(200,0,0,0.8)");
  ctx.fillStyle = grad;
  ctx.fillRect(barX, barY, barW, 8);

  ctx.fillStyle = "#ffffff";
  ctx.font = "9px sans-serif";
  ctx.fillText("0 bits/px", barX, barY - 2);
  ctx.fillText(`${max.toFixed(2)} bits/px`, barX + barW - 50, barY - 2);
}
