/**
 * Stream Data Context
 *
 * Provides stream/frame data to all panels
 */

import { createContext, useContext, useState, useCallback, ReactNode, useEffect, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { FrameInfo } from '../types/video';
import { createLogger } from '../utils/logger';

const logger = createLogger('StreamDataContext');

/**
 * Count frame types in the frame array
 */
function countFrameTypes(frames: FrameInfo[]): Record<string, number> {
  const frameTypes: Record<string, number> = {};
  for (const frame of frames) {
    frameTypes[frame.frame_type] = (frameTypes[frame.frame_type] || 0) + 1;
  }
  return frameTypes;
}

/**
 * Calculate total size of all frames
 */
function calculateTotalSize(frames: FrameInfo[]): number {
  let totalSize = 0;
  for (const frame of frames) {
    totalSize += frame.size;
  }
  return totalSize;
}

/**
 * Count keyframes in the frame array
 */
function countKeyframes(frames: FrameInfo[]): number {
  let keyFrames = 0;
  for (const frame of frames) {
    if (frame.key_frame) {
      keyFrames++;
    }
  }
  return keyFrames;
}

/**
 * Calculate frame statistics from an array of frames
 */
function calculateFrameStats(frames: FrameInfo[]): FrameStats {
  const totalFrames = frames.length;
  const totalSize = calculateTotalSize(frames);
  const avgSize = totalFrames > 0 ? totalSize / totalFrames : 0;

  return {
    totalFrames,
    frameTypes: countFrameTypes(frames),
    totalSize,
    avgSize,
    keyFrames: countKeyframes(frames),
  };
}

interface StreamDataContextType {
  frames: FrameInfo[];
  filePath: string | null;
  loading: boolean;
  error: string | null;
  currentFrameIndex: number;
  setCurrentFrameIndex: (index: number) => void;
  refreshFrames: () => Promise<void>;
  clearData: () => void;
  getFrameStats: () => FrameStats;
  setFilePath: (path: string | null) => void;
  setFrames: React.Dispatch<React.SetStateAction<FrameInfo[]>>;
}

interface FrameStats {
  totalFrames: number;
  frameTypes: Record<string, number>;
  totalSize: number;
  avgSize: number;
  keyFrames: number;
}

const StreamDataContext = createContext<StreamDataContextType | undefined>(undefined);

export function StreamDataProvider({ children }: { children: ReactNode }) {
  const [frames, setFrames] = useState<FrameInfo[]>([]);
  const [filePath, setFilePath] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [currentFrameIndex, setCurrentFrameIndex] = useState(0);

  // Load frames from backend
  const refreshFrames = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      logger.info('refreshFrames: Calling get_frames command...');
      const result = await invoke<FrameInfo[]>('get_frames');
      logger.info('refreshFrames: Got result:', result);
      logger.info('refreshFrames: Result length:', result?.length || 0);
      setFrames(result || []);
    } catch (err) {
      const errorMsg = err as string;
      setError(errorMsg);
      logger.error('Failed to load frames:', errorMsg);
    } finally {
      setLoading(false);
    }
  }, []);

  // Clear all data (used when closing file)
  const clearData = useCallback(() => {
    setFrames([]);
    setFilePath(null);
    setCurrentFrameIndex(0);
    setError(null);
    setLoading(false);
  }, []);

  // Update file path
  const handleSetFilePath = useCallback((path: string | null) => {
    setFilePath(path);
  }, []);

  // Load frames on mount (skip in test environment to avoid act() warnings)
  useEffect(() => {
    // Only load if we don't have frames yet and we're not in test
    if (frames.length === 0 && process.env.NODE_ENV !== 'test') {
      refreshFrames();
    }
  }, []);

  // Get frame statistics
  const getFrameStats = useCallback((): FrameStats => {
    return calculateFrameStats(frames);
  }, [frames]);

  // Memoize context value to prevent unnecessary re-renders in consumers
  const contextValue = useMemo<StreamDataContextType>(() => ({
    frames,
    filePath,
    loading,
    error,
    currentFrameIndex,
    setCurrentFrameIndex,
    refreshFrames,
    clearData,
    getFrameStats,
    setFilePath: handleSetFilePath,
    setFrames,
  }), [frames, filePath, loading, error, currentFrameIndex, refreshFrames, clearData, getFrameStats, handleSetFilePath]);

  return (
    <StreamDataContext.Provider value={contextValue}>
      {children}
    </StreamDataContext.Provider>
  );
}

export function useStreamData(): StreamDataContextType {
  const context = useContext(StreamDataContext);
  if (!context) {
    throw new Error('useStreamData must be used within a StreamDataProvider');
  }
  return context;
}
