/**
 * Advanced Visualization Renderers - v1.0.0
 *
 * F11-F15 visualization modes:
 * - F11: Intra Prediction Modes
 * - F12: Inter Prediction Modes
 * - F13: Deblocking Filter
 * - F14: SAO/ALF (Sample Adaptive Offset / Adaptive Loop Filter)
 * - F15: Sample Values (YUV pixel inspection)
 */

import { type FrameInfo, type PredictionModeGrid, type MVGrid, type QPGrid } from '../../types/video';

/**
 * Color maps for intra prediction modes
 * Based on HEVC/VVC intra mode angles
 */
export const INTRA_MODE_COLORS: Record<number, string> = [
  '#000000', // 0: Planar
  '#ff0000', // 1: DC
  '#00ff00', // 2: Angular 2
  '#0000ff', // 3: Angular 3
  '#ffff00', // 4: Angular 4
  '#ff00ff', // 5: Angular 5
  '#00ffff', // 6: Angular 6
  '#ff8000', // 7: Angular 7
  '#8000ff', // 8: Angular 8
  '#ff0080', // 9: Angular 9
  '#80ff00', // 10: Angular 10
  '#0080ff', // 11: Angular 11
  '#80ff80', // 12: Angular 12
  '#ff8080', // 13: Angular 13
  '#80ffff', // 14: Angular 14
  '#ffff80', // 15: Angular 15
  '#ff80ff', // 16: Angular 16
  '#80ff00', // 17: Angular 17
  '#00ff80', // 18: Angular 18
  '#0080ff', // 19: Angular 19
  '#ff8000', // 20: Angular 20
  '#8000ff', // 21: Angular 21
  '#ff0080', // 22: Angular 22
  '#80ff80', // 23: Angular 23
  '#80ff00', // 24: Angular 24
  '#00ff80', // 25: Angular 25
  '#008080', // 26: Angular 26
  '#808000', // 27: Angular 27
  '#008080', // 28: Angular 28
  '#800080', // 29: Angular 29
  '#808080', // 30: Angular 30
  '#ff8080', // 31: Angular 31
  '#80ff80', // 32: Angular 32
  '#8080ff', // 33: Angular 33
  '#ff80ff', // 34: Angular 34
];

/**
 * Get intra mode color
 */
export function getIntraModeColor(mode: number | null): string {
  if (mode === null || mode < 0 || mode >= INTRA_MODE_COLORS.length) {
    return '#333333';
  }
  return INTRA_MODE_COLORS[mode];
}

/**
 * Render intra prediction mode overlay
 */
export function renderIntraModes(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  grid: PredictionModeGrid,
  blockSize: number = 16
): void {
  const { grid_w, grid_h, modes } = grid;
  const blockW = width / grid_w;
  const blockH = height / grid_h;

  for (let y = 0; y < grid_h; y++) {
    for (let x = 0; x < grid_w; x++) {
      const idx = y * grid_w + x;
      const mode = modes[idx];

      if (mode !== null) {
        const bx = Math.floor(x * blockW);
        const by = Math.floor(y * blockH);
        const bw = Math.ceil(blockW);
        const bh = Math.ceil(blockH);

        ctx.fillStyle = getIntraModeColor(mode);
        ctx.fillRect(bx, by, bw, bh);

        // Draw mode number for larger blocks
        if (bw >= 32 && bh >= 16) {
          ctx.fillStyle = mode > 20 ? '#000' : '#fff';
          ctx.font = '10px monospace';
          ctx.fillText(mode.toString(), bx + 2, by + 12);
        }
      }
    }
  }
}

/**
 * Inter prediction mode types
 */
export enum InterMode {
  Skip = 0,
  Merge = 1,
  MotionVector = 2,
  Intra = 3,
}

/**
 * Get inter mode color
 */
export function getInterModeColor(mode: InterMode): string {
  switch (mode) {
    case InterMode.Skip:
      return '#4caf50'; // Green
    case InterMode.Merge:
      return '#2196f3'; // Blue
    case InterMode.MotionVector:
      return '#ff9800'; // Orange
    case InterMode.Intra:
      return '#f44336'; // Red
    default:
      return '#666666';
  }
}

/**
 * Render inter prediction mode overlay
 */
export function renderInterModes(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  mvGrid: MVGrid,
  blockSize: number = 16
): void {
  const { grid_w, grid_h, mv_l0, mode } = mvGrid;
  const blockW = width / grid_w;
  const blockH = height / grid_h;

  for (let y = 0; y < grid_h; y++) {
    for (let x = 0; x < grid_w; x++) {
      const idx = y * grid_w + x;

      // Determine mode from MV data
      let interMode = InterMode.MotionVector;

      if (mode && mode[idx] !== undefined) {
        interMode = mode[idx];
      } else if (mv_l0[idx].dx_qpel === 0 && mv_l0[idx].dy_qpel === 0) {
        // Check if this is a skip or merge block
        interMode = InterMode.Merge;
      }

      const bx = Math.floor(x * blockW);
      const by = Math.floor(y * blockH);
      const bw = Math.ceil(blockW);
      const bh = Math.ceil(blockH);

      ctx.fillStyle = getInterModeColor(interMode);
      ctx.fillRect(bx, by, bw, bh);

      // Draw border
      ctx.strokeStyle = 'rgba(0, 0, 0, 0.3)';
      ctx.lineWidth = 1;
      ctx.strokeRect(bx, by, bw, bh);
    }
  }
}

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
      const by = y * blockH;

      if (saoType === 'edge') {
        // Edge offset visualization - green tint
        ctx.fillStyle = `rgba(0, 255, 100, 0.3)`;
      } else if (saoType === 'band') {
        // Band offset visualization - blue tint
        ctx.fillStyle = `rgba(100, 100, 255, 0.3)`;
      }

      ctx.fillRect(bx, by, blockW, blockH);

      // Draw SAO type indicator
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.5)';
      ctx.lineWidth = 1;
      ctx.strokeRect(bx, by, blockW, blockH);
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

