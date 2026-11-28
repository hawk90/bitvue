/**
 * JPEG XS Precinct Overlay Renderer
 *
 * F1 (JPEG XS): Visualises the precinct grid with per-precinct estimated
 * bit-cost heatmap. Each precinct is a fixed-size slice column × slice row
 * tile; colour intensity encodes the relative bit budget.
 *
 * ⚠ Bit-cost values are uniform-distribution estimates until entropy decoding
 *   is implemented.
 */

import type { OverlayRendererProps } from "../types";
import {
  drawCellGrid,
  drawGridLines,
  drawNoDataNotice,
} from "../utils/drawing";
import { getFrameExtra } from "../utils/helpers";

interface PrecinctMap {
  cols: number;
  rows: number;
  precinct_w: number;
  precinct_h: number;
  bits: number[];
}

/** Jet colormap: maps [0,1] to rgba. */
function jetColor(t: number, alpha = 0.65): string {
  const r = Math.round(
    255 * Math.min(1, Math.max(0, 1.5 - Math.abs(t * 4 - 3))),
  );
  const g = Math.round(
    255 * Math.min(1, Math.max(0, 1.5 - Math.abs(t * 4 - 2))),
  );
  const b = Math.round(
    255 * Math.min(1, Math.max(0, 1.5 - Math.abs(t * 4 - 1))),
  );
  return `rgba(${r},${g},${b},${alpha})`;
}

export function JpegXsPrecinctRenderer({
  ctx,
  width,
  height,
  frame,
}: OverlayRendererProps): void {
  const map = getFrameExtra<PrecinctMap>(frame, "precinct_map");

  if (!map?.bits?.length || map.cols === 0 || map.rows === 0) {
    const msg = "Precinct: no data";
    drawNoDataNotice(ctx, width, height, msg);
    return;
  }

  const { cols, rows, bits } = map;
  const maxBits = Math.max(...bits, 1);

  drawCellGrid(ctx, width, height, cols, rows, (_row, _col, idx) =>
    jetColor((bits[idx] ?? 0) / maxBits),
  );

  // ── Precinct grid lines ────────────────────────────────────────────────────
  drawGridLines(ctx, width, height, cols, rows, "rgba(255,255,255,0.20)");

  // ── Legend (colour bar) ────────────────────────────────────────────────────
  const barW = 12;
  const barH = 80;
  const barX = 10;
  const barY = 10;
  for (let py = 0; py < barH; py++) {
    const t = 1 - py / barH;
    ctx.fillStyle = jetColor(t, 1);
    ctx.fillRect(barX, barY + py, barW, 1);
  }
  ctx.strokeStyle = "rgba(255,255,255,0.50)";
  ctx.lineWidth = 0.5;
  ctx.strokeRect(barX, barY, barW, barH);
  ctx.fillStyle = "#ffffff";
  ctx.font = "9px sans-serif";
  ctx.fillText("Hi", barX + barW + 3, barY + 8);
  ctx.fillText("Lo", barX + barW + 3, barY + barH);

  // ── Footer ─────────────────────────────────────────────────────────────────
  ctx.fillStyle = "rgba(0,0,0,0.60)";
  ctx.fillRect(10, height - 22, 210, 16);
  ctx.fillStyle = "#aaddff";
  ctx.font = "9px sans-serif";
  ctx.fillText(`⚠ precinct proxy — ${cols}×${rows} precincts`, 14, height - 9);
}
