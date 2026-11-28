/**
 * AVS3 CCSAO (Cross-Component SAO) Overlay Renderer
 *
 * F9 (AVS3): Visualizes Cross-Component Sample Adaptive Offset per CTU.
 *
 * Color scheme:
 *   CCSAO applied   → cyan-blue tinted  rgba(0, 180, 220, 0.55)
 *   CCSAO not applied → grey            rgba(100, 100, 100, 0.25)
 *
 * luma_code (0..=4) modulates blue intensity for additional visual detail.
 *
 * ⚠ Data is a picture-level heuristic proxy until full AEC decoding is added.
 */

import type { OverlayRendererProps } from "../types";
import {
  drawCellGrid,
  drawGridLines,
  drawNoDataNotice,
} from "../utils/drawing";
import { getFrameExtra } from "../utils/helpers";

interface CcsaoMapData {
  grid_w: number;
  grid_h: number;
  ctu_size: number;
  ccsao_applied: boolean[];
  luma_code: number[];
}

/** Map luma_code (0–4) to a cyan-to-blue gradient. */
function ccsaoColor(lumaCode: number): string {
  // luma_code 0 = deep blue, 4 = cyan
  const t = lumaCode / 4;
  const r = Math.round(0 + t * 0);
  const g = Math.round(100 + t * 80);
  const b = Math.round(220 - t * 20);
  return `rgba(${r},${g},${b},0.55)`;
}

export function Avs3CcsaoRenderer({
  ctx,
  width,
  height,
  frame,
}: OverlayRendererProps): void {
  const ccsaoMap = getFrameExtra<CcsaoMapData>(frame, "ccsao_map");
  if (!ccsaoMap?.ccsao_applied?.length) {
    const msg = "CCSAO: disabled or unavailable";
    drawNoDataNotice(ctx, width, height, msg);
    return;
  }

  const { grid_w, grid_h, ctu_size, ccsao_applied, luma_code } = ccsaoMap;

  drawCellGrid(ctx, width, height, grid_w, grid_h, (_row, _col, idx) =>
    (ccsao_applied[idx] ?? false)
      ? ccsaoColor(luma_code[idx] ?? 0)
      : "rgba(100,100,100,0.22)",
  );

  // ── CTU grid lines ────────────────────────────────────────────────────────
  drawGridLines(ctx, width, height, grid_w, grid_h, "rgba(255,255,255,0.15)");

  // ── Legend ────────────────────────────────────────────────────────────────
  const padX = 8;
  const padY = 8;
  const swatchSize = 10;
  const boxX = 10;
  const boxY = 10;
  const boxW = 130;
  const boxH = padY * 2 + 2 * 16;

  ctx.fillStyle = "rgba(0,0,0,0.68)";
  ctx.fillRect(boxX, boxY, boxW, boxH);
  ctx.font = "10px sans-serif";

  // Applied swatch
  ctx.fillStyle = ccsaoColor(2);
  ctx.fillRect(boxX + padX, boxY + padY, swatchSize, swatchSize);
  ctx.fillStyle = "#ffffff";
  ctx.fillText("CCSAO applied", boxX + padX + swatchSize + 5, boxY + padY + 9);

  // Not applied swatch
  ctx.fillStyle = "rgba(100,100,100,0.60)";
  ctx.fillRect(boxX + padX, boxY + padY + 16, swatchSize, swatchSize);
  ctx.fillStyle = "#aaaaaa";
  ctx.fillText("not applied", boxX + padX + swatchSize + 5, boxY + padY + 25);

  // ── Proxy warning ─────────────────────────────────────────────────────────
  ctx.fillStyle = "rgba(0,0,0,0.60)";
  ctx.fillRect(10, height - 22, 140, 16);
  ctx.fillStyle = "#aaddff";
  ctx.font = "9px sans-serif";
  ctx.fillText(`⚠ CCSAO proxy — CTU=${ctu_size}px`, 14, height - 9);
}
