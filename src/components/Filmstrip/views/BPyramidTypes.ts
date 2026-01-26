/**
 * B-Pyramid Types
 *
 * Shared types for B-Pyramid visualization components
 */

import type { FrameInfo } from '../../../types/video';

/** B-Pyramid view props */
export interface BPyramidViewProps {
  frames: FrameInfo[];
  currentFrameIndex: number;
  onFrameClick: (frameIndex: number) => void;
  getFrameTypeColorClass: (frameType: string) => string;
}

/** Frame with temporal level information */
export interface FrameWithLevel {
  index: number;
  frameIndex: number;
  frameType: string;
  refFrames: number[];
  level: number;
  isKeyframe: boolean;
}

/** Temporal level containing frames */
export interface TemporalLevel {
  level: number;
  frames: FrameWithLevel[];
}

/** Arrow path data for SVG rendering */
export interface ArrowPath {
  d: string;
  color: string;
  highlighted: boolean;
}

/** Result of temporal level analysis */
export interface TemporalAnalysis {
  levels: TemporalLevel[];
  gopBoundaries: number[];
  frameMap: Map<number, FrameWithLevel>;
  framePositions: Map<number, { x: number; y: number }>;
}
