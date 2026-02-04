/**
 * Advanced Visualization Renderers - Type Definitions
 *
 * Types for the advanced visualization system
 */

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
 * Sample inspector for pixel values
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
