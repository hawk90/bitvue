/**
 * Overlay Renderer Types
 *
 * Type definitions for the mode overlay rendering system
 */

import type { VisualizationMode } from "../../../contexts/ModeContext";
import type { FrameInfo } from "../../../types/video";

/**
 * Color stop for gradient interpolation
 */
export interface ColorStop {
  t: number; // Position (0.0 to 1.0)
  r: number; // Red component (0-255)
  g: number; // Green component (0-255)
  b: number; // Blue component (0-255)
}

/**
 * AV1 CDEF block data
 */
export interface Av1CdefBlock {
  x: number;
  y: number;
  size: number;
  direction: number;
  strength: number;
}

/**
 * AV1 CDEF filter data for a frame
 */
export interface Av1CdefData {
  width: number;
  height: number;
  blockSize: number;
  blocks: Av1CdefBlock[];
  damping: number;
  yPrimaryStrength: number;
  ySecondaryStrength: number;
}

/**
 * AV1 loop restoration unit
 */
export interface Av1RestorationUnit {
  x: number;
  y: number;
  size: number;
  /** 0=None, 1=Wiener, 2=SgrProj, 3=Dual */
  restorationType: number;
}

/**
 * AV1 loop restoration data for a frame
 */
export interface Av1LoopRestorationData {
  width: number;
  height: number;
  unitSize: number;
  /** 0=None, 1=Wiener, 2=SgrProj, 3=Dual */
  yType: number;
  units: Av1RestorationUnit[];
}

/**
 * AV1 film grain synthesis parameters
 */
export interface Av1FilmGrainData {
  enabled: boolean;
  seed: number;
  scalingShift: number;
  arCoeffLag: number;
  chromaScalingFromLuma: boolean;
  overlap: boolean;
}

/**
 * AV1 super resolution parameters
 */
export interface Av1SuperResData {
  enabled: boolean;
  scaleDenominator: number;
  upscaledWidth: number;
  upscaledHeight: number;
}

/**
 * Combined AV1 advanced features data returned by get_av1_features
 */
export interface Av1FeaturesData {
  frameIndex: number;
  cdef?: Av1CdefData;
  loopRestoration?: Av1LoopRestorationData;
  filmGrain?: Av1FilmGrainData;
  superResolution?: Av1SuperResData;
}

/**
 * Main render options for mode overlay
 */
export interface OverlayRenderOptions {
  mode: VisualizationMode;
  frame: FrameInfo | null;
  canvas: HTMLCanvasElement;
  ctx: CanvasRenderingContext2D;
  /** AV1 advanced feature data (CDEF, LR, film-grain, super-res) */
  av1Features?: Av1FeaturesData;
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
