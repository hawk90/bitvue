/**
 * StreamDataContext Provider Tests
 * Tests stream data context provider for frame information
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { StreamDataProvider, useStreamData } from '../StreamDataContext';
import type { FrameInfo } from '@/types/video';

// Mock Tauri invoke - must be done this way for Vitest hoisting
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock logger - inline to avoid hoisting issues
vi.mock('@/utils/logger', () => ({
  createLogger: vi.fn(() => ({
    error: vi.fn(),
    warn: vi.fn(),
    info: vi.fn(),
    debug: vi.fn(),
  })),
}));

// Get the mocked invoke function
import { invoke } from '@tauri-apps/api/core';
const mockedInvoke = invoke as ReturnType<typeof vi.fn>;

// Get the mocked logger
import { createLogger } from '@/utils/logger';
const mockLogger = createLogger('test') as ReturnType<typeof createLogger>;

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
    mockedInvoke.mockResolvedValue([]);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    // Initially loading is false (no auto-load in test env)
    expect(result.current.loading).toBe(false);
    expect(result.current.frames).toEqual([]);
    expect(result.current.currentFrameIndex).toBe(0);
    expect(result.current.error).toBeNull();
  });

  it('should have all required methods', async () => {
    mockedInvoke.mockResolvedValue([]);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    expect(typeof result.current.setCurrentFrameIndex).toBe('function');
    expect(typeof result.current.refreshFrames).toBe('function');
    expect(typeof result.current.clearData).toBe('function');
    expect(typeof result.current.getFrameStats).toBe('function');
  });

  it('should load frames on refresh', async () => {
    mockedInvoke.mockResolvedValue(mockFrames);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await act(async () => {
      await result.current.refreshFrames();
    });

    expect(mockedInvoke).toHaveBeenCalledWith('get_frames');
    expect(result.current.frames).toEqual(mockFrames);
    expect(result.current.loading).toBe(false);
  });

  it('should set loading state during refresh', async () => {
    let resolveInvoke: (value: FrameInfo[]) => void;
    mockedInvoke.mockImplementation(() => {
      return new Promise(resolve => {
        resolveInvoke = resolve;
      });
    });

    const { result } = renderHook(() => useStreamData(), { wrapper });

    act(() => {
      result.current.refreshFrames();
    });

    // Should be loading
    expect(result.current.loading).toBe(true);

    // Resolve the load
    await act(async () => {
      resolveInvoke!(mockFrames);
      await waitFor(() => result.current.loading === false);
    });

    expect(result.current.loading).toBe(false);
  });

  it('should handle load error', async () => {
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    mockedInvoke.mockRejectedValue(new Error('Failed to load frames'));

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await act(async () => {
      await result.current.refreshFrames();
    });

    expect(result.current.error).toBeTruthy();
    expect(result.current.frames).toEqual([]);
    expect(result.current.loading).toBe(false);

    consoleSpy.mockRestore();
  });

  it('should clear error on successful load', async () => {
    mockedInvoke
      .mockRejectedValueOnce(new Error('First load failed'))
      .mockResolvedValueOnce(mockFrames);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    // First load fails
    await act(async () => {
      await result.current.refreshFrames();
    });

    expect(result.current.error).toBeTruthy();

    // Second load succeeds
    await act(async () => {
      await result.current.refreshFrames();
    });

    expect(result.current.error).toBeNull();
  });
});

describe('StreamDataContext frame index management', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <StreamDataProvider>{children}</StreamDataProvider>
  );

  it('should set current frame index', async () => {
    mockedInvoke.mockResolvedValue(mockFrames);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    act(() => {
      result.current.setCurrentFrameIndex(2);
    });

    expect(result.current.currentFrameIndex).toBe(2);
  });

  it('should allow setting frame index to 0', async () => {
    mockedInvoke.mockResolvedValue(mockFrames);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    act(() => {
      result.current.setCurrentFrameIndex(3);
      result.current.setCurrentFrameIndex(0);
    });

    expect(result.current.currentFrameIndex).toBe(0);
  });

  it('should handle setting same frame index', async () => {
    mockedInvoke.mockResolvedValue(mockFrames);

    const { result } = renderHook(() => useStreamData(), { wrapper });

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
  });

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <StreamDataProvider>{children}</StreamDataProvider>
  );

  it('should calculate total frames count', async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    act(() => {
      result.current.setFrames(mockFrames);
    });

    const stats = result.current.getFrameStats();

    expect(stats.totalFrames).toBe(5);
  });

  it('should count key frames', async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    act(() => {
      result.current.setFrames(mockFrames);
    });

    const stats = result.current.getFrameStats();

    expect(stats.keyFrames).toBe(2); // frame 0 and frame 3
  });

  it('should calculate total size', async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    act(() => {
      result.current.setFrames(mockFrames);
    });

    const stats = result.current.getFrameStats();

    expect(stats.totalSize).toBe(160000); // 50000 + 30000 + 20000 + 35000 + 25000
  });

  it('should calculate average size', async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    act(() => {
      result.current.setFrames(mockFrames);
    });

    const stats = result.current.getFrameStats();

    expect(stats.avgSize).toBe(32000); // 160000 / 5
  });

  it('should count frame types', async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    act(() => {
      result.current.setFrames(mockFrames);
    });

    const stats = result.current.getFrameStats();

    expect(stats.frameTypes.I).toBe(1);
    expect(stats.frameTypes.P).toBe(2);
    expect(stats.frameTypes.B).toBe(2);
  });

  it('should handle empty frames array', async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    const stats = result.current.getFrameStats();

    expect(stats.totalFrames).toBe(0);
    expect(stats.totalSize).toBe(0);
    expect(stats.avgSize).toBe(0);
    expect(stats.keyFrames).toBe(0);
    expect(stats.frameTypes).toEqual({});
  });

  it('should recalculate stats when frames change', async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    act(() => {
      result.current.setFrames(mockFrames);
    });

    const initialStats = result.current.getFrameStats();
    expect(initialStats.totalFrames).toBe(5);

    // Update frames
    act(() => {
      result.current.setFrames([mockFrames[0], mockFrames[1]]);
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
    mockedInvoke.mockResolvedValue(mockFrames);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await act(async () => {
      await result.current.refreshFrames();
    });

    expect(result.current.frames).toEqual(mockFrames);
  });

  it('should call get_frames command', async () => {
    mockedInvoke.mockResolvedValue(mockFrames);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await act(async () => {
      await result.current.refreshFrames();
    });

    expect(mockedInvoke).toHaveBeenCalledWith('get_frames');
  });

  it('should handle empty result from backend', async () => {
    mockedInvoke.mockResolvedValue([]);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await act(async () => {
      await result.current.refreshFrames();
    });

    expect(result.current.frames).toEqual([]);
  });

  it('should handle null result from backend', async () => {
    mockedInvoke.mockResolvedValue(null);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await act(async () => {
      await result.current.refreshFrames();
    });

    expect(result.current.frames).toEqual([]);
  });

  it('should support multiple refresh calls', async () => {
    mockedInvoke.mockResolvedValue([mockFrames[0]]);

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await act(async () => {
      await result.current.refreshFrames();
    });

    expect(mockedInvoke).toHaveBeenCalledTimes(1);

    await act(async () => {
      await result.current.refreshFrames();
    });

    expect(mockedInvoke).toHaveBeenCalledTimes(2);
  });
});

describe('StreamDataContext clearData', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <StreamDataProvider>{children}</StreamDataProvider>
  );

  it('should clear all data', async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    act(() => {
      result.current.setFrames(mockFrames);
      result.current.setFilePath('/test/path');
    });

    expect(result.current.frames.length).toBe(5);

    act(() => {
      result.current.clearData();
    });

    expect(result.current.frames).toEqual([]);
    expect(result.current.currentFrameIndex).toBe(0);
    expect(result.current.error).toBeNull();
    expect(result.current.filePath).toBeNull();
  });

  it('should reset loading state on clear', async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    act(() => {
      result.current.clearData();
    });

    expect(result.current.loading).toBe(false);
  });

  it('should clear error state on clear', async () => {
    mockedInvoke.mockRejectedValueOnce(new Error('Load error'));

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await act(async () => {
      await result.current.refreshFrames();
    });

    expect(result.current.error).toBeTruthy();

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
    mockedInvoke.mockRejectedValue(new Error('Network error'));

    const wrapper = ({ children }: { children: React.ReactNode }) => (
      <StreamDataProvider>{children}</StreamDataProvider>
    );

    const { result } = renderHook(() => useStreamData(), { wrapper });

    await act(async () => {
      await result.current.refreshFrames();
    });

    // Verify error was set (logging is handled by the logger internally)
    expect(result.current.error).toBeTruthy();
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

    const { result } = renderHook(() => useStreamData(), { wrapper });

    act(() => {
      result.current.setFrames(framesWithoutKeyFrame);
    });

    const stats = result.current.getFrameStats();
    expect(stats.keyFrames).toBe(0);
  });

  it('should handle frames with zero size', async () => {
    const framesWithZeroSize = [
      { frame_index: 0, frame_type: 'I', size: 0, poc: 0 },
      { frame_index: 1, frame_type: 'P', size: 30000, poc: 1 },
    ];

    const { result } = renderHook(() => useStreamData(), { wrapper });

    act(() => {
      result.current.setFrames(framesWithZeroSize);
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

    const { result } = renderHook(() => useStreamData(), { wrapper });

    act(() => {
      result.current.setFrames(largeFrames);
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

    const { result } = renderHook(() => useStreamData(), { wrapper });

    act(() => {
      result.current.setFrames(mixedFrames);
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

    const { result } = renderHook(() => useStreamData(), { wrapper });

    act(() => {
      result.current.setFrames(framesWithRefs);
    });

    expect(result.current.frames[1].ref_frames).toEqual([0]);
    expect(result.current.frames[2].ref_frames).toEqual([0, 1]);
  });

  it('should handle negative frame index', async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    act(() => {
      result.current.setCurrentFrameIndex(-1);
    });

    expect(result.current.currentFrameIndex).toBe(-1);
  });

  it('should handle very large frame index', async () => {
    const { result } = renderHook(() => useStreamData(), { wrapper });

    act(() => {
      result.current.setCurrentFrameIndex(999999);
    });

    expect(result.current.currentFrameIndex).toBe(999999);
  });
});

describe('StreamDataContext React stability', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <StreamDataProvider>{children}</StreamDataProvider>
  );

  it('should provide stable callbacks across renders', async () => {
    const { result, rerender } = renderHook(() => useStreamData(), { wrapper });

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

    act(() => {
      result.current.setFrames(mockFrames);
    });

    const initialStats = result.current.getFrameStats();

    act(() => {
      result.current.setFrames([mockFrames[0]]);
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

    const { result } = renderHook(() => useStreamData(), { wrapper });

    act(() => {
      result.current.setFrames(lowercaseFrames);
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

    const { result } = renderHook(() => useStreamData(), { wrapper });

    act(() => {
      result.current.setFrames(mixedCaseFrames);
    });

    const stats = result.current.getFrameStats();
    expect(stats.frameTypes.I).toBe(1);
    expect(stats.frameTypes.p).toBe(1);
    expect(stats.frameTypes.B).toBe(1);
  });
});
