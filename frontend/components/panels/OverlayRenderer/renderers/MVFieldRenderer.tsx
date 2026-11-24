/**
 * MV Field Overlay Renderer
 *
 * F6: Shows motion vectors as arrows using real parser data.
 * When `webglCanvas` is provided and the block count exceeds
 * WEBGL_MV_THRESHOLD, rendering is delegated to the WebGL2 path for
 * better performance on dense grids.
 */

import type { OverlayRendererProps } from "../types";
import { getCssVar } from "../../../../utils/css";
import { drawArrow } from "../utils/drawing";
import { drawInfoBadge } from "../utils/helpers";
import {
  renderMVWebGL,
  clearMVWebGL,
  WEBGL_MV_THRESHOLD,
} from "../webgl/mv-webgl";

export interface MVFieldOverlayProps extends OverlayRendererProps {
  /** Optional WebGL overlay canvas. When present and the grid is dense,
   *  rendering switches to the WebGL2 path. */
  webglCanvas?: HTMLCanvasElement;
}

export function MVFieldOverlay({
  ctx,
  width,
  height,
  frame,
  webglCanvas,
}: MVFieldOverlayProps) {
  if (frame.frame_type === "I" || frame.frame_type === "KEY") {
    // No motion vectors for intra frames
    drawInfoBadge(
      ctx,
      width,
      ["No motion vectors (Intra frame)"],
      "14px sans-serif",
      getCssVar("--text-bright") || "rgba(255, 255, 255, 0.8)",
    );
    return;
  }

  if (!frame.mv_grid) {
    // No MV data available - show message
    drawInfoBadge(
      ctx,
      width,
      ["MV data not available for this frame"],
      "14px sans-serif",
      getCssVar("--text-bright") || "rgba(255, 255, 255, 0.8)",
    );
    return;
  }

  const { mv_l0, block_w, block_h, grid_w, grid_h } = frame.mv_grid;
  const totalBlocks = grid_w * grid_h;

  // ── WebGL fast path for dense grids ─────────────────────────────────────
  if (webglCanvas) {
    if (totalBlocks > WEBGL_MV_THRESHOLD) {
      renderMVWebGL(webglCanvas, frame, width, height);
      return;
    } else {
      // Clear any leftover WebGL content before 2D path runs
      clearMVWebGL(webglCanvas);
    }
  }

  // Density control: cap at 8000 vectors (per spec)
  const maxVectors = 8000;
  const stride =
    totalBlocks > maxVectors
      ? Math.ceil(Math.sqrt(totalBlocks / maxVectors))
      : 1;

  const mvColor =
    getCssVar("--accent-primary-light") || "rgba(100, 200, 255, 0.8)";
  ctx.strokeStyle = mvColor;
  ctx.fillStyle = mvColor;
  ctx.lineWidth = 2;

  let drawnCount = 0;

  // Draw MV arrows with stride sampling
  for (let row = 0; row < grid_h; row += stride) {
    for (let col = 0; col < grid_w; col += stride) {
      const idx = row * grid_w + col;
      const mv = mv_l0[idx];

      // Check for missing MV (sentinel value)
      if (mv.dx_qpel === 2147483647 || mv.dy_qpel === 2147483647) continue;

      // Convert quarter-pel to pixels
      const dx = mv.dx_qpel / 4;
      const dy = mv.dy_qpel / 4;

      const centerX = col * block_w + block_w / 2;
      const centerY = row * block_h + block_h / 2;

      drawArrow(ctx, centerX, centerY, dx, dy);
      drawnCount++;
    }
  }

  // Draw MV statistics
  drawInfoBadge(
    ctx,
    width,
    [`MV vectors: ${drawnCount}`, `Stride: ${stride}`],
    "12px monospace",
    getCssVar("--text-bright") || "#fff",
  );
}
