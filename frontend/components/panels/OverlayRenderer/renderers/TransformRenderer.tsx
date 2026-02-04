/**
 * Transform Overlay Renderer
 *
 * F4: Shows transform block sizes using real parser data
 */

import { memo } from 'react';
import type { OverlayRendererProps, LegendItem } from '../types';
import { getCssVar } from '../../../../utils/css';
import { TX_SIZE_COLORS, getTxSizeName, getTxSizePixels } from '../utils/colors';
import { drawLegend } from '../utils/drawing';

export const TransformOverlay = memo(function TransformOverlay({
  ctx,
  width,
  height,
  frame,
}: OverlayRendererProps) {
  if (!frame.transform_grid) {
    // No transform data available - show message
    ctx.fillStyle = 'rgba(0, 0, 0, 0.7)';
    ctx.fillRect(10, 10, 260, 30);
    ctx.fillStyle = getCssVar('--text-bright') || '#fff';
    ctx.font = '12px sans-serif';
    ctx.fillText('Transform data not available for this frame', 20, 30);
    return;
  }

  const { tx_sizes, block_w, block_h, grid_w, grid_h } = frame.transform_grid;
  const borderColor = getCssVar('--border-light') || 'rgba(255, 255, 255, 0.1)';

  // Count transform sizes for legend
  const txCounts = new Map<number, number>();
  let blockCount = 0;

  // Render transform blocks
  for (let row = 0; row < grid_h; row++) {
    for (let col = 0; col < grid_w; col++) {
      const idx = row * grid_w + col;
      const txSize = tx_sizes[idx];

      // Skip missing values
      if (txSize === null || txSize === undefined) continue;

      const x = col * block_w;
      const y = row * block_h;
      const size = getTxSizePixels(txSize);

      // Get color for this transform size
      const color = TX_SIZE_COLORS[txSize] || '#888888';

      // Draw transform block outline (centered within the block)
      ctx.strokeStyle = color;
      ctx.lineWidth = 2;
      ctx.strokeRect(x + (block_w - size) / 2, y + (block_h - size) / 2, size, size);

      // Count for stats
      txCounts.set(txSize, (txCounts.get(txSize) || 0) + 1);
      blockCount++;
    }
  }

  // Create legend from unique transform sizes
  const legendItems: LegendItem[] = Array.from(txCounts.entries())
    .sort((a, b) => a[0] - b[0])  // Sort by tx size
    .map(([txSize, count]) => ({
      color: (TX_SIZE_COLORS[txSize] || '#888888') + '80',
      label: `${getTxSizeName(txSize)} (${count})`,
    }));

  // Draw stats
  ctx.fillStyle = 'rgba(0, 0, 0, 0.7)';
  ctx.fillRect(10, 10, 160, 50);
  ctx.fillStyle = getCssVar('--text-bright') || '#fff';
  ctx.font = '12px monospace';
  ctx.fillText(`Blocks: ${blockCount}`, 20, 28);
  ctx.fillText(`Types: ${txCounts.size}`, 20, 48);

  // Draw legend
  if (legendItems.length > 0) {
    drawLegend(ctx, legendItems, width, height);
  }
});
