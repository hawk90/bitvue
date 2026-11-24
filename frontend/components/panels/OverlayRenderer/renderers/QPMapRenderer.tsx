/**
 * QP Map Overlay Renderer
 *
 * F5: Shows quantization parameter heatmap using real parser data
 */

import type { OverlayRendererProps } from "../types";
import { getCssVar } from "../../../../utils/css";
import { drawInfoBadge, qpToColor } from "../utils/helpers";

export function QPMapOverlay({ ctx, width, frame }: OverlayRendererProps) {
  if (!frame.qp_grid) {
    // No QP data available - show message
    drawInfoBadge(
      ctx,
      width,
      ["QP data not available for this frame"],
      "12px sans-serif",
      getCssVar("--text-bright") || "#fff",
    );
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
  drawInfoBadge(
    ctx,
    width,
    [`QP: ${qp_min} - ${qp_max}`, `Blocks: ${grid_w}x${grid_h}`],
    "12px monospace",
    getCssVar("--text-bright") || "#fff",
  );
}
