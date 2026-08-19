/**
 * AV1 Loop Restoration Overlay Renderer
 *
 * F8 (AV1): Colors each restoration unit by type:
 *   None     — gray
 *   Wiener   — blue
 *   SgrProj  — green
 *   Dual     — yellow
 * A color-coded legend is drawn in the top-left corner.
 */

import type { FrameInfo } from "../../../../types/video";
import type { Av1LoopRestorationData } from "../types";

interface Av1LoopRestorationRendererProps {
  ctx: CanvasRenderingContext2D;
  width: number;
  height: number;
  frame: FrameInfo;
  loopRestoration: Av1LoopRestorationData;
}

/** Fill color by restoration unit type. */
const RESTORATION_COLORS: Record<number, string> = {
  0: "rgba(100, 100, 100, 0.45)", // None — gray
  1: "rgba(66,  135, 245, 0.55)", // Wiener — blue
  2: "rgba(52,  168,  83, 0.55)", // SgrProj — green
  3: "rgba(251, 188,   5, 0.55)", // Dual — yellow
};

const LEGEND_ITEMS = [
  { color: "rgba(66,  135, 245, 0.80)", label: "Wiener" },
  { color: "rgba(52,  168,  83, 0.80)", label: "SgrProj" },
  { color: "rgba(251, 188,   5, 0.80)", label: "Dual" },
  { color: "rgba(140, 140, 140, 0.80)", label: "None" },
];

export function Av1LoopRestorationRenderer({
  ctx,
  width: _width,
  height: _height,
  frame: _frame,
  loopRestoration,
}: Av1LoopRestorationRendererProps): void {
  ctx.save();

  // ── Draw restoration units ────────────────────────────────────────────────
  for (const unit of loopRestoration.units) {
    const color =
      RESTORATION_COLORS[unit.restorationType] ?? RESTORATION_COLORS[0];
    ctx.fillStyle = color;
    ctx.fillRect(unit.x, unit.y, unit.size, unit.size);

    // Thin border to distinguish adjacent units
    ctx.strokeStyle = "rgba(255, 255, 255, 0.20)";
    ctx.lineWidth = 0.5;
    ctx.strokeRect(unit.x, unit.y, unit.size, unit.size);
  }

  // ── Legend box ────────────────────────────────────────────────────────────
  const swatchSize = 12;
  const itemH = 18;
  const boxPad = 10;
  const boxW = 130;
  const boxH = boxPad * 2 + LEGEND_ITEMS.length * itemH;
  const boxX = 10;
  const boxY = 10;

  ctx.fillStyle = "rgba(0, 0, 0, 0.72)";
  ctx.fillRect(boxX, boxY, boxW, boxH);

  ctx.font = "11px monospace";

  LEGEND_ITEMS.forEach((item, i) => {
    const y = boxY + boxPad + i * itemH;
    ctx.fillStyle = item.color;
    ctx.fillRect(boxX + boxPad, y, swatchSize, swatchSize);
    ctx.fillStyle = "#ffffff";
    ctx.fillText(item.label, boxX + boxPad + swatchSize + 6, y + 10);
  });

  ctx.restore();
}
