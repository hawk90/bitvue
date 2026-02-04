/**
 * MV Field Overlay Renderer
 *
 * F6: Shows motion vectors as arrows using real parser data
 */

import { memo } from 'react';
import type { OverlayRendererProps } from '../types';
import { getCssVar } from '../../../../utils/css';
import { drawArrow } from '../utils/drawing';

export const MVFieldOverlay = memo(function MVFieldOverlay({
  ctx,
  _width,
  _height,
  frame,
}: OverlayRendererProps) {
  if (frame.frame_type === 'I' || frame.frame_type === 'KEY') {
    // No motion vectors for intra frames
    ctx.fillStyle = getCssVar('--text-bright') || 'rgba(255, 255, 255, 0.8)';
    ctx.font = '14px sans-serif';
    ctx.fillText('No motion vectors (Intra frame)', 10, 20);
    return;
  }

  if (!frame.mv_grid) {
    // No MV data available - show message
    ctx.fillStyle = getCssVar('--text-bright') || 'rgba(255, 255, 255, 0.8)';
    ctx.font = '14px sans-serif';
    ctx.fillText('MV data not available for this frame', 10, 20);
    return;
  }

  const { mv_l0, block_w, block_h, grid_w, grid_h } = frame.mv_grid;

  // Density control: cap at 8000 vectors (per spec)
  const maxVectors = 8000;
  const totalBlocks = grid_w * grid_h;
  const stride = totalBlocks > maxVectors
    ? Math.ceil(Math.sqrt(totalBlocks / maxVectors))
    : 1;

  const mvColor = getCssVar('--accent-primary-light') || 'rgba(100, 200, 255, 0.8)';
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
  ctx.fillStyle = 'rgba(0, 0, 0, 0.7)';
  ctx.fillRect(10, 10, 150, 50);
  ctx.fillStyle = getCssVar('--text-bright') || '#fff';
  ctx.font = '12px monospace';
  ctx.fillText(`MV vectors: ${drawnCount}`, 20, 28);
  ctx.fillText(`Stride: ${stride}`, 20, 48);
});
