/**
 * StreamDataContext Provider Tests
 * Tests stream data context provider for frame information
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { StreamDataProvider, useStreamData } from '../StreamDataContext';
import type { FrameInfo } from '@/types/video';

// Mock Tauri invoke
const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: mockInvoke,
}));

// Mock logger
vi.mock('@/utils/logger', () => ({
  createLogger: () => ({
    error: vi.fn(),
    warn: vi.fn(),
    info: vi.fn(),
    debug: vi.fn(),
  }),
}));

const mockFrames: FrameInfo[] = [
  { frame_index: 0, frame_type: 'I', size: 50000, poc: 0, key_frame: true },
  { frame_index: 1, frame_type: 'P', size: 30000, poc: 1, ref_frames: [0] },
  { frame_index: 2, frame_type: 'B', size: 20000, poc: 2, ref_frames: [0, 1] },
  { frame_index: 3, frame_type: 'P', size: 35000, poc: 3, ref_frames: [2], key_frame: true },
  { frame_index: 4, frame_type: 'B', size: 25000, poc: 4, ref_frames: [2, 3] },
];

describe('StreamDataContext', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <StreamDataProvider>{children}</StreamDataProvider>
  );

  it('should provide default state', async () => {
    mockInvoke.mockResolvedValue([]);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.frames).toEqual([]);
    expect(result.current.currentFrameIndex).toBe(0);
    expect(result.current.error).toBeNull();
  });

  it('should have all required methods', async () => {
    mockInvoke.mockResolvedValue([]);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(typeof result.current.setCurrentFrameIndex).toBe('function');
    expect(typeof result.current.refreshFrames).toBe('function');
    expect(typeof result.current.clearData).toBe('function');
    expect(typeof result.current.getFrameStats).toBe('function');
  });

  it('should load frames on mount', async () => {
    mockInvoke.mockResolvedValue(mockFrames);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(mockInvoke).toHaveBeenCalledWith('get_frames');
    expect(result.current.frames).toEqual(mockFrames);
  });

  it('should set loading state during refresh', async () => {
    let resolveInvoke: (value: FrameInfo[]) => void;
    mockInvoke.mockImplementation(() => {
      return new Promise(resolve => {
        resolveInvoke = resolve;
      });
    });

    const { result } = renderHook(() => useStreamData(), { wrapper });

    // Initial load
    await waitFor(() => {
      expect(result.current.loading).toBe(true);
    });

    // Resolve the initial load
    act(() => {
      resolveInvoke!(mockFrames);
    });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });
  });

  it('should handle load error', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    mockInvoke.mockRejectedValue(new Error('Failed to load frames'));

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error).toBeTruthy();
    expect(result.current.frames).toEqual([]);

    consoleSpy.mockRestore();
  });

  it('should clear error on successful load', async () => {
    mockInvoke
      .mockRejectedValueOnce(new Error('First load failed'))
      .mockResolvedValueOnce(mockFrames);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.error).toBeTruthy();
    });

    act(() => {
      result.current.refreshFrames();
    });

    await waitFor(() => {
      expect(result.current.error).toBeNull();
    });
  });
});

describe('StreamDataContext frame index management', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue(mockFrames);
  });

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <StreamDataProvider>{children}</StreamDataProvider>
  );

  it('should set current frame index', async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    act(() => {
      result.current.setCurrentFrameIndex(2);
    });

    expect(result.current.currentFrameIndex).toBe(2);
  });

  it('should allow setting frame index to 0', async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    act(() => {
      result.current.setCurrentFrameIndex(3);
      result.current.setCurrentFrameIndex(0);
    });

    expect(result.current.currentFrameIndex).toBe(0);
  });

  it('should handle setting same frame index', async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    act(() => {
      result.current.setCurrentFrameIndex(5);
      result.current.setCurrentFrameIndex(5);
    });

    expect(result.current.currentFrameIndex).toBe(5);
  });
});

describe('StreamDataContext getFrameStats', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue(mockFrames);
  });

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <StreamDataProvider>{children}</StreamDataProvider>
  );

  it('should calculate total frames count', async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    const stats = result.current.getFrameStats();

    expect(stats.totalFrames).toBe(5);
  });

  it('should count key frames', async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    const stats = result.current.getFrameStats();

    expect(stats.keyFrames).toBe(2); // frame 0 and frame 3
  });

  it('should calculate total size', async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    const stats = result.current.getFrameStats();

    expect(stats.totalSize).toBe(160000); // 50000 + 30000 + 20000 + 35000 + 25000
  });

  it('should calculate average size', async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    const stats = result.current.getFrameStats();

    expect(stats.avgSize).toBe(32000); // 160000 / 5
  });

  it('should count frame types', async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    const stats = result.current.getFrameStats();

    expect(stats.frameTypes.I).toBe(1);
    expect(stats.frameTypes.P).toBe(2);
    expect(stats.frameTypes.B).toBe(2);
  });

  it('should handle empty frames array', async () => {
    mockInvoke.mockResolvedValue([]);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    const stats = result.current.getFrameStats();

    expect(stats.totalFrames).toBe(0);
    expect(stats.totalSize).toBe(0);
    expect(stats.avgSize).toBe(0);
    expect(stats.keyFrames).toBe(0);
    expect(stats.frameTypes).toEqual({});
  });

  it('should recalculate stats when frames change', async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    const initialStats = result.current.getFrameStats();
    expect(initialStats.totalFrames).toBe(5);

    // Clear and reload with different frames
    act(() => {
      result.current.clearData();
    });

    mockInvoke.mockResolvedValue([mockFrames[0], mockFrames[1]]);

    await act(async () => {
      await result.current.refreshFrames();
    });

    const newStats = result.current.getFrameStats();
    expect(newStats.totalFrames).toBe(2);
  });
});

describe('StreamDataContext refreshFrames', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <StreamDataProvider>{children}</StreamDataProvider>
  );

  it('should refresh frames from backend', async () => {
    mockInvoke.mockResolvedValue(mockFrames);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.frames).toEqual(mockFrames);
  });

  it('should call get_frames command', async () => {
    mockInvoke.mockResolvedValue(mockFrames);

    renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('get_frames');
    });
  });

  it('should handle empty result from backend', async () => {
    mockInvoke.mockResolvedValue([]);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.frames).toEqual([]);
  });

  it('should handle null result from backend', async () => {
    mockInvoke.mockResolvedValue(null);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.frames).toEqual([]);
  });

  it('should support multiple refresh calls', async () => {
    mockInvoke.mockResolvedValue([mockFrames[0]]);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(mockInvoke).toHaveBeenCalledTimes(1);

    act(() => {
      result.current.refreshFrames();
    });

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledTimes(2);
    });
  });
});

describe('StreamDataContext clearData', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue(mockFrames);
  });

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <StreamDataProvider>{children}</StreamDataProvider>
  );

  it('should clear all data', async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.frames.length).toBeGreaterThan(0);

    act(() => {
      result.current.clearData();
    });

    expect(result.current.frames).toEqual([]);
    expect(result.current.currentFrameIndex).toBe(0);
    expect(result.current.error).toBeNull();
  });

  it('should reset loading state on clear', async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    act(() => {
      result.current.clearData();
    });

    expect(result.current.loading).toBe(false);
  });

  it('should clear error state on clear', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('Load error'));

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.error).toBeTruthy();
    });

    act(() => {
      result.current.clearData();
    });

    expect(result.current.error).toBeNull();
  });
});

describe('StreamDataContext error handling', () => {
  it('should throw error when useStreamData used outside provider', () => {
    expect(() => {
      renderHook(() => useStreamData());
    }).toThrow('useStreamData must be used within a StreamDataProvider');
  });

  it('should log error on load failure', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    mockInvoke.mockRejectedValue(new Error('Network error'));

    const wrapper = ({ children }: { children: React.ReactNode }) => (
      <StreamDataProvider>{children}</StreamDataProvider>
    );

    renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(consoleSpy).toHaveBeenCalledWith(
        expect.stringContaining('Failed to load frames:'),
        expect.anything()
      );
    });

    consoleSpy.mockRestore();
  });
});

describe('StreamDataContext edge cases', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <StreamDataProvider>{children}</StreamDataProvider>
  );

  it('should handle frames without key_frame property', async () => {
    const framesWithoutKeyFrame = [
      { frame_index: 0, frame_type: 'I', size: 50000, poc: 0 },
      { frame_index: 1, frame_type: 'P', size: 30000, poc: 1 },
    ];

    mockInvoke.mockResolvedValue(framesWithoutKeyFrame);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    const stats = result.current.getFrameStats();
    expect(stats.keyFrames).toBe(0);
  });

  it('should handle frames with zero size', async () => {
    const framesWithZeroSize = [
      { frame_index: 0, frame_type: 'I', size: 0, poc: 0 },
      { frame_index: 1, frame_type: 'P', size: 30000, poc: 1 },
    ];

    mockInvoke.mockResolvedValue(framesWithZeroSize);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    const stats = result.current.getFrameStats();
    expect(stats.totalSize).toBe(30000);
    expect(stats.avgSize).toBe(15000);
  });

  it('should handle very large frame counts', async () => {
    const largeFrames = Array.from({ length: 10000 }, (_, i) => ({
      frame_index: i,
      frame_type: 'I',
      size: 50000,
      poc: i,
    })) as FrameInfo[];

    mockInvoke.mockResolvedValue(largeFrames);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    const stats = result.current.getFrameStats();
    expect(stats.totalFrames).toBe(10000);
  });

  it('should handle frames with all frame types', async () => {
    const mixedFrames = [
      { frame_index: 0, frame_type: 'I', size: 50000, poc: 0 },
      { frame_index: 1, frame_type: 'P', size: 30000, poc: 1 },
      { frame_index: 2, frame_type: 'B', size: 20000, poc: 2 },
      { frame_index: 3, frame_type: 'KEY', size: 60000, poc: 3 },
      { frame_index: 4, frame_type: 'INTRA', size: 55000, poc: 4 },
      { frame_index: 5, frame_type: 'INTER', size: 40000, poc: 5 },
    ] as FrameInfo[];

    mockInvoke.mockResolvedValue(mixedFrames);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    const stats = result.current.getFrameStats();
    expect(stats.frameTypes.I).toBe(1);
    expect(stats.frameTypes.P).toBe(1);
    expect(stats.frameTypes.B).toBe(1);
    expect(stats.frameTypes.KEY).toBe(1);
    expect(stats.frameTypes.INTRA).toBe(1);
    expect(stats.frameTypes.INTER).toBe(1);
  });

  it('should handle frames with ref_frames array', async () => {
    const framesWithRefs = [
      { frame_index: 0, frame_type: 'I', size: 50000, poc: 0 },
      { frame_index: 1, frame_type: 'P', size: 30000, poc: 1, ref_frames: [0] },
      { frame_index: 2, frame_type: 'B', size: 20000, poc: 2, ref_frames: [0, 1] },
      { frame_index: 3, frame_type: 'B', size: 25000, poc: 3, ref_frames: [1, 2] },
    ] as FrameInfo[];

    mockInvoke.mockResolvedValue(framesWithRefs);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.frames[1].ref_frames).toEqual([0]);
    expect(result.current.frames[2].ref_frames).toEqual([0, 1]);
  });

  it('should handle negative frame index', async () => {
    mockInvoke.mockResolvedValue(mockFrames);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    act(() => {
      result.current.setCurrentFrameIndex(-1);
    });

    expect(result.current.currentFrameIndex).toBe(-1);
  });

  it('should handle very large frame index', async () => {
    mockInvoke.mockResolvedValue(mockFrames);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    act(() => {
      result.current.setCurrentFrameIndex(999999);
    });

    expect(result.current.currentFrameIndex).toBe(999999);
  });
});

describe('StreamDataContext React stability', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue(mockFrames);
  });

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <StreamDataProvider>{children}</StreamDataProvider>
  );

  it('should provide stable callbacks across renders', async () => {
    const { result, rerender } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    const setCurrentFrameIndexRef = result.current.setCurrentFrameIndex;
    const refreshFramesRef = result.current.refreshFrames;
    const clearDataRef = result.current.clearData;
    const getFrameStatsRef = result.current.getFrameStats;

    rerender();

    expect(result.current.setCurrentFrameIndex).toBe(setCurrentFrameIndexRef);
    expect(result.current.refreshFrames).toBe(refreshFramesRef);
    expect(result.current.clearData).toBe(clearDataRef);
    expect(result.current.getFrameStats).toBe(getFrameStatsRef);
  });

  it('should update stats when frames change', async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    const initialStats = result.current.getFrameStats();

    act(() => {
      result.current.clearData();
    });

    mockInvoke.mockResolvedValue([mockFrames[0]]);

    await act(async () => {
      await result.current.refreshFrames();
    });

    const newStats = result.current.getFrameStats();

    expect(newStats.totalFrames).not.toBe(initialStats.totalFrames);
  });
});

describe('StreamDataContext frame type variations', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <StreamDataProvider>{children}</StreamDataProvider>
  );

  it('should handle lowercase frame types', async () => {
    const lowercaseFrames = [
      { frame_index: 0, frame_type: 'i', size: 50000, poc: 0 },
      { frame_index: 1, frame_type: 'p', size: 30000, poc: 1 },
      { frame_index: 2, frame_type: 'b', size: 20000, poc: 2 },
    ] as FrameInfo[];

    mockInvoke.mockResolvedValue(lowercaseFrames);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    const stats = result.current.getFrameStats();
    expect(stats.frameTypes.i).toBe(1);
    expect(stats.frameTypes.p).toBe(1);
    expect(stats.frameTypes.b).toBe(1);
  });

  it('should handle mixed case frame types', async () => {
    const mixedCaseFrames = [
      { frame_index: 0, frame_type: 'I', size: 50000, poc: 0 },
      { frame_index: 1, frame_type: 'p', size: 30000, poc: 1 },
      { frame_index: 2, frame_type: 'B', size: 20000, poc: 2 },
    ] as FrameInfo[];

    mockInvoke.mockResolvedValue(mixedCaseFrames);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    const stats = result.current.getFrameStats();
    expect(stats.frameTypes.I).toBe(1);
    expect(stats.frameTypes.p).toBe(1);
    expect(stats.frameTypes.B).toBe(1);
  });
});
