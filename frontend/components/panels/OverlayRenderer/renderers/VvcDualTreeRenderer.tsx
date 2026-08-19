/**
 * VVC Dual Tree Renderer
 *
 * F1 mode for VVC: renders luma and chroma coding tree boundaries separately.
 *
 * Color scheme (VQ Analyzer parity):
 *   tree_type 0 or 1 (luma / single-tree): blue border (#4488FF)
 *   tree_type 2       (chroma):             red  border (#FF4444)
 *
 * When no dual-tree data is available (non-VVC stream or pre-v0.12 data that
 * predates the tree_type field), falls back to the standard CodingFlow palette.
 */

import type { OverlayRendererProps } from "../types";

export function VvcDualTreeRenderer({
  ctx,
  width,
  height,
  frame,
}: OverlayRendererProps) {
  const grid = frame.partition_grid;
  if (!grid) {
    ctx.fillStyle = "rgba(0,0,0,0.7)";
    ctx.fillRect(10, 10, 280, 30);
    ctx.fillStyle = "#fff";
    ctx.font = "12px sans-serif";
    ctx.fillText("Partition data not available for this frame", 20, 30);
    return;
  }

  const { blocks, coded_width, coded_height } = grid;
  const scaleX = width / coded_width;
  const scaleY = height / coded_height;

  const hasDualTree = blocks.some(
    (b) => b.tree_type !== undefined && b.tree_type !== null,
  );

  ctx.lineWidth = 1;

  for (const block of blocks) {
    const px = block.x * scaleX;
    const py = block.y * scaleY;
    const pw = block.width * scaleX;
    const ph = block.height * scaleY;

    if (hasDualTree) {
      const tt = block.tree_type ?? 0;
      if (tt === 2) {
        // Chroma tree — red fill + red border
        ctx.fillStyle = "rgba(255, 68, 68, 0.15)";
        ctx.strokeStyle = "rgba(255, 68, 68, 0.85)";
      } else {
        // Luma tree (single or dual luma) — blue fill + blue border
        ctx.fillStyle = "rgba(68, 136, 255, 0.12)";
        ctx.strokeStyle = "rgba(68, 136, 255, 0.80)";
      }
    } else {
      // Fallback: shade by depth
      const alpha = Math.max(0.1, 0.5 - block.depth * 0.08);
      ctx.fillStyle = `rgba(100, 160, 255, ${alpha})`;
      ctx.strokeStyle = "rgba(80, 140, 230, 0.7)";
    }

    ctx.fillRect(px, py, pw, ph);
    ctx.strokeRect(px, py, pw, ph);
  }

  // Legend
  if (hasDualTree) {
    const legendY = 12;
    const items: Array<{ color: string; label: string }> = [
      { color: "rgba(68,136,255,0.8)", label: "Luma tree" },
      { color: "rgba(255,68,68,0.8)", label: "Chroma tree" },
    ];
    let lx = 10;
    ctx.font = "11px sans-serif";
    for (const item of items) {
      ctx.fillStyle = "rgba(0,0,0,0.6)";
      ctx.fillRect(lx, legendY, 10, 10);
      ctx.fillStyle = item.color;
      ctx.fillRect(lx, legendY, 10, 10);
      ctx.fillStyle = "#fff";
      ctx.fillText(item.label, lx + 14, legendY + 9);
      lx += 90;
    }
  }
}
