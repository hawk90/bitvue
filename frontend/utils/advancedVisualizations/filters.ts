/**
 * Advanced Visualization Renderers - Filters
 *
 * F13: Deblocking Filter
 * F14: SAO/ALF (Sample Adaptive Offset / Adaptive Loop Filter)
 */

import type { QPGrid } from '../../../types/video';

/**
 * Deblocking filter strength visualization
 */
export function renderDeblocking(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  qpGrid: QPGrid,
  deblockingStrength: number = 0
): void {
  const { grid_w, grid_h, qp } = qpGrid;
  const blockW = width / grid_w;
  const blockH = height / grid_h;

  // Deblocking filter visualization shows edges with filtered strength
  for (let y = 0; y <= grid_h; y++) {
    for (let x = 0; x <= grid_w; x++) {
      // Vertical edge
      if (x < grid_w) {
        const edgeX = Math.floor(x * blockW);
        const strength = Math.min(deblockingStrength, 3);

        // Calculate color based on QP difference (simplified)
        const leftQP = x > 0 ? qp[y * grid_w + (x - 1)] : qp[0];
        const rightQP = qp[y * grid_w + x];
        const qpDiff = Math.abs(leftQP - rightQP);

        ctx.strokeStyle = `rgba(255, 255, 0, ${Math.min(1, qpDiff / 20)})`;
        ctx.lineWidth = strength + 1;
        ctx.beginPath();
        ctx.moveTo(edgeX, 0);
        ctx.lineTo(edgeX, height);
        ctx.stroke();
      }

      // Horizontal edge
      if (y < grid_h) {
        const edgeY = Math.floor(y * blockH);
        const strength = Math.min(deblockingStrength, 3);

        const topQP = y > 0 ? qp[(y - 1) * grid_w + x] : qp[0];
        const bottomQP = qp[y * grid_w + x];
        const qpDiff = Math.abs(topQP - bottomQP);

        ctx.strokeStyle = `rgba(255, 255, 0, ${Math.min(1, qpDiff / 20)})`;
        ctx.lineWidth = strength + 1;
        ctx.beginPath();
        ctx.moveTo(0, edgeY);
        ctx.lineTo(width, edgeY);
        ctx.stroke();
      }
    }
  }
}

/**
 * SAO (Sample Adaptive Offset) visualization
 */
export function renderSAO(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  saoType: 'edge' | 'band' | 'none',
  saoOffset: number = 0
): void {
  if (saoType === 'none') {
    return;
  }

  const blockW = 64; // CTU size
  const gridW = Math.ceil(width / blockW);
  const gridH = Math.ceil(height / blockW);

  for (let y = 0; y < gridH; y++) {
    for (let x = 0; x < gridW; x++) {
      const bx = x * blockW;
      const by = y * blockW;

      if (saoType === 'edge') {
        // Edge offset visualization - green tint
        ctx.fillStyle = `rgba(0, 255, 100, 0.3)`;
      } else if (saoType === 'band') {
        // Band offset visualization - blue tint
        ctx.fillStyle = `rgba(100, 100, 255, 0.3)`;
      }

      ctx.fillRect(bx, by, blockW, blockW);

      // Draw SAO type indicator
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.5)';
      ctx.lineWidth = 1;
      ctx.strokeRect(bx, by, blockW, blockW);
    }
  }
}

/**
 * ALF (Adaptive Loop Filter) visualization
 */
export function renderALF(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  alfEnabled: boolean[],
  alfCoefficients?: number[][]
): void {
  if (!alfEnabled || alfEnabled.length === 0) {
    return;
  }

  const blockW = 64;
  const gridW = Math.ceil(width / blockW);

  for (let i = 0; i < alfEnabled.length; i++) {
    if (alfEnabled[i]) {
      const bx = (i % gridW) * blockW;
      const by = Math.floor(i / gridW) * blockW;

      // ALF visualization - purple tint
      ctx.fillStyle = `rgba(200, 100, 255, 0.4)`;
      ctx.fillRect(bx, by, blockW, blockW);

      // Draw ALF indicator
      ctx.strokeStyle = 'rgba(255, 100, 255, 0.8)';
      ctx.lineWidth = 2;
      ctx.strokeRect(bx, by, blockW, blockW);
    }
  }
}
