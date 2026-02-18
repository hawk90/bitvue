/**
 * Frame Navigation Utilities
 *
 * Helper functions for frame navigation
 */

import type { FrameInfo } from "../types/video";

/**
 * Validate frame index is within bounds
 */
function isValidIndex(frames: FrameInfo[], index: number): boolean {
  return index >= 0 && index < frames.length;
}

/**
 * Find the next keyframe (I or KEY frame type) starting from given index
 */
export function findNextKeyframe(
  frames: FrameInfo[],
  fromIndex: number,
): number | null {
  if (!isValidIndex(frames, fromIndex)) {
    return null;
  }

  for (let i = fromIndex + 1; i < frames.length; i++) {
    const frame = frames[i];
    if (
      frame.frame_type === "I" ||
      frame.frame_type === "KEY" ||
      frame.key_frame
    ) {
      return i;
    }
  }
  return null;
}

/**
 * Find the previous keyframe (I or KEY frame type) starting from given index
 */
export function findPrevKeyframe(
  frames: FrameInfo[],
  fromIndex: number,
): number | null {
  if (!isValidIndex(frames, fromIndex)) {
    return null;
  }

  for (let i = fromIndex - 1; i >= 0; i--) {
    const frame = frames[i];
    if (
      frame.frame_type === "I" ||
      frame.frame_type === "KEY" ||
      frame.key_frame
    ) {
      return i;
    }
  }
  return null;
}

/**
 * Find the first frame of a specific type starting from given index
 */
export function findNextFrameByType(
  frames: FrameInfo[],
  fromIndex: number,
  frameType: string,
): number | null {
  if (!isValidIndex(frames, fromIndex)) {
    return null;
  }

  for (let i = fromIndex + 1; i < frames.length; i++) {
    if (frames[i].frame_type === frameType) {
      return i;
    }
  }
  return null;
}

/**
 * Find the previous frame of a specific type starting from given index
 */
export function findPrevFrameByType(
  frames: FrameInfo[],
  fromIndex: number,
  frameType: string,
): number | null {
  if (!isValidIndex(frames, fromIndex)) {
    return null;
  }

  for (let i = fromIndex - 1; i >= 0; i--) {
    if (frames[i].frame_type === frameType) {
      return i;
    }
  }
  return null;
}

/**
 * Find frame by frame number
 */
export function findFrameByNumber(
  frames: FrameInfo[],
  frameNumber: number,
): number | null {
  if (frames.length === 0) {
    return null;
  }

  for (let i = 0; i < frames.length; i++) {
    if (frames[i].frame_index === frameNumber) {
      return i;
    }
  }
  return null;
}

/**
 * Find frames matching a search query
 */
export function searchFrames(
  frames: FrameInfo[],
  query: string,
): { index: number; frame: FrameInfo; matchType: string }[] {
  if (!query.trim()) {
    return [];
  }

  const lowerQuery = query.toLowerCase();
  const results: { index: number; frame: FrameInfo; matchType: string }[] = [];

  frames.forEach((frame, idx) => {
    // Match by frame type
    if (frame.frame_type.toLowerCase().includes(lowerQuery)) {
      results.push({ index: idx, frame, matchType: "type" });
      return;
    }

    // Match by frame number
    if (String(frame.frame_index).includes(lowerQuery)) {
      results.push({ index: idx, frame, matchType: "number" });
      return;
    }

    // Match by PTS
    if (frame.pts !== undefined && String(frame.pts).includes(lowerQuery)) {
      results.push({ index: idx, frame, matchType: "pts" });
      return;
    }

    // Match by POC
    if (frame.poc !== undefined && String(frame.poc).includes(lowerQuery)) {
      results.push({ index: idx, frame, matchType: "poc" });
      return;
    }
  });

  return results;
}

/**
 * Get all keyframe indices
 */
export function getKeyframeIndices(frames: FrameInfo[]): number[] {
  const indices: number[] = [];
  frames.forEach((frame, idx) => {
    if (
      frame.frame_type === "I" ||
      frame.frame_type === "KEY" ||
      frame.key_frame
    ) {
      indices.push(idx);
    }
  });
  return indices;
}

/**
 * Calculate frame range info
 */
export function getFrameRangeInfo(
  frames: FrameInfo[],
  startIndex: number,
  endIndex: number,
) {
  const rangeFrames = frames.slice(
    Math.max(0, startIndex),
    Math.min(frames.length, endIndex + 1),
  );
  const totalSize = rangeFrames.reduce((sum, f) => sum + f.size, 0);
  const avgSize = rangeFrames.length > 0 ? totalSize / rangeFrames.length : 0;

  const typeCounts = rangeFrames.reduce(
    (acc, frame) => {
      acc[frame.frame_type] = (acc[frame.frame_type] || 0) + 1;
      return acc;
    },
    {} as Record<string, number>,
  );

  return {
    count: rangeFrames.length,
    totalSize,
    avgSize,
    typeCounts,
  };
}
