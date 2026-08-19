/**
 * JPEG XS Dequant Overlay Renderer
 *
 * F2 (JPEG XS): Wavelet sub-band energy heatmap.
 * Each cell corresponds to one wavelet sub-band (LL, LH1..LHn, HL1..HLn,
 * HH1..HHn). Colour intensity encodes normalised coefficient energy.
 *
 * ⚠ Energy values are structure estimates; full entropy decoding needed for
 *   per-coefficient accuracy.
 */

import type { OverlayRendererProps } from "../types";

interface DequantMap {
  grid_w: number;
  grid_h: number;
  energy: number[];
  labels: string[];
}

/** Energy colour: low=blue, mid=green, high=red. */
function energyColor(e: number): string {
  const r = Math.round(255 * Math.min(1, e * 2));
  const g = Math.round(
    255 * Math.min(1, Math.max(0, 1 - Math.abs(e - 0.5) * 4)),
  );
  const b = Math.round(255 * Math.min(1, Math.max(0, 1 - e * 2)));
  return `rgba(${r},${g},${b},0.70)`;
}

export function JpegXsDequantRenderer({
  ctx,
  width,
  height,
  frame,
}: OverlayRendererProps): void {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const map = (frame as any).dequant_map as DequantMap | undefined;

  if (!map?.energy?.length) {
    const msg = "Dequant: no sub-band data";
    ctx.font = "13px monospace";
    const bw = ctx.measureText(msg).width + 24;
    const bx = (width - bw) / 2;
    const by = (height - 30) / 2;
    ctx.fillStyle = "rgba(0,0,0,0.70)";
    ctx.fillRect(bx, by, bw, 30);
    ctx.fillStyle = "#888888";
    ctx.fillText(msg, bx + 12, by + 20);
    return;
  }

  const { grid_w: gw, grid_h: gh, energy, labels } = map;
  const cellW = width / gw;
  const cellH = height / gh;

  for (let row = 0; row < gh; row++) {
    for (let col = 0; col < gw; col++) {
      const idx = row * gw + col;
      const e = energy[idx] ?? 0;
      ctx.fillStyle = energyColor(e);
      ctx.fillRect(col * cellW, row * cellH, cellW, cellH);

      // Sub-band label (small text if cell is large enough)
      const label = labels[idx];
      if (label && cellW > 28 && cellH > 14) {
        ctx.fillStyle = "rgba(255,255,255,0.90)";
        ctx.font = `${Math.min(11, cellH * 0.35)}px monospace`;
        ctx.fillText(label, col * cellW + 3, row * cellH + cellH * 0.6);
      }
    }
  }

  // ── Grid lines ──────────────────────────────────────────────────────────────
  ctx.strokeStyle = "rgba(255,255,255,0.18)";
  ctx.lineWidth = 0.5;
  for (let c = 0; c <= gw; c++) {
    ctx.beginPath();
    ctx.moveTo(c * cellW, 0);
    ctx.lineTo(c * cellW, height);
    ctx.stroke();
  }
  for (let r = 0; r <= gh; r++) {
    ctx.beginPath();
    ctx.moveTo(0, r * cellH);
    ctx.lineTo(width, r * cellH);
    ctx.stroke();
  }

  // ── Footer ─────────────────────────────────────────────────────────────────
  ctx.fillStyle = "rgba(0,0,0,0.60)";
  ctx.fillRect(10, height - 22, 200, 16);
  ctx.fillStyle = "#aaddff";
  ctx.font = "9px sans-serif";
  ctx.fillText("⚠ sub-band energy proxy", 14, height - 9);
}
