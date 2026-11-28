/**
 * AVS3 ESAO (Enhanced SAO) Overlay Renderer
 *
 * F8 (AVS3): Visualizes Enhanced Sample Adaptive Offset per CTU.
 *
 * ESAO type → color:
 *   0 (OFF)      → transparent / no fill
 *   1 (EO_0)     → red-ish     rgba(220, 50, 50, 0.55)   horizontal edge
 *   2 (EO_90)    → blue        rgba(50, 100, 220, 0.55)  vertical edge
 *   3 (EO_135)   → green       rgba(40, 180, 80, 0.55)   diagonal ↘
 *   4 (EO_45)    → yellow      rgba(220, 200, 0, 0.55)   diagonal ↗
 *   5 (BO)       → purple      rgba(160, 50, 220, 0.55)  band offset
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

interface EsaoMapData {
  grid_w: number;
  grid_h: number;
  ctu_size: number;
  esao_type: number[];
  esao_class: number[];
}

const ESAO_COLORS: Record<number, string> = {
  0: "rgba(0,0,0,0)", // OFF — transparent
  1: "rgba(220, 50,  50,  0.55)", // EO_0   horizontal
  2: "rgba( 50,100, 220,  0.55)", // EO_90  vertical
  3: "rgba( 40,180,  80,  0.55)", // EO_135 diagonal ↘
  4: "rgba(220,200,   0,  0.55)", // EO_45  diagonal ↗
  5: "rgba(160, 50, 220,  0.55)", // BO     band offset
};

const ESAO_LABELS: Record<number, string> = {
  0: "OFF",
  1: "EO_0",
  2: "EO_90",
  3: "EO_135",
  4: "EO_45",
  5: "BO",
};

const LEGEND_ITEMS = Object.entries(ESAO_LABELS)
  .filter(([k]) => Number(k) !== 0)
  .map(([k, label]) => ({
    color: ESAO_COLORS[Number(k)],
    label,
  }));

export function Avs3EsaoRenderer({
  ctx,
  width,
  height,
  frame,
}: OverlayRendererProps): void {
  const esaoMap = getFrameExtra<EsaoMapData>(frame, "esao_map");
  if (!esaoMap?.esao_type?.length) {
    // No ESAO data — show "disabled" notice
    const msg = "ESAO: disabled or unavailable";
    drawNoDataNotice(ctx, width, height, msg);
    return;
  }

  const { grid_w, grid_h, ctu_size, esao_type } = esaoMap;

  drawCellGrid(ctx, width, height, grid_w, grid_h, (_row, _col, idx) => {
    const t = esao_type[idx] ?? 0;
    if (t === 0) return null; // OFF → skip
    return ESAO_COLORS[t] ?? ESAO_COLORS[0];
  });

  // ── CTU grid lines ────────────────────────────────────────────────────────
  drawGridLines(ctx, width, height, grid_w, grid_h, "rgba(255,255,255,0.18)");

  // ── Legend ────────────────────────────────────────────────────────────────
  const swatchSize = 10;
  const itemH = 16;
  const padX = 8;
  const padY = 8;
  const boxW = 100;
  const boxH = padY * 2 + LEGEND_ITEMS.length * itemH;
  const boxX = 10;
  const boxY = 10;

  ctx.fillStyle = "rgba(0,0,0,0.68)";
  ctx.fillRect(boxX, boxY, boxW, boxH);
  ctx.font = "10px sans-serif";

  LEGEND_ITEMS.forEach((item, i) => {
    const y = boxY + padY + i * itemH;
    ctx.fillStyle = item.color;
    ctx.fillRect(boxX + padX, y, swatchSize, swatchSize);
    ctx.fillStyle = "#ffffff";
    ctx.fillText(item.label, boxX + padX + swatchSize + 5, y + 9);
  });

  // ── CTU size note ─────────────────────────────────────────────────────────
  ctx.fillStyle = "rgba(0,0,0,0.60)";
  ctx.fillRect(10, height - 22, 130, 16);
  ctx.fillStyle = "#aaddff";
  ctx.font = "9px sans-serif";
  ctx.fillText(`⚠ ESAO proxy — CTU=${ctu_size}px`, 14, height - 9);
}
