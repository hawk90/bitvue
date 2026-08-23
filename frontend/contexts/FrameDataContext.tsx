/**
 * Frame Data Context
 *
 * Provides frame data and statistics (stable, changes only on file load)
 * Separated from currentFrameIndex to prevent re-renders during navigation
 * Uses Web Worker for stats calculation to prevent UI blocking
 */

import {
  createContext,
  useContext,
  useState,
  useCallback,
  useEffect,
  useRef,
  ReactNode,
  useMemo,
} from "react";
import type { FrameInfo } from "../types/video";

interface FrameStats {
  totalFrames: number;
  frameTypes: Record<string, number>;
  totalSize: number;
  avgSize: number;
  keyFrames: number;
}

/** Container-level stream metadata (`bitvue_engine::ContainerModel`, via `getStreamInfo`) --
 *  distinct from per-frame `FrameInfo`: this is the coded/display resolution and codec name for
 *  the whole stream, not any one frame's data. `null` until `indexStream` has actually run. */
interface StreamInfo {
  width: number;
  height: number;
  codec: string;
  bitDepth: number | null;
  /** Stream-wide PTS quality (EDGE-03) -- `null` until `applyDisplayOrder` resolves (non-fatal,
   *  same "falls back to N/A" convention as the rest of that function), not fabricated. */
  ptsQuality: "Ok" | "Warn" | "Bad" | null;
}

interface FrameDataContextType {
  frames: FrameInfo[];
  setFrames: React.Dispatch<React.SetStateAction<FrameInfo[]>>;
  getFrameStats: () => FrameStats;
  streamInfo: StreamInfo | null;
  setStreamInfo: React.Dispatch<React.SetStateAction<StreamInfo | null>>;
}

const FrameDataContext = createContext<FrameDataContextType | undefined>(
  undefined,
);

// Threshold for using worker (100 frames = ~10KB data)
const WORKER_THRESHOLD = 100;

export function FrameDataProvider({ children }: { children: ReactNode }) {
  const [frames, setFrames] = useState<FrameInfo[]>([]);
  const [streamInfo, setStreamInfo] = useState<StreamInfo | null>(null);
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

    // For large arrays, use Web Worker.
    // Track mount state so we can discard results and terminate the worker if
    // the component unmounts before the async initialization completes.
    let mounted = true;

    const calculateWithWorker = async () => {
      // Terminate existing worker if any
      if (workerRef.current) {
        workerRef.current.terminate();
        workerRef.current = null;
      }

      try {
        // Create new worker
        const worker = new Worker(
          new URL("../workers/frameStatsWorker.ts", import.meta.url),
          { type: "module" },
        );

        // If the component unmounted while we were constructing the worker,
        // terminate it immediately to prevent a memory leak.
        if (!mounted) {
          worker.terminate();
          return;
        }

        workerRef.current = worker;

        // Set up message handler
        worker.onmessage = (event: MessageEvent<FrameStats>) => {
          if (mounted) {
            setFrameStats(event.data);
          }
          // Don't terminate worker immediately - it might be reused
        };

        worker.onerror = (error) => {
          console.error("Frame stats worker error:", error);
          if (mounted) {
            // Fallback to sync calculation on error
            const stats = calculateFrameStatsSync(frames);
            setFrameStats(stats);
          }
          worker.terminate();
          if (workerRef.current === worker) {
            workerRef.current = null;
          }
        };

        // Send frames to worker
        worker.postMessage(frames);
      } catch (error) {
        console.error("Failed to create frame stats worker:", error);
        if (mounted) {
          // Fallback to sync calculation
          const stats = calculateFrameStatsSync(frames);
          setFrameStats(stats);
        }
      }
    };

    calculateWithWorker();

    return () => {
      mounted = false;
      if (workerRef.current) {
        workerRef.current.terminate();
        workerRef.current = null;
      }
    };
  }, [frames]);

  // Get frame statistics (returns current value)
  const getFrameStats = useCallback((): FrameStats => {
    return frameStats;
  }, [frameStats]);

  const contextValue = useMemo<FrameDataContextType>(
    () => ({
      frames,
      setFrames,
      getFrameStats,
      streamInfo,
      setStreamInfo,
    }),
    [frames, getFrameStats, streamInfo],
  );

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
    throw new Error("useFrameData must be used within a FrameDataProvider");
  }
  return context;
}

// Export type for use in other components
export type { FrameStats, FrameDataContextType };
