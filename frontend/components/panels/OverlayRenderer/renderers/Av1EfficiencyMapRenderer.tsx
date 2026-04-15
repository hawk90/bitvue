/**
 * AV1 / VP9 Efficiency Map Renderer
 *
 * Info overlay: bits-per-pixel efficiency heatmap.
 *
 * Each CTU-sized cell is colored by the ratio of its size (in bits) to its
 * pixel area. Higher bits-per-pixel = less efficient encoding = warmer color.
 *
 * Color scale (VQ Analyzer parity, jet-like):
 *   0 bpp (fully skipped) → deep blue  (#0000CC)
 *   1 bpp (normal)        → green      (#00CC00)
 *   ≥3 bpp (complex)      → red        (#CC0000)
 *
 * The QP grid is used as a proxy: low QP → more bits allocated → higher bpp.
 * Blocks estimated from: bpp ≈ (64 - QP) * scale_factor.
 */

import type { OverlayRendererProps } from "../types";

/** Jet-like colormap: t ∈ [0,1] → rgba string */
function jetColor(t: number, alpha = 0.55): string {
  const clamp = (v: number) => Math.max(0, Math.min(255, Math.round(v)));
  const r = clamp(t < 0.5 ? 0 : t < 0.75 ? (t - 0.5) * 4 * 255 : 255);
  const g = clamp(t < 0.25 ? t * 4 * 255 : t < 0.75 ? 255 : (1 - t) * 4 * 255);
  const b = clamp(t < 0.25 ? 255 : t < 0.5 ? (0.5 - t) * 4 * 255 : 0);
  return `rgba(${r},${g},${b},${alpha})`;
}

export function Av1EfficiencyMapOverlay({
  ctx,
  width,
  height,
  frame,
}: OverlayRendererProps): void {
  const qpGrid = frame.qp_grid;
  if (!qpGrid) return; // no-op if no data

  const { grid_w, grid_h, qp } = qpGrid;
  const cellW = width / grid_w;
  const cellH = height / grid_h;

  // Find QP range for normalization
  let minQp = 63,
    maxQp = 0;
  for (const v of qp) {
    if (v < minQp) minQp = v;
    if (v > maxQp) maxQp = v;
  }
  const range = Math.max(1, maxQp - minQp);

  for (let row = 0; row < grid_h; row++) {
    for (let col = 0; col < grid_w; col++) {
      const idx = row * grid_w + col;
      const blockQp = qp[idx] ?? 32;
      // Lower QP → more bits → higher t (warmer color = less efficient)
      // We invert: low QP = high "bit cost" proxy
      const t = 1 - (blockQp - minQp) / range;
      ctx.fillStyle = jetColor(t);
      ctx.fillRect(col * cellW, row * cellH, cellW, cellH);
    }
  }

  // ── Colorbar legend ───────────────────────────────────────────────────────
  const barX = 10;
  const barY = height - 22;
  const barW = 160;
  const barH = 8;

  const grad = ctx.createLinearGradient(barX, 0, barX + barW, 0);
  grad.addColorStop(0, "rgba(0,0,200,0.8)");
  grad.addColorStop(0.25, "rgba(0,200,200,0.8)");
  grad.addColorStop(0.5, "rgba(0,200,0,0.8)");
  grad.addColorStop(0.75, "rgba(200,200,0,0.8)");
  grad.addColorStop(1, "rgba(200,0,0,0.8)");
  ctx.fillStyle = grad;
  ctx.fillRect(barX, barY, barW, barH);

  ctx.fillStyle = "#ffffff";
  ctx.font = "9px sans-serif";
  ctx.fillText("low bpp", barX, barY - 2);
  ctx.fillText("high bpp", barX + barW - 40, barY - 2);
}
