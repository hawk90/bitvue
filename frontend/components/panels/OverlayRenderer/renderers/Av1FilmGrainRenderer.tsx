/**
 * AV1 Film Grain Overlay Renderer
 *
 * F9 (AV1): Visualizes film grain synthesis state.
 *
 * When film grain is ENABLED:
 *   - Draws a grain noise texture over the frame to preview synthesis intensity
 *     (the noise density is proportional to scalingShift)
 *   - Shows synthesis parameters in an info box (top-right)
 *   - Labels the frame "POST-GRAIN" in the top-left
 *
 * When film grain is DISABLED:
 *   - Shows a centered "disabled" notice
 *
 * VQ Analyzer parity: the before/after pixel comparison requires two decoded
 * buffers (pre-grain and post-grain). Bitvue renders the grain texture as a
 * simulation until a dedicated `pre_grain_frame` field is added to FrameInfo.
 */

import type { FrameInfo } from "../../../../types/video";
import type { Av1FilmGrainData } from "../types";

interface Av1FilmGrainRendererProps {
  ctx: CanvasRenderingContext2D;
  width: number;
  height: number;
  frame: FrameInfo;
  filmGrain: Av1FilmGrainData | undefined;
}

/**
 * Draw a pseudo-random grain noise texture seeded by `seed`.
 * Dots density scales with `intensity` (0–1).
 */
function drawGrainTexture(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  seed: number,
  intensity: number,
): void {
  // Simple LCG-based PRNG seeded by the grain seed
  let state = seed ^ 0xdeadbeef;
  const next = () => {
    state = ((state * 1664525 + 1013904223) | 0) >>> 0;
    return state;
  };

  const dotCount = Math.round(width * height * intensity * 0.04);
  ctx.fillStyle = "rgba(255, 255, 255, 0.18)";
  for (let i = 0; i < dotCount; i++) {
    const x = next() % width;
    const y = next() % height;
    ctx.fillRect(x, y, 1, 1);
  }
  ctx.fillStyle = "rgba(0, 0, 0, 0.14)";
  for (let i = 0; i < dotCount; i++) {
    const x = next() % width;
    const y = next() % height;
    ctx.fillRect(x, y, 1, 1);
  }
}

export function Av1FilmGrainRenderer({
  ctx,
  width,
  height,
  frame: _frame,
  filmGrain,
}: Av1FilmGrainRendererProps): void {
  ctx.save();

  if (!filmGrain?.enabled) {
    const msg = "Film Grain: disabled";
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

  // ── Grain texture overlay ─────────────────────────────────────────────────
  // scalingShift 8–11: higher = stronger grain. Normalize to [0,1].
  const intensity = Math.min(1, Math.max(0, (filmGrain.scalingShift - 6) / 6));
  drawGrainTexture(ctx, width, height, filmGrain.seed, intensity);

  // ── POST-GRAIN label (top-left) ───────────────────────────────────────────
  ctx.fillStyle = "rgba(0,0,0,0.65)";
  ctx.fillRect(10, 10, 110, 22);
  ctx.fillStyle = "rgba(251,188,5,1)";
  ctx.font = "bold 11px monospace";
  ctx.fillText("POST-GRAIN", 18, 25);

  // ── Parameters info box (top-right) ──────────────────────────────────────
  const chromaStr = filmGrain.chromaScalingFromLuma ? "yes" : "no";
  const overlapStr = filmGrain.overlap ? "yes" : "no";

  const lines = [
    "Film Grain Synthesis Active",
    `Seed: ${filmGrain.seed}   Shift: ${filmGrain.scalingShift}`,
    `AR Coeff Lag: ${filmGrain.arCoeffLag}`,
    `Chroma from Luma: ${chromaStr}`,
    `Overlap: ${overlapStr}`,
  ];

  const lineH = 18;
  const padX = 12;
  const padY = 10;
  const boxH = padY * 2 + lines.length * lineH;

  ctx.font = "11px monospace";
  let maxW = 0;
  for (const line of lines) {
    const w = ctx.measureText(line).width;
    if (w > maxW) maxW = w;
  }
  const boxW = maxW + padX * 2;
  const boxX = width - boxW - 10;
  const boxY = 10;

  ctx.fillStyle = "rgba(0, 0, 0, 0.78)";
  ctx.fillRect(boxX, boxY, boxW, boxH);

  lines.forEach((line, i) => {
    ctx.fillStyle = i === 0 ? "rgba(251,188,5,1)" : "#ffffff";
    ctx.font = i === 0 ? "bold 11px monospace" : "11px monospace";
    ctx.fillText(line, boxX + padX, boxY + padY + 12 + i * lineH);
  });

  ctx.restore();
}
