/**
 * VC-3 / DNxHD Segment Overlay Renderer
 *
 * F1 (VC-3): Macroblock grid with uniform bit-cost heatmap.
 * Each 16×16 macroblock cell is coloured by estimated bit cost (currently
 * uniform across the frame).
 *
 * CompId is shown in the legend box together with:
 *   - chroma sampling (4:2:0 / 4:2:2 / 4:4:4)
 *   - bits per component
 *   - DNxHD vs DNxHR variant
 */

import type { OverlayRendererProps } from "../types";
import {
  drawCellGrid,
  drawGridLines,
  drawNoDataNotice,
} from "../utils/drawing";

interface MbGrid {
  cols: number;
  rows: number;
  bits: number[];
}

interface Vc3FrameData {
  mb_grid?: MbGrid;
  comp_id?: string;
  chroma_sampling?: number;
  bits_per_component?: number;
  is_dnxhr?: boolean;
}

/** Uniform orange tint for VC-3 macroblocks. */
function mbColor(normalised: number): string {
  const r = Math.round(180 + normalised * 75);
  const g = Math.round(80 + normalised * 80);
  const b = Math.round(20 + normalised * 30);
  return `rgba(${r},${g},${b},0.55)`;
}

export function Vc3SegmentRenderer({
  ctx,
  width,
  height,
  frame,
}: OverlayRendererProps): void {
  const vc3 = frame as unknown as Vc3FrameData;
  const grid = vc3.mb_grid as MbGrid | undefined;

  if (!grid?.bits?.length || grid.cols === 0 || grid.rows === 0) {
    const msg = "VC-3: no MB grid data";
    drawNoDataNotice(ctx, width, height, msg);
    return;
  }

  const { cols, rows, bits } = grid;
  const maxBits = Math.max(...bits, 1);

  drawCellGrid(ctx, width, height, cols, rows, (_row, _col, idx) =>
    mbColor((bits[idx] ?? 0) / maxBits),
  );

  // ── MB grid lines ─────────────────────────────────────────────────────────
  drawGridLines(ctx, width, height, cols, rows, "rgba(255,255,255,0.15)");

  // ── Info box ───────────────────────────────────────────────────────────────
  const compId = vc3.comp_id ?? "DNxHD";
  const chroma = vc3.chroma_sampling ?? 422;
  const bpc = vc3.bits_per_component ?? 8;
  const variant = vc3.is_dnxhr ? "DNxHR" : "DNxHD";

  const lines = [
    variant,
    compId,
    `Chroma: ${chroma}`,
    `Depth: ${bpc}-bit`,
    `MBs: ${cols}×${rows}`,
  ];
  const boxW = 130;
  const boxH = 10 + lines.length * 14;
  const boxX = 10;
  const boxY = 10;

  ctx.fillStyle = "rgba(0,0,0,0.68)";
  ctx.fillRect(boxX, boxY, boxW, boxH);
  ctx.font = "10px sans-serif";
  lines.forEach((line, i) => {
    ctx.fillStyle = i === 0 ? "#ffaa44" : "#ffffff";
    ctx.fillText(line, boxX + 8, boxY + 14 + i * 14);
  });

  // ── Footer ─────────────────────────────────────────────────────────────────
  ctx.fillStyle = "rgba(0,0,0,0.60)";
  ctx.fillRect(10, height - 22, 190, 16);
  ctx.fillStyle = "#aaddff";
  ctx.font = "9px sans-serif";
  ctx.fillText("⚠ MB grid — uniform bit estimate", 14, height - 9);
}
