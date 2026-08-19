/**
 * AV1 Super Resolution Overlay Renderer
 *
 * F7 (AV1): When super-res is disabled, shows a centered status message.
 * When enabled:
 *   - Draws a vertical dashed line at the decoded (downscaled) width
 *   - Labels each side of the boundary
 *   - Shows scale parameters in the top-left corner
 */

import type { FrameInfo } from "../../../../types/video";
import type { Av1SuperResData } from "../types";

interface Av1SuperResRendererProps {
  ctx: CanvasRenderingContext2D;
  width: number;
  height: number;
  frame: FrameInfo;
  superResolution: Av1SuperResData | undefined;
}

export function Av1SuperResRenderer({
  ctx,
  width,
  height,
  frame: _frame,
  superResolution,
}: Av1SuperResRendererProps): void {
  ctx.save();

  if (!superResolution?.enabled) {
    // ── Disabled notice ───────────────────────────────────────────────────
    const msg = "SuperRes: disabled";
    ctx.font = "14px monospace";
    const measured = ctx.measureText(msg);
    const boxW = measured.width + 24;
    const boxH = 32;
    const boxX = (width - boxW) / 2;
    const boxY = (height - boxH) / 2;

    ctx.fillStyle = "rgba(0, 0, 0, 0.70)";
    ctx.fillRect(boxX, boxY, boxW, boxH);
    ctx.fillStyle = "#aaaaaa";
    ctx.fillText(msg, boxX + 12, boxY + 21);
    ctx.restore();
    return;
  }

  // ── Compute downscaled boundary ───────────────────────────────────────────
  const { scaleDenominator, upscaledWidth, upscaledHeight } = superResolution;
  // AV1 spec: downscaledWidth = ceil(upscaledWidth * 8 / scaleDenominator)
  const downscaledWidth = Math.round((upscaledWidth * 8) / scaleDenominator);
  const lineX = downscaledWidth;

  // ── Dashed vertical divider ───────────────────────────────────────────────
  ctx.strokeStyle = "rgba(255, 220, 50, 0.90)";
  ctx.lineWidth = 2;
  ctx.setLineDash([10, 6]);
  ctx.beginPath();
  ctx.moveTo(lineX, 0);
  ctx.lineTo(lineX, height);
  ctx.stroke();
  ctx.setLineDash([]);

  // ── Side labels ───────────────────────────────────────────────────────────
  ctx.font = "bold 13px sans-serif";
  ctx.fillStyle = "rgba(255, 220, 50, 0.95)";

  const leftLabel = "Decoded (downscaled)";
  const rightLabel = "Upscaled";

  // Left label — centered in the left region
  const leftCenterX = lineX / 2;
  const leftMetrics = ctx.measureText(leftLabel);
  ctx.fillText(leftLabel, leftCenterX - leftMetrics.width / 2, height - 20);

  // Right label — centered in the right region
  const rightRegionW = width - lineX;
  const rightCenterX = lineX + rightRegionW / 2;
  const rightMetrics = ctx.measureText(rightLabel);
  ctx.fillText(rightLabel, rightCenterX - rightMetrics.width / 2, height - 20);

  // ── Info box ──────────────────────────────────────────────────────────────
  const scale = (scaleDenominator / 8).toFixed(3);
  const infoLines = [
    `SuperRes x${scale}`,
    `Upscaled: ${upscaledWidth}x${upscaledHeight}`,
    `Downscaled: ${downscaledWidth}px wide`,
  ];

  const boxX = 10;
  const boxY = 10;
  const boxW = 210;
  const boxH = 20 + infoLines.length * 18;

  ctx.fillStyle = "rgba(0, 0, 0, 0.72)";
  ctx.fillRect(boxX, boxY, boxW, boxH);
  ctx.fillStyle = "#ffffff";
  ctx.font = "11px monospace";
  infoLines.forEach((line, i) => {
    ctx.fillText(line, boxX + 10, boxY + 16 + i * 18);
  });

  ctx.restore();
}
