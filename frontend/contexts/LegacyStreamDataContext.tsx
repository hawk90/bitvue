/**
 * Legacy StreamData Context (backward-compatible)
 *
 * Provides the combined stream data API for backward compatibility.
 * New code should use the individual split contexts directly:
 * - useFrameData() from FrameDataContext
 * - useFileState() from FileStateContext
 * - useCurrentFrame() from CurrentFrameContext
 */

import {
  createContext,
  useContext,
  useState,
  useCallback,
  useMemo,
  type ReactNode,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import type { FrameInfo } from "../types/video";

interface StreamDataContextType {
  loading: boolean;
  frames: FrameInfo[];
  currentFrameIndex: number;
  error: string | null;
  filePath: string | null;
  setCurrentFrameIndex: (index: number) => void;
  setFrames: (frames: FrameInfo[]) => void;
  setFilePath: (path: string | null) => void;
  refreshFrames: () => Promise<void>;
  clearData: () => void;
  getFrameStats: () => {
    totalFrames: number;
    frameTypes: Record<string, number>;
    totalSize: number;
    avgSize: number;
    keyFrames: number;
  };
}

const StreamDataContext = createContext<StreamDataContextType | undefined>(
  undefined,
);

export function StreamDataProvider({ children }: { children: ReactNode }) {
  const [loading, setLoading] = useState(false);
  const [frames, setFrames] = useState<FrameInfo[]>([]);
  const [currentFrameIndex, setCurrentFrameIndex] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [filePath, setFilePath] = useState<string | null>(null);

  const refreshFrames = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<FrameInfo[]>("get_frames");
      setFrames(result || []);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      setError(errorMsg);
    } finally {
      setLoading(false);
    }
  }, []);

  const clearData = useCallback(() => {
    setFrames([]);
    setCurrentFrameIndex(0);
    setError(null);
    setFilePath(null);
    setLoading(false);
  }, []);

  const getFrameStats = useCallback(() => {
    const totalFrames = frames.length;
    let totalSize = 0;
    let keyFrames = 0;
    const frameTypes: Record<string, number> = {};

    for (const frame of frames) {
      totalSize += frame.size;
      if (frame.key_frame) keyFrames++;
      frameTypes[frame.frame_type] = (frameTypes[frame.frame_type] || 0) + 1;
    }

    const avgSize = totalFrames > 0 ? totalSize / totalFrames : 0;

    return { totalFrames, frameTypes, totalSize, avgSize, keyFrames };
  }, [frames]);

  const contextValue = useMemo<StreamDataContextType>(
    () => ({
      loading,
      frames,
      currentFrameIndex,
      error,
      filePath,
      setCurrentFrameIndex,
      setFrames,
      setFilePath,
      refreshFrames,
      clearData,
      getFrameStats,
    }),
    [
      loading,
      frames,
      currentFrameIndex,
      error,
      filePath,
      refreshFrames,
      clearData,
      getFrameStats,
    ],
  );

  return (
    <StreamDataContext.Provider value={contextValue}>
      {children}
    </StreamDataContext.Provider>
  );
}

export function useStreamData(): StreamDataContextType {
  const context = useContext(StreamDataContext);
  if (!context) {
    throw new Error("useStreamData must be used within a StreamDataProvider");
  }
  return context;
}
