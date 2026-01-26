/**
 * Overlay Renderer Types
 *
 * Type definitions for the mode overlay rendering system
 */

import type { VisualizationMode } from '../../../contexts/ModeContext';
import type { FrameInfo } from '../../../types/video';

/**
 * Color stop for gradient interpolation
 */
export interface ColorStop {
  t: number;      // Position (0.0 to 1.0)
  r: number;      // Red component (0-255)
  g: number;      // Green component (0-255)
  b: number;      // Blue component (0-255)
}

/**
 * Main render options for mode overlay
 */
export interface OverlayRenderOptions {
  mode: VisualizationMode;
  frame: FrameInfo | null;
  canvas: HTMLCanvasElement;
  ctx: CanvasRenderingContext2D;
}

/**
 * Props for individual overlay renderers
 */
export interface OverlayRendererProps {
  ctx: CanvasRenderingContext2D;
  width: number;
  height: number;
  frame: FrameInfo;
}

/**
 * Legend item for overlay visualization
 */
export interface LegendItem {
  color: string;
  label: string;
}
