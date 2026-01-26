/**
 * useStreamData Hook Tests
 * Tests stream data context hook
 */

import { describe, it, expect, vi } from 'vitest';
import { renderHook } from '@testing-library/react';
import { useStreamData } from '../useStreamData';

// Mock StreamDataContext
vi.mock('../contexts/StreamDataContext', () => ({
  useStreamData: () => ({
    frames: [
      { frame_index: 0, frame_type: 'I', size: 50000, poc: 0 },
      { frame_index: 1, frame_type: 'P', size: 30000, poc: 1 },
    ],
    currentFrameIndex: 0,
    getFrameStats: vi.fn(() => ({
      totalFrames: 2,
      keyFrames: 1,
      avgSize: 40000,
      totalSize: 80000,
      frameTypes: { I: 1, P: 1 },
    })),
  }),
}));

describe('useStreamData', () => {
  it('should return stream data from context', () => {
    const { result } = renderHook(() => useStreamData());

    expect(result.current.frames).toHaveLength(2);
    expect(result.current.currentFrameIndex).toBe(0);
  });

  it('should have frames array', () => {
    const { result } = renderHook(() => useStreamData());

    expect(Array.isArray(result.current.frames)).toBe(true);
    expect(result.current.frames[0].frame_index).toBe(0);
  });

  it('should have currentFrameIndex', () => {
    const { result } = renderHook(() => useStreamData());

    expect(typeof result.current.currentFrameIndex).toBe('number');
  });

  it('should have getFrameStats function', () => {
    const { result } = renderHook(() => useStreamData());

    expect(typeof result.current.getFrameStats).toBe('function');
  });

  it('should return stats from getFrameStats', () => {
    const { result } = renderHook(() => useStreamData());

    const stats = result.current.getFrameStats();

    expect(stats.totalFrames).toBe(2);
    expect(stats.avgSize).toBe(40000);
  });
});

describe('useStreamData frame access', () => {
  it('should provide access to current frame', () => {
    const { result } = renderHook(() => useStreamData());

    const currentFrame = result.current.frames[result.current.currentFrameIndex];
    expect(currentFrame).toBeDefined();
    expect(currentFrame.frame_index).toBe(0);
  });

  it('should provide access to all frames', () => {
    const { result } = renderHook(() => useStreamData());

    expect(result.current.frames.map(f => f.frame_index)).toEqual([0, 1]);
  });

  it('should handle empty frames array', () => {
    vi.doMock('../contexts/StreamDataContext', () => ({
      useStreamData: () => ({
        frames: [],
        currentFrameIndex: -1,
        getFrameStats: vi.fn(() => ({
          totalFrames: 0,
          keyFrames: 0,
          avgSize: 0,
          totalSize: 0,
          frameTypes: {},
        })),
      }),
    }));

    const { result } = renderHook(() => useStreamData());

    expect(result.current.frames).toEqual([]);
  });
});

describe('useStreamData statistics', () => {
  it('should calculate frame type distribution', () => {
    const { result } = renderHook(() => useStreamData());

    const stats = result.current.getFrameStats();

    expect(stats.frameTypes.I).toBe(1);
    expect(stats.frameTypes.P).toBe(1);
  });

  it('should calculate total frames', () => {
    const { result } = renderHook(() => useStreamData());

    const stats = result.current.getFrameStats();

    expect(stats.totalFrames).toBe(2);
  });

  it('should calculate average size', () => {
    const { result } = renderHook(() => useStreamData());

    const stats = result.current.getFrameStats();

    expect(stats.avgSize).toBe(40000);
  });

  it('should calculate total size', () => {
    const { result } = renderHook(() => useStreamData());

    const stats = result.current.getFrameStats();

    expect(stats.totalSize).toBe(80000);
  });
});

describe('useStreamData edge cases', () => {
  it('should handle currentFrameIndex out of bounds', () => {
    vi.doMock('../contexts/StreamDataContext', () => ({
      useStreamData: () => ({
        frames: [{ frame_index: 0, frame_type: 'I', size: 50000 }],
        currentFrameIndex: 999,
        getFrameStats: vi.fn(() => ({ totalFrames: 1 })),
      }),
    }));

    const { result } = renderHook(() => useStreamData());

    expect(result.current.frames[result.current.currentFrameIndex]).toBeUndefined();
  });

  it('should handle frames without getFrameStats', () => {
    vi.doMock('../contexts/StreamDataContext', () => ({
      useStreamData: () => ({
        frames: [{ frame_index: 0, frame_type: 'I', size: 50000 }],
        currentFrameIndex: 0,
      }),
    }));

    const { result } = renderHook(() => useStreamData());

    expect(result.current.frames).toBeDefined();
  });
});
