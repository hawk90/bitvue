/**
 * QP Map Overlay Renderer
 *
 * F5: Shows quantization parameter heatmap using real parser data
 */

import { memo } from "react";
import type { OverlayRendererProps } from "../types";
import { getCssVar } from "../../../../utils/css";
import { qpToColor } from "../utils/helpers";

export const QPMapOverlay = memo(function QPMapOverlay({
  ctx,
  _width,
  _height,
  frame,
}: OverlayRendererProps) {
  if (!frame.qp_grid) {
    // No QP data available - show message
    ctx.fillStyle = "rgba(0, 0, 0, 0.7)";
    ctx.fillRect(10, 10, 220, 30);
    ctx.fillStyle = getCssVar("--text-bright") || "#fff";
    ctx.font = "12px sans-serif";
    ctx.fillText("QP data not available for this frame", 20, 30);
    return;
  }

  const { qp, block_w, block_h, grid_w, grid_h, qp_min, qp_max } =
    frame.qp_grid;

  // Render QP heatmap blocks
  for (let row = 0; row < grid_h; row++) {
    for (let col = 0; col < grid_w; col++) {
      const idx = row * grid_w + col;
      const qpVal = qp[idx];

      // Skip missing values
      if (qpVal === -1) continue;

      const color = qpToColor(qpVal, qp_min, qp_max);
      ctx.fillStyle = color;
      ctx.fillRect(col * block_w, row * block_h, block_w, block_h);
    }
  }

  // Draw QP value indicator
  ctx.fillStyle = "rgba(0, 0, 0, 0.7)";
  ctx.fillRect(10, 10, 120, 50);
  ctx.fillStyle = getCssVar("--text-bright") || "#fff";
  ctx.font = "12px monospace";
  ctx.fillText(`QP: ${qp_min} - ${qp_max}`, 20, 28);
  ctx.fillText(`Blocks: ${grid_w}x${grid_h}`, 20, 48);
});
