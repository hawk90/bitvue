/**
 * Coding Flow Overlay Renderer
 *
 * F2: Shows block structure (CTU/tile boundaries) using real partition data
 */

import { memo } from 'react';
import type { OverlayRendererProps } from '../types';
import { getCssVar } from '../../../../utils/css';
import { drawFrameTypeIndicator } from '../utils/drawing';

export const CodingFlowOverlay = memo(function CodingFlowOverlay({
  ctx,
  width,
  height,
  frame,
}: OverlayRendererProps) {
  if (!frame.partition_grid) {
    // No partition data available - show message
    ctx.fillStyle = 'rgba(0, 0, 0, 0.7)';
    ctx.fillRect(10, 10, 240, 30);
    ctx.fillStyle = getCssVar('--text-bright') || '#fff';
    ctx.font = '12px sans-serif';
    ctx.fillText('Partition data not available', 20, 30);
    return;
  }

  const { blocks, sb_size } = frame.partition_grid;

  // Draw partition blocks
  ctx.lineWidth = 1;

  for (const block of blocks) {
    // Color based on partition type
    let color = getCssVar('--accent-primary') || 'rgba(0, 122, 204, 0.5)';

    switch (block.partition) {
      case 0: // PartitionType.None
        color = 'rgba(100, 150, 255, 0.3)';  // Blue - leaf block
        break;
      case 1: // PartitionType.Horz
        color = 'rgba(255, 150, 100, 0.4)';  // Orange - horizontal split
        break;
      case 2: // PartitionType.Vert
        color = 'rgba(100, 200, 255, 0.4)';  // Cyan - vertical split
        break;
      case 3: // PartitionType.Split
        color = 'rgba(255, 255, 100, 0.4)';  // Yellow - 4-way split
        break;
      default:
        color = 'rgba(200, 200, 200, 0.3)';  // Gray - other
    }

    // Darken color based on depth (deeper = darker)
    const depthFactor = Math.max(0.2, 1 - (block.depth * 0.15));
    const match = color.match(/[\d.]+/g);
    if (match) {
      const [r, g, b, a] = match.map(parseFloat);
      color = `rgba(${r * depthFactor}, ${g * depthFactor}, ${b * depthFactor}, ${a})`;
    }

    ctx.strokeStyle = color;
    ctx.strokeRect(block.x, block.y, block.width, block.height);

    // Draw block label for CTU-level blocks
    if (block.width >= sb_size && block.height >= sb_size) {
      ctx.fillStyle = 'rgba(255, 255, 255, 0.7)';
      ctx.font = '10px monospace';
      const label = `${Math.round(block.x / sb_size)}${String.fromCharCode(65 + (block.y / sb_size))}`;
      ctx.fillText(label, block.x + 2, block.y + 12);
    }
  }

  // Draw frame type indicator in corner
  drawFrameTypeIndicator(ctx, frame.frame_type, width);

  // Draw block count info
  ctx.fillStyle = 'rgba(0, 0, 0, 0.7)';
  ctx.fillRect(10, 10, 140, 40);
  ctx.fillStyle = getCssVar('--text-bright') || '#fff';
  ctx.font = '12px monospace';
  ctx.fillText(`Blocks: ${blocks.length}`, 20, 28);
  ctx.fillText(`SB: ${sb_size}px`, 20, 45);
});
