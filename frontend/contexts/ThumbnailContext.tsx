/**
 * Thumbnail Context
 *
 * Manages thumbnail loading state and caching
 * Separated from StreamDataContext for better separation of concerns
 */

import {
  createContext,
  useContext,
  useState,
  useCallback,
  ReactNode,
  useRef,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { createLogger } from "../utils/logger";
import { processThumbnailResults } from "../utils/thumbnailUtils";
import { TIMING } from "../constants/ui";

const logger = createLogger("ThumbnailContext");

/** Maximum number of decoded thumbnails to keep in memory at once.
 *  At ~20 KB each (160×90 JPEG), 256 entries ≈ 5 MB.
 *  When the limit is exceeded the oldest-loaded entries are evicted (FIFO).
 */
const MAX_CACHED_THUMBNAILS = 256;

export interface ThumbnailResult {
  frame_index: number;
  thumbnail_data: string;
  width: number;
  height: number;
  success: boolean;
  error?: string;
}

interface ThumbnailContextType {
  thumbnails: Map<number, string>;
  loading: Set<number>;
  loadThumbnails: (indices: number[]) => void;
  getThumbnail: (frameIndex: number) => string | undefined;
  isLoading: (frameIndex: number) => boolean;
  clearCache: () => void;
  preloadRange: (startIndex: number, count: number) => void;
}

const ThumbnailContext = createContext<ThumbnailContextType | undefined>(
  undefined,
);

export function ThumbnailProvider({ children }: { children: ReactNode }) {
  const [thumbnails, setThumbnails] = useState<Map<number, string>>(new Map());
  const [loading, setLoading] = useState<Set<number>>(new Set());

  // Use refs to avoid stale closures in callbacks
  const thumbnailsRef = useRef(thumbnails);
  const loadingRef = useRef(loading);

  // FIFO eviction queue — tracks insertion order for LRU eviction.
  // We use a plain array here because:
  //  - pushes happen at most once per thumbnail batch
  //  - evictions walk from the front (oldest entry)
  const evictionQueueRef = useRef<number[]>([]);

  // Refs for debouncing thumbnail loads
  const debounceTimeoutRef = useRef<number | null>(null);
  const pendingLoadRef = useRef<Set<number>>(new Set());

  // Keep refs in sync with state
  const updateThumbnailsRef = useCallback(
    (newThumbnails: Map<number, string>) => {
      thumbnailsRef.current = newThumbnails;
    },
    [],
  );

  const updateLoadingRef = useCallback((newLoading: Set<number>) => {
    loadingRef.current = newLoading;
  }, []);

  // Internal function to perform the actual thumbnail loading
  const performLoad = useCallback(
    async (indicesToLoad: number[]): Promise<void> => {
      if (indicesToLoad.length === 0) {
        return;
      }

      // Mark as loading
      const newLoading = new Set(loadingRef.current);
      indicesToLoad.forEach((i) => newLoading.add(i));
      setLoading(newLoading);
      updateLoadingRef(newLoading);

      try {
        const results = await invoke<ThumbnailResult[]>("get_thumbnails", {
          frameIndices: indicesToLoad,
        });

        // Process results using shared utility
        const processed = processThumbnailResults(results);

        // Update thumbnails map with LRU eviction
        const newThumbnails = new Map(thumbnailsRef.current);
        processed.forEach((dataUrl, frameIndex) => {
          // Only enqueue new entries — don't duplicate entries that are
          // being refreshed (their eviction-order position stays unchanged).
          if (!newThumbnails.has(frameIndex)) {
            evictionQueueRef.current.push(frameIndex);
          }
          newThumbnails.set(frameIndex, dataUrl);
        });

        // Evict oldest entries when the cache exceeds the size cap.
        while (newThumbnails.size > MAX_CACHED_THUMBNAILS) {
          const oldest = evictionQueueRef.current.shift();
          if (oldest !== undefined) {
            newThumbnails.delete(oldest);
          } else {
            break; // eviction queue is empty (shouldn't happen, but guard anyway)
          }
        }

        setThumbnails(newThumbnails);
        updateThumbnailsRef(newThumbnails);
      } catch (error) {
        logger.error("Failed to load thumbnails:", error);
      } finally {
        // Remove from loading state
        const newLoading = new Set(loadingRef.current);
        indicesToLoad.forEach((i) => newLoading.delete(i));
        setLoading(newLoading);
        updateLoadingRef(newLoading);
      }
    },
    [updateLoadingRef, updateThumbnailsRef],
  );

  // Load thumbnails for the given frame indices (debounced)
  const loadThumbnails = useCallback(
    (indices: number[]): void => {
      // Filter out already loaded or currently loading thumbnails
      const indicesToLoad = indices.filter(
        (i) => !thumbnailsRef.current.has(i) && !loadingRef.current.has(i),
      );

      if (indicesToLoad.length === 0) {
        return;
      }

      // Add to pending load set
      indicesToLoad.forEach((i) => pendingLoadRef.current.add(i));

      // Clear any pending load timeout
      if (debounceTimeoutRef.current !== null) {
        clearTimeout(debounceTimeoutRef.current);
      }

      // Set new timeout for debounced load
      debounceTimeoutRef.current = window.setTimeout(() => {
        // Get all pending indices and clear the set
        const pendingIndices = Array.from(pendingLoadRef.current);
        pendingLoadRef.current.clear();

        // Perform the actual load
        performLoad(pendingIndices);
      }, TIMING.THUMBNAIL_LOAD_DELAY);
    },
    [performLoad],
  );

  // Get thumbnail for a specific frame
  const getThumbnail = useCallback((frameIndex: number): string | undefined => {
    return thumbnailsRef.current.get(frameIndex);
  }, []);

  // Check if a thumbnail is currently loading
  const isLoading = useCallback((frameIndex: number): boolean => {
    return loadingRef.current.has(frameIndex);
  }, []);

  // Clear all cached thumbnails
  const clearCache = useCallback(() => {
    // Clear any pending load timeout
    if (debounceTimeoutRef.current !== null) {
      clearTimeout(debounceTimeoutRef.current);
      debounceTimeoutRef.current = null;
    }
    // Clear pending load set and eviction queue
    pendingLoadRef.current.clear();
    evictionQueueRef.current = [];
    // Clear thumbnails map
    setThumbnails(new Map());
    updateThumbnailsRef(new Map());
  }, [updateThumbnailsRef]);

  // Preload thumbnails for a range of frames
  const preloadRange = useCallback(
    (startIndex: number, count: number): void => {
      const indices = Array.from({ length: count }, (_, i) => startIndex + i);
      loadThumbnails(indices);
    },
    [loadThumbnails],
  );

  return (
    <ThumbnailContext.Provider
      value={{
        thumbnails,
        loading,
        loadThumbnails,
        getThumbnail,
        isLoading,
        clearCache,
        preloadRange,
      }}
    >
      {children}
    </ThumbnailContext.Provider>
  );
}

export function useThumbnails(): ThumbnailContextType {
  const context = useContext(ThumbnailContext);
  if (!context) {
    throw new Error("useThumbnails must be used within a ThumbnailProvider");
  }
  return context;
}
