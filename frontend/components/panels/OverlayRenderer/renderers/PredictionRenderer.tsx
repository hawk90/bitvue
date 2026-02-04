/**
 * Prediction Overlay Renderer
 *
 * F3: Shows prediction modes with color coding
 */

import { memo } from 'react';
import type { OverlayRendererProps, LegendItem } from '../types';
import { getCssVar } from '../../../../utils/css';
import { PREDICTION_MODE_COLORS, getPredictionModeName } from '../utils/colors';
import { drawLegend } from '../utils/drawing';

export const PredictionOverlay = memo(function PredictionOverlay({
  ctx,
  width,
  height,
  frame,
}: OverlayRendererProps) {
  if (!frame.prediction_mode_grid) {
    // No prediction mode data available - show message
    ctx.fillStyle = 'rgba(0, 0, 0, 0.7)';
    ctx.fillRect(10, 10, 260, 30);
    ctx.fillStyle = getCssVar('--text-bright') || '#fff';
    ctx.font = '12px sans-serif';
    ctx.fillText('Prediction mode data not available', 20, 30);
    return;
  }

  const { modes, block_w, block_h, grid_w, grid_h } = frame.prediction_mode_grid;
  const borderColor = getCssVar('--border-light') || 'rgba(255, 255, 255, 0.1)';

  // Collect unique modes for legend
  const uniqueModes = new Set<number>();
  let blockCount = 0;

  // Render prediction mode blocks
  for (let row = 0; row < grid_h; row++) {
    for (let col = 0; col < grid_w; col++) {
      const idx = row * grid_w + col;
      const mode = modes[idx];

      // Skip missing values
      if (mode === null || mode === undefined) continue;

      const x = col * block_w;
      const y = row * block_h;

      // Get color for this mode
      const baseColor = PREDICTION_MODE_COLORS[mode] || '#888888';
      const color = baseColor + '40'; // 25% opacity

      ctx.fillStyle = color;
      ctx.fillRect(x, y, block_w, block_h);
      ctx.strokeStyle = borderColor;
      ctx.strokeRect(x, y, block_w, block_h);

      uniqueModes.add(mode);
      blockCount++;
    }
  }

  // Create legend from unique modes (sorted by mode value)
  const legendItems: LegendItem[] = Array.from(uniqueModes)
    .sort((a, b) => a - b)
    .map(mode => ({
      color: (PREDICTION_MODE_COLORS[mode] || '#888888') + '80',
      label: getPredictionModeName(mode),
    }))
    .slice(0, 8); // Limit to 8 items for space

  // Draw stats
  ctx.fillStyle = 'rgba(0, 0, 0, 0.7)';
  ctx.fillRect(10, 10, 160, 50);
  ctx.fillStyle = getCssVar('--text-bright') || '#fff';
  ctx.font = '12px monospace';
  ctx.fillText(`Blocks: ${blockCount}`, 20, 28);
  ctx.fillText(`Modes: ${uniqueModes.size}`, 20, 48);

  // Draw legend
  if (legendItems.length > 0) {
    drawLegend(ctx, legendItems, width, height);
  }
});
