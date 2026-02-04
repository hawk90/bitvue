/**
 * Frame Data Context
 *
 * Provides frame data and statistics (stable, changes only on file load)
 * Separated from currentFrameIndex to prevent re-renders during navigation
 * Uses Web Worker for stats calculation to prevent UI blocking
 */

import { createContext, useContext, useState, useCallback, useEffect, useRef, ReactNode, useMemo } from 'react';
import type { FrameInfo } from '../types/video';

interface FrameStats {
  totalFrames: number;
  frameTypes: Record<string, number>;
  totalSize: number;
  avgSize: number;
  keyFrames: number;
}

interface FrameDataContextType {
  frames: FrameInfo[];
  setFrames: React.Dispatch<React.SetStateAction<FrameInfo[]>>;
  getFrameStats: () => FrameStats;
}

const FrameDataContext = createContext<FrameDataContextType | undefined>(undefined);

// Threshold for using worker (100 frames = ~10KB data)
const WORKER_THRESHOLD = 100;

export function FrameDataProvider({ children }: { children: ReactNode }) {
  const [frames, setFrames] = useState<FrameInfo[]>([]);
  const [frameStats, setFrameStats] = useState<FrameStats>({
    totalFrames: 0,
    frameTypes: {},
    totalSize: 0,
    avgSize: 0,
    keyFrames: 0,
  });

  const workerRef = useRef<Worker | null>(null);

  // Cleanup worker on unmount
  useEffect(() => {
    return () => {
      if (workerRef.current) {
        workerRef.current.terminate();
        workerRef.current = null;
      }
    };
  }, []);

  // Calculate frame statistics (synchronous for small arrays, worker for large)
  useEffect(() => {
    // For small arrays, calculate directly on main thread
    if (frames.length < WORKER_THRESHOLD) {
      const stats = calculateFrameStatsSync(frames);
      setFrameStats(stats);
      return;
    }

    // For large arrays, use Web Worker
    const calculateWithWorker = async () => {
      // Terminate existing worker if any
      if (workerRef.current) {
        workerRef.current.terminate();
        workerRef.current = null;
      }

      try {
        // Create new worker
        const worker = new Worker(
          new URL('../workers/frameStatsWorker.ts', import.meta.url),
          { type: 'module' }
        );

        workerRef.current = worker;

        // Set up message handler
        worker.onmessage = (event: MessageEvent<FrameStats>) => {
          setFrameStats(event.data);
          // Don't terminate worker immediately - it might be reused
        };

        worker.onerror = (error) => {
          console.error('Frame stats worker error:', error);
          // Fallback to sync calculation on error
          const stats = calculateFrameStatsSync(frames);
          setFrameStats(stats);
          worker.terminate();
          workerRef.current = null;
        };

        // Send frames to worker
        worker.postMessage(frames);
      } catch (error) {
        console.error('Failed to create frame stats worker:', error);
        // Fallback to sync calculation
        const stats = calculateFrameStatsSync(frames);
        setFrameStats(stats);
      }
    };

    calculateWithWorker();
  }, [frames]);

  // Get frame statistics (returns current value)
  const getFrameStats = useCallback((): FrameStats => {
    return frameStats;
  }, [frameStats]);

  const contextValue = useMemo<FrameDataContextType>(() => ({
    frames,
    setFrames,
    getFrameStats,
  }), [frames, getFrameStats]);

  return (
    <FrameDataContext.Provider value={contextValue}>
      {children}
    </FrameDataContext.Provider>
  );
}

/**
 * Synchronous fallback for calculating frame statistics
 * Used for small arrays or when worker fails
 */
function calculateFrameStatsSync(frames: FrameInfo[]): FrameStats {
  const totalFrames = frames.length;
  let totalSize = 0;
  let keyFrames = 0;
  const frameTypes: Record<string, number> = {};

  for (const frame of frames) {
    totalSize += frame.size;

    if (frame.key_frame) {
      keyFrames++;
    }

    frameTypes[frame.frame_type] = (frameTypes[frame.frame_type] || 0) + 1;
  }

  const avgSize = totalFrames > 0 ? totalSize / totalFrames : 0;

  return {
    totalFrames,
    frameTypes,
    totalSize,
    avgSize,
    keyFrames,
  };
}

export function useFrameData(): FrameDataContextType {
  const context = useContext(FrameDataContext);
  if (!context) {
    throw new Error('useFrameData must be used within a FrameDataProvider');
  }
  return context;
}

// Export type for use in other components
export type { FrameStats, FrameDataContextType };
