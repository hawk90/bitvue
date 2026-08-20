/**
 * AV1 CDEF Filter Overlay Renderer
 *
 * F6 (AV1): Renders CDEF direction / strength per superblock.
 *   Pass 1 — fill block with a blue→red strength heatmap
 *   Pass 2 — draw a direction arrow at block center (8 directions)
 * Blocks with strength=0 are shown in light gray (no filter applied).
 * A summary legend is drawn in the bottom-left corner.
 *
 * CDEF direction encoding (AV1 spec §7.15):
 *   0=horizontal, 1=≈22°, 2=45°, 3=≈67°, 4=vertical,
 *   5=≈112°, 6=135°, 7=≈157°
 */

import type { FrameInfo } from "../../../../types/video";
import type { Av1CdefData } from "../types";

interface Av1CdefRendererProps {
  ctx: CanvasRenderingContext2D;
  width: number;
  height: number;
  frame: FrameInfo;
  cdef: Av1CdefData | undefined;
}

/** Map a CDEF strength value (0–255) to an RGBA fill color string. */
function strengthToColor(strength: number): string {
  if (strength === 0) return "rgba(180, 180, 180, 0.35)";
  const t = Math.min(strength / 255, 1);
  const r = Math.round(t * 220);
  const g = Math.round(80 * (1 - t));
  const b = Math.round(220 * (1 - t));
  return `rgba(${r}, ${g}, ${b}, 0.50)`;
}

/**
 * CDEF direction → angle in radians.
 * AV1 direction 0 = horizontal (east), increases counter-clockwise.
 */
const CDEF_DIR_ANGLE: number[] = [
  0, // 0: horizontal
  Math.PI / 8, // 1: 22.5°
  Math.PI / 4, // 2: 45°
  (3 * Math.PI) / 8, // 3: 67.5°
  Math.PI / 2, // 4: vertical
  (5 * Math.PI) / 8, // 5: 112.5°
  (3 * Math.PI) / 4, // 6: 135°
  (7 * Math.PI) / 8, // 7: 157.5°
];

/** Draw a bidirectional direction arrow (CDEF filter direction). */
function drawDirectionArrow(
  ctx: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  angle: number,
  length: number,
): void {
  const dx = Math.cos(angle) * length;
  const dy = Math.sin(angle) * length;

  ctx.beginPath();
  ctx.moveTo(cx - dx, cy - dy);
  ctx.lineTo(cx + dx, cy + dy);
  ctx.stroke();

  // Arrowheads on both ends (bidirectional)
  const headLen = Math.max(3, length * 0.3);
  const headAngle = Math.PI / 5;
  for (const sign of [1, -1]) {
    const ex = cx + sign * dx;
    const ey = cy + sign * dy;
    const backAngle = angle + Math.PI; // reverse direction for arrowhead
    ctx.beginPath();
    ctx.moveTo(ex, ey);
    ctx.lineTo(
      ex + sign * Math.cos(backAngle - headAngle) * headLen,
      ey + sign * Math.sin(backAngle - headAngle) * headLen,
    );
    ctx.moveTo(ex, ey);
    ctx.lineTo(
      ex + sign * Math.cos(backAngle + headAngle) * headLen,
      ey + sign * Math.sin(backAngle + headAngle) * headLen,
    );
    ctx.stroke();
  }
}

export function Av1CdefRenderer({
  ctx,
  width,
  height,
  frame: _frame,
  cdef,
}: Av1CdefRendererProps): void {
  ctx.save();

  if (!cdef) {
    // Absent -- backend's extract_cdef_data returns None when the stream doesn't carry
    // cdef_params at all (distinct from a real block having strength 0), same "disabled"
    // convention as the sibling AV1-feature renderers.
    const msg = "CDEF: disabled";
    ctx.font = "14px monospace";
    const measured = ctx.measureText(msg);
    const boxW = measured.width + 24;
    const boxH = 32;
    const boxX = (width - boxW) / 2;
    const boxY = (height - boxH) / 2;

    ctx.fillStyle = "rgba(0, 0, 0, 0.70)";
    ctx.fillRect(boxX, boxY, boxW, boxH);
    ctx.fillStyle = "#888888";
    ctx.fillText(msg, boxX + 12, boxY + 21);
    ctx.restore();
    return;
  }

  // ── Pass 1: block strength heatmap ────────────────────────────────────────
  ctx.lineWidth = 0.5;
  ctx.strokeStyle = "rgba(255, 255, 255, 0.30)";

  for (const block of cdef.blocks) {
    ctx.fillStyle = strengthToColor(block.strength);
    ctx.fillRect(block.x, block.y, block.size, block.size);
    ctx.strokeRect(block.x, block.y, block.size, block.size);
  }

  // ── Pass 2: direction arrows (skip blocks with no filter) ─────────────────
  const arrowLength = Math.max(4, cdef.blockSize * 0.28);

  for (const block of cdef.blocks) {
    if (block.strength === 0) continue;

    const cx = block.x + block.size / 2;
    const cy = block.y + block.size / 2;
    const angle = CDEF_DIR_ANGLE[block.direction & 7] ?? 0;

    ctx.strokeStyle = "rgba(255, 255, 255, 0.80)";
    ctx.lineWidth = 1.2;
    drawDirectionArrow(ctx, cx, cy, angle, arrowLength);
  }

  // ── Legend box ────────────────────────────────────────────────────────────
  const legendText = [
    `CDEF Y-Primary: ${cdef.yPrimaryStrength}`,
    `Y-Secondary: ${cdef.ySecondaryStrength}`,
    `Damping: ${cdef.damping}`,
    "Arrows = filter direction",
  ];

  const boxX = 10;
  const boxY = height - 90;
  const boxW = 215;
  const boxH = 80;

  ctx.fillStyle = "rgba(0, 0, 0, 0.72)";
  ctx.fillRect(boxX, boxY, boxW, boxH);

  ctx.font = "11px monospace";
  legendText.forEach((line, i) => {
    ctx.fillStyle = i === 3 ? "#aaddff" : "#ffffff";
    ctx.fillText(line, boxX + 10, boxY + 18 + i * 18);
  });

  ctx.restore();
}
