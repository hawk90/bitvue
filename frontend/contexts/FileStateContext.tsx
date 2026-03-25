/**
 * File State Context
 *
 * Manages file loading state and operations
 * Separated from frame navigation to prevent unnecessary re-renders
 * Supports chunked frame loading for faster initial load
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
import { invoke } from "@tauri-apps/api/core";
import type { FrameInfo } from "../types/video";
import { createLogger } from "../utils/logger";
import { useFrameData } from "./FrameDataContext";

const logger = createLogger("FileStateContext");

interface ChunkedFramesResponse {
  frames: FrameInfo[];
  total_frames: number;
  has_more: boolean;
  offset: number;
}

interface FileStateContextType {
  filePath: string | null;
  loading: boolean;
  error: string | null;
  setFilePath: (path: string | null) => void;
  refreshFrames: () => Promise<FrameInfo[]>;
  loadMoreFrames: () => Promise<FrameInfo[]>;
  hasMoreFrames: boolean;
  totalFrames: number;
  clearData: () => void;
}

const FileStateContext = createContext<FileStateContextType | undefined>(
  undefined,
);

// Chunk size for progressive loading
const CHUNK_SIZE = 100;
// Threshold for using chunked loading (200+ frames)
const CHUNKED_LOADING_THRESHOLD = 200;

export function FileStateProvider({ children }: { children: ReactNode }) {
  const { setFrames } = useFrameData();
  const [filePath, setFilePath] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hasMoreFrames, setHasMoreFrames] = useState(false);
  const [totalFrames, setTotalFrames] = useState(0);

  const currentOffsetRef = useRef(0);
  const isLoadingMoreRef = useRef(false);

  // Load frames from backend (with chunking support)
  const refreshFrames = useCallback(async () => {
    setLoading(true);
    setError(null);
    currentOffsetRef.current = 0;
    setHasMoreFrames(false);
    setTotalFrames(0);
    setFrames([]);

    try {
      logger.info("refreshFrames: Calling get_frames_chunk command...");
      const startTime = performance.now();
      const firstChunk = await invoke<ChunkedFramesResponse>(
        "get_frames_chunk",
        {
          offset: 0,
          limit: CHUNK_SIZE,
        },
      );

      let allFrames = firstChunk.frames || [];
      currentOffsetRef.current = firstChunk.offset + allFrames.length;
      setHasMoreFrames(firstChunk.has_more);
      setTotalFrames(firstChunk.total_frames);
      setFrames(allFrames);

      while (currentOffsetRef.current < firstChunk.total_frames) {
        const nextChunk = await invoke<ChunkedFramesResponse>(
          "get_frames_chunk",
          {
            offset: currentOffsetRef.current,
            limit: CHUNK_SIZE,
          },
        );
        if (nextChunk.frames.length === 0) break;
        allFrames = [...allFrames, ...nextChunk.frames];
        currentOffsetRef.current = nextChunk.offset + nextChunk.frames.length;
        setHasMoreFrames(nextChunk.has_more);
        setTotalFrames(nextChunk.total_frames);
        setFrames(allFrames);
      }

      const elapsed = performance.now() - startTime;
      logger.info(
        `refreshFrames: Loaded ${allFrames.length} frames in ${elapsed.toFixed(2)}ms`,
      );

      if (allFrames.length >= CHUNKED_LOADING_THRESHOLD) {
        logger.info(
          `refreshFrames: Loaded large file progressively (${allFrames.length} frames)`,
        );
      }

      setHasMoreFrames(false);
      return allFrames;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      setError(errorMsg);
      logger.error("Failed to load frames:", errorMsg);
      setFrames([]);
      return [];
    } finally {
      setLoading(false);
    }
  }, [setFrames]);

  // Load more frames using chunked loading
  const loadMoreFrames = useCallback(async () => {
    // Prevent concurrent loading
    if (isLoadingMoreRef.current || !hasMoreFrames) {
      return [];
    }

    isLoadingMoreRef.current = true;

    try {
      logger.info(
        `loadMoreFrames: Loading chunk at offset ${currentOffsetRef.current}`,
      );

      const result = await invoke<ChunkedFramesResponse>("get_frames_chunk", {
        offset: currentOffsetRef.current,
        limit: CHUNK_SIZE,
      });

      logger.info(
        `loadMoreFrames: Got ${result.frames.length} frames, has_more: ${result.has_more}, total: ${result.total_frames}`,
      );

      // Update state for next chunk
      currentOffsetRef.current = result.offset + result.frames.length;
      setHasMoreFrames(result.has_more);
      setTotalFrames(result.total_frames);

      return result.frames || [];
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      logger.error("Failed to load frames chunk:", errorMsg);
      setError(errorMsg);
      return [];
    } finally {
      isLoadingMoreRef.current = false;
    }
  }, [hasMoreFrames]);

  // Clear all data (used when closing file)
  const clearData = useCallback(() => {
    setFilePath(null);
    setError(null);
    setLoading(false);
    setHasMoreFrames(false);
    setTotalFrames(0);
    currentOffsetRef.current = 0;
    isLoadingMoreRef.current = false;
    setFrames([]);
  }, []);

  const contextValue = useMemo<FileStateContextType>(
    () => ({
      filePath,
      loading,
      error,
      setFilePath,
      refreshFrames,
      loadMoreFrames,
      hasMoreFrames,
      totalFrames,
      clearData,
    }),
    [
      filePath,
      loading,
      error,
      refreshFrames,
      loadMoreFrames,
      hasMoreFrames,
      totalFrames,
      clearData,
    ],
  );

  return (
    <FileStateContext.Provider value={contextValue}>
      {children}
    </FileStateContext.Provider>
  );
}

export function useFileState(): FileStateContextType {
  const context = useContext(FileStateContext);
  if (!context) {
    throw new Error("useFileState must be used within a FileStateProvider");
  }
  return context;
}

export type { FileStateContextType };
