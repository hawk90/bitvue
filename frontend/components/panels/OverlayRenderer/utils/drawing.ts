/**
 * Drawing Utilities for Overlay Rendering
 *
 * Canvas drawing functions for overlays
 */

import { getCssVar } from "../../../../utils/css";
import type { LegendItem } from "../types";

/**
 * Draw arrow between two points with proper offset from node edges
 */
export function drawArrowBetweenPoints(
  ctx: CanvasRenderingContext2D,
  x1: number,
  y1: number,
  x2: number,
  y2: number,
  nodeRadius: number,
  color: string,
): void {
  // Calculate direction and distance
  const dx = x2 - x1;
  const dy = y2 - y1;
  const distance = Math.sqrt(dx * dx + dy * dy);

  if (distance < nodeRadius * 2) return; // Too close

  // Calculate start and end points (at node edges)
  const startX = x1 + (dx / distance) * nodeRadius;
  const startY = y1 + (dy / distance) * nodeRadius;
  const endX = x2 - (dx / distance) * nodeRadius;
  const endY = y2 - (dy / distance) * nodeRadius;

  // Draw arrow line
  ctx.strokeStyle = color;
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.moveTo(startX, startY);
  ctx.lineTo(endX, endY);
  ctx.stroke();

  // Draw arrowhead at end
  const angle = Math.atan2(endY - startY, endX - startX);
  const headLen = 8;

  ctx.fillStyle = color;
  ctx.beginPath();
  ctx.moveTo(endX, endY);
  ctx.lineTo(
    endX - headLen * Math.cos(angle - Math.PI / 6),
    endY - headLen * Math.sin(angle - Math.PI / 6),
  );
  ctx.lineTo(
    endX - headLen * Math.cos(angle + Math.PI / 6),
    endY - headLen * Math.sin(angle + Math.PI / 6),
  );
  ctx.closePath();
  ctx.fill();
}

/**
 * Draw an arrow from (x, y) with delta (dx, dy)
 */
export function drawArrow(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  dx: number,
  dy: number,
): void {
  const x2 = x + dx;
  const y2 = y + dy;

  // Draw line
  ctx.beginPath();
  ctx.moveTo(x, y);
  ctx.lineTo(x2, y2);
  ctx.stroke();

  // Draw arrowhead
  const angle = Math.atan2(dy, dx);
  const headLen = 6;
  ctx.beginPath();
  ctx.moveTo(x2, y2);
  ctx.lineTo(
    x2 - headLen * Math.cos(angle - Math.PI / 6),
    y2 - headLen * Math.sin(angle - Math.PI / 6),
  );
  ctx.lineTo(
    x2 - headLen * Math.cos(angle + Math.PI / 6),
    y2 - headLen * Math.sin(angle + Math.PI / 6),
  );
  ctx.closePath();
  ctx.fill();
}

/**
 * Draw frame type indicator in corner
 */
export function drawFrameTypeIndicator(
  ctx: CanvasRenderingContext2D,
  frameType: string,
  width: number,
): void {
  const color = getFrameTypeColor(frameType);

  ctx.fillStyle = "rgba(0, 0, 0, 0.7)";
  ctx.fillRect(width - 60, 10, 50, 24);

  ctx.fillStyle = color;
  ctx.font = "bold 14px sans-serif";
  ctx.fillText(frameType, width - 50, 27);
}

/**
 * Draw legend for overlay
 */
export function drawLegend(
  ctx: CanvasRenderingContext2D,
  items: LegendItem[],
  width: number,
  height: number,
): void {
  const legendWidth = items.length * 80 + 20;
  const legendHeight = 30;
  const x = width - legendWidth - 10;
  const y = height - legendHeight - 10;

  ctx.fillStyle = "rgba(0, 0, 0, 0.7)";
  ctx.fillRect(x, y, legendWidth, legendHeight);

  items.forEach((item, idx) => {
    const itemX = x + 10 + idx * 80;

    ctx.fillStyle = item.color;
    ctx.fillRect(itemX, y + 10, 16, 12);

    ctx.fillStyle = getCssVar("--text-bright") || "#fff";
    ctx.font = "11px sans-serif";
    ctx.fillText(item.label, itemX + 22, y + 20);
  });
}

// Re-export getFrameTypeColor from colors for convenience
import { getFrameTypeColor as getFrameTypeColorFromColors } from "./colors";
export { getFrameTypeColorFromColors as getFrameTypeColor };
