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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const esaoMap = (frame as any).esao_map as EsaoMapData | undefined;
  if (!esaoMap?.esao_type?.length) {
    // No ESAO data — show "disabled" notice
    const msg = "ESAO: disabled or unavailable";
    ctx.font = "13px monospace";
    const bw = ctx.measureText(msg).width + 24;
    const bh = 30;
    const bx = (width - bw) / 2;
    const by = (height - bh) / 2;
    ctx.fillStyle = "rgba(0,0,0,0.70)";
    ctx.fillRect(bx, by, bw, bh);
    ctx.fillStyle = "#888888";
    ctx.fillText(msg, bx + 12, by + 20);
    return;
  }

  const { grid_w, grid_h, ctu_size, esao_type } = esaoMap;
  const cellW = width / grid_w;
  const cellH = height / grid_h;

  for (let row = 0; row < grid_h; row++) {
    for (let col = 0; col < grid_w; col++) {
      const idx = row * grid_w + col;
      const t = esao_type[idx] ?? 0;
      const color = ESAO_COLORS[t] ?? ESAO_COLORS[0];
      if (t === 0) continue; // OFF → skip
      ctx.fillStyle = color;
      ctx.fillRect(col * cellW, row * cellH, cellW, cellH);
    }
  }

  // ── CTU grid lines ────────────────────────────────────────────────────────
  ctx.strokeStyle = "rgba(255,255,255,0.18)";
  ctx.lineWidth = 0.5;
  for (let col = 0; col <= grid_w; col++) {
    ctx.beginPath();
    ctx.moveTo(col * cellW, 0);
    ctx.lineTo(col * cellW, height);
    ctx.stroke();
  }
  for (let row = 0; row <= grid_h; row++) {
    ctx.beginPath();
    ctx.moveTo(0, row * cellH);
    ctx.lineTo(width, row * cellH);
    ctx.stroke();
  }

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