/**
 * Sample values pixel inspector
 */
export interface SampleInspector {
  enabled: boolean;
  x: number;
  y: number;
  sampleY: number;
  sampleU: number;
  sampleV: number;
  rgb: string;
}

/**
 * Render sample values overlay
 */
export function renderSampleValues(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  inspector: SampleInspector,
  blockSize: number = 8
): void {
  if (!inspector.enabled) {
    return;
  }

  const gridX = Math.floor(inspector.x / blockSize);
  const gridY = Math.floor(inspector.y / blockSize);

  // Highlight inspected pixel's block
  ctx.strokeStyle = '#ffffff';
  ctx.lineWidth = 2;
  ctx.strokeRect(gridX * blockSize, gridY * blockSize, blockSize, blockSize);

  // Draw crosshair at inspected pixel
  ctx.strokeStyle = '#ff0000';
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(inspector.x, 0);
  ctx.lineTo(inspector.x, height);
  ctx.moveTo(0, inspector.y);
  ctx.lineTo(width, inspector.y);
  ctx.stroke();

  // Draw sample info box
  const padding = 8;
  const boxWidth = 140;
  const boxHeight = 100;
  const boxX = Math.min(inspector.x + 10, width - boxWidth - 10);
  const boxY = Math.min(inspector.y + 10, height - boxHeight - 10);

  ctx.fillStyle = 'rgba(0, 0, 0, 0.8)';
  ctx.fillRect(boxX, boxY, boxWidth, boxHeight);
  ctx.strokeStyle = '#ffffff';
  ctx.lineWidth = 1;
  ctx.strokeRect(boxX, boxY, boxWidth, boxHeight);

  // Draw sample values
  ctx.fillStyle = '#ffffff';
  ctx.font = '12px monospace';
  ctx.textAlign = 'left';

  let lineY = boxY + 20;
  const lineHeight = 16;

  ctx.fillText(`Position: (${inspector.x}, ${inspector.y})`, boxX + padding, lineY);
  lineY += lineHeight;

  ctx.fillText(`Y: ${inspector.sampleY}`, boxX + padding, lineY);
  lineY += lineHeight;

  ctx.fillText(`U: ${inspector.sampleU}`, boxX + padding, lineY);
  lineY += lineHeight;

  ctx.fillText(`V: ${inspector.sampleV}`, boxX + padding, lineY);
  lineY += lineHeight;

  ctx.fillStyle = inspector.rgb;
  ctx.fillRect(boxX + padding, boxY + boxHeight - 20, 40, 12);
  ctx.strokeStyle = '#ffffff';
  ctx.strokeRect(boxX + padding, boxY + boxHeight - 20, 40, 12);

  ctx.fillStyle = '#ffffff';
  ctx.fillText('RGB', boxX + padding + 4, boxY + boxHeight - 12);
}

/**
 * Get color from YUV values
 */
export function yuvToRgb(y: number, u: number, v: number): string {
  // ITU-R BT.709 conversion
  const yVal = y;
  const uVal = u - 128;
  const vVal = v - 128;

  const r = Math.round(yVal + 1.5748 * vVal);
  const g = Math.round(yVal - 0.1873 * uVal - 0.4681 * vVal);
  const b = Math.round(yVal + 1.8556 * uVal);

  const rClamped = Math.max(0, Math.min(255, r));
  const gClamped = Math.max(0, Math.min(255, g));
  const bClamped = Math.max(0, Math.min(255, b));

  return `rgb(${rClamped}, ${gClamped}, ${bClamped})`;
}

/**
 * Create sample inspector from pixel data
 */
export function createSampleInspector(
  yuvData: Uint8Array,
  width: number,
  height: number,
  x: number,
  y: number,
  enabled: boolean = true
): SampleInspector {
  if (x < 0 || x >= width || y < 0 || y >= height) {
    return {
      enabled: false,
      x: 0,
      y: 0,
      sampleY: 0,
      sampleU: 0,
      sampleV: 0,
      rgb: '#000000',
    };
  }

  const yOffset = y * width + x;
  const uvOffset = Math.floor(y / 2) * Math.floor(width / 2) + Math.floor(x / 2);

  const sampleY = yuvData[yOffset];
  const sampleU = yuvData[width * height + uvOffset];
  const sampleV = yuvData[width * height * 5 / 4 + uvOffset];

  const rgb = yuvToRgb(sampleY, sampleU, sampleV);

  return {
    enabled,
    x,
    y,
    sampleY,
    sampleU,
    sampleV,
    rgb,
  };
}
