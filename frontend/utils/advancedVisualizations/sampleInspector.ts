/**
 * Advanced Visualization Renderers - Sample Inspector
 *
 * F15: Sample Values (YUV pixel inspection)
 */

import type { SampleInspector } from "./types";

/**
 * Render sample values overlay
 */
export function renderSampleValues(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  inspector: SampleInspector,
  blockSize: number = 8,
): void {
  if (!inspector.enabled) {
    return;
  }

  const gridX = Math.floor(inspector.x / blockSize);
  const gridY = Math.floor(inspector.y / blockSize);

  // Highlight inspected pixel's block
  ctx.strokeStyle = "#ffffff";
  ctx.lineWidth = 2;
  ctx.strokeRect(gridX * blockSize, gridY * blockSize, blockSize, blockSize);

  // Draw crosshair at inspected pixel
  ctx.strokeStyle = "#ff0000";
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(inspector.x, 0);
  ctx.lineTo(inspector.x, height);
  ctx.moveTo(0, inspector.y);
  ctx.lineTo(width, inspector.y);
  ctx.stroke();

  // Draw sample info box
  const padding = 8;
  const boxWidth = 140;
  const boxHeight = 100;
  const boxX = Math.min(inspector.x + 10, width - boxWidth - 10);
  const boxY = Math.min(inspector.y + 10, height - boxHeight - 10);

  ctx.fillStyle = "rgba(0, 0, 0, 0.8)";
  ctx.fillRect(boxX, boxY, boxWidth, boxHeight);
  ctx.strokeStyle = "#ffffff";
  ctx.lineWidth = 1;
  ctx.strokeRect(boxX, boxY, boxWidth, boxHeight);

  // Draw sample values
  ctx.fillStyle = "#ffffff";
  ctx.font = "12px monospace";
  ctx.textAlign = "left";

  let lineY = boxY + 20;
  const lineHeight = 16;

  ctx.fillText(
    `Position: (${inspector.x}, ${inspector.y})`,
    boxX + padding,
    lineY,
  );
  lineY += lineHeight;

  ctx.fillText(`Y: ${inspector.sampleY}`, boxX + padding, lineY);
  lineY += lineHeight;

  ctx.fillText(`U: ${inspector.sampleU}`, boxX + padding, lineY);
  lineY += lineHeight;

  ctx.fillText(`V: ${inspector.sampleV}`, boxX + padding, lineY);
  lineY += lineHeight;

  ctx.fillStyle = inspector.rgb;
  ctx.fillRect(boxX + padding, boxY + boxHeight - 20, 40, 12);
  ctx.strokeStyle = "#ffffff";
  ctx.strokeRect(boxX + padding, boxY + boxHeight - 20, 40, 12);

  ctx.fillStyle = "#ffffff";
  ctx.fillText("RGB", boxX + padding + 4, boxY + boxHeight - 12);
}

/**
 * Get color from YUV values
 */
export function yuvToRgb(y: number, u: number, v: number): string {
  // ITU-R BT.709 conversion
  const yVal = y;
  const uVal = u - 128;
  const vVal = v - 128;

  const r = Math.round(yVal + 1.5748 * vVal);
  const g = Math.round(yVal - 0.1873 * uVal - 0.4681 * vVal);
  const b = Math.round(yVal + 1.8556 * uVal);

  const rClamped = Math.max(0, Math.min(255, r));
  const gClamped = Math.max(0, Math.min(255, g));
  const bClamped = Math.max(0, Math.min(255, b));

  return `rgb(${rClamped}, ${gClamped}, ${bClamped})`;
}

/**
 * Create sample inspector from pixel data
 */
export function createSampleInspector(
  yuvData: Uint8Array,
  width: number,
  height: number,
  x: number,
  y: number,
  enabled: boolean = true,
): SampleInspector {
  if (x < 0 || x >= width || y < 0 || y >= height) {
    return {
      enabled: false,
      x: 0,
      y: 0,
      sampleY: 0,
      sampleU: 0,
      sampleV: 0,
      rgb: "#000000",
    };
  }

  const yOffset = y * width + x;
  const uvOffset =
    Math.floor(y / 2) * Math.floor(width / 2) + Math.floor(x / 2);

  const sampleY = yuvData[yOffset];
  const sampleU = yuvData[width * height + uvOffset];
  const sampleV = yuvData[(width * height * 5) / 4 + uvOffset];

  const rgb = yuvToRgb(sampleY, sampleU, sampleV);

  return {
    enabled,
    x,
    y,
    sampleY,
    sampleU,
    sampleV,
    rgb,
  };
}

export { SampleInspector };
