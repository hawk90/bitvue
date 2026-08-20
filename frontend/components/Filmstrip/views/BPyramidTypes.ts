/**
 * B-Pyramid Types
 *
 * Shared types for B-Pyramid visualization components
 */

import type { FrameInfo } from "../../../types/video";

/** B-Pyramid view props */
export interface BPyramidViewProps {
  frames: FrameInfo[];
  currentFrameIndex: number;
  onFrameClick: (frameIndex: number) => void;
  getFrameTypeColorClass: (frameType: string) => string;
  /** When true, renders every frame's reference arrows simultaneously (dimmed), instead of only
   * the currently-selected frame's -- lets the "Show All Refs" toggle reveal the whole GOP
   * dependency graph at a glance. Defaults to false (original per-selection behavior, kept
   * because full-graph rendering gets visually noisy at high frame counts). */
  showAllArrows?: boolean;
  /** Called when the user clicks the view's "Show All Refs" toggle. Omit to hide the toggle
   * (e.g. when a caller wants a fixed showAllArrows value with no user control). */
  onToggleShowAllArrows?: () => void;
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
