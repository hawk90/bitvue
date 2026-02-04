/**
 * Frame Navigation Utility Tests
 */

import { describe, it, expect } from 'vitest';
import {
  findNextKeyframe,
  findPrevKeyframe,
  findNextFrameByType,
  findPrevFrameByType,
  findFrameByNumber,
} from '../frameNavigation';
import { mockFrames } from '@/test/test-utils';

describe('findNextKeyframe', () => {
  it('should find the next keyframe', () => {
    const result = findNextKeyframe(mockFrames, 0);
    // No more keyframes after frame 0 in our mock data
    expect(result).toBeNull();
  });

  it('should return null if no next keyframe exists', () => {
    const frames = [
      { frame_index: 0, frame_type: 'P', size: 1000 },
      { frame_index: 1, frame_type: 'P', size: 1000 },
    ] as any;
    const result = findNextKeyframe(frames, 0);
    expect(result).toBeNull();
  });

  it('should handle empty frames array', () => {
    const result = findNextKeyframe([], 0);
    expect(result).toBeNull();
  });
});

describe('findPrevKeyframe', () => {
  it('should find the previous keyframe', () => {
    const frames = [
      { frame_index: 0, frame_type: 'I', size: 1000 },
      { frame_index: 1, frame_type: 'P', size: 1000 },
      { frame_index: 2, frame_type: 'P', size: 1000 },
      { frame_index: 3, frame_type: 'I', size: 1000 },
    ] as any;

    const result = findPrevKeyframe(frames, 3);
    expect(result).toBe(0);
  });

  it('should return null if no previous keyframe exists', () => {
    const result = findPrevKeyframe(mockFrames, 0);
    expect(result).toBeNull();
  });

  it('should handle empty frames array', () => {
    const result = findPrevKeyframe([], 0);
    expect(result).toBeNull();
  });
});

describe('findNextFrameByType', () => {
  it('should find the next frame of specified type', () => {
    const frames = [
      { frame_index: 0, frame_type: 'I', size: 1000 },
      { frame_index: 1, frame_type: 'P', size: 1000 },
      { frame_index: 2, frame_type: 'B', size: 1000 },
      { frame_index: 3, frame_type: 'P', size: 1000 },
    ] as any;

    const result = findNextFrameByType(frames, 1, 'P');
    expect(result).toBe(3);
  });

  it('should return null if no next frame of type exists', () => {
    const result = findNextFrameByType(mockFrames, 2, 'I');
    expect(result).toBeNull();
  });

  it('should be case sensitive', () => {
    const frames = [
      { frame_index: 0, frame_type: 'i', size: 1000 },
      { frame_index: 1, frame_type: 'P', size: 1000 },
    ] as any;

    const result = findNextFrameByType(frames, 0, 'I');
    expect(result).toBeNull();
  });
});

describe('findPrevFrameByType', () => {
  it('should find the previous frame of specified type', () => {
    const frames = [
      { frame_index: 0, frame_type: 'I', size: 1000 },
      { frame_index: 1, frame_type: 'P', size: 1000 },
      { frame_index: 2, frame_type: 'P', size: 1000 },
      { frame_index: 3, frame_type: 'B', size: 1000 },
    ] as any;

    const result = findPrevFrameByType(frames, 3, 'P');
    expect(result).toBe(2);
  });

  it('should return null if no previous frame of type exists', () => {
    const result = findPrevFrameByType(mockFrames, 0, 'P');
    expect(result).toBeNull();
  });
});

describe('findFrameByNumber', () => {
  it('should find frame by exact number', () => {
    const result = findFrameByNumber(mockFrames, 1);
    expect(result).toBe(1);
  });

  it('should return null if frame number not found', () => {
    const result = findFrameByNumber(mockFrames, 999);
    expect(result).toBeNull();
  });

  it('should handle empty frames array', () => {
    const result = findFrameByNumber([], 0);
    expect(result).toBeNull();
  });

  it('should find first frame', () => {
    const result = findFrameByNumber(mockFrames, 0);
    expect(result).toBe(0);
  });

  it('should find last frame', () => {
    const result = findFrameByNumber(mockFrames, 2);
    expect(result).toBe(2);
  });
});
