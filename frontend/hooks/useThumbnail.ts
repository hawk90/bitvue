/**
 * useThumbnail Hook
 *
 * Custom hook for loading and caching frame thumbnails
 * Handles thumbnail loading state, caching, and batch loading
 *
 * Usage:
 * ```tsx
 * const { thumbnails, loading, loadThumbnails, clearCache } = useThumbnail();
 *
 * // Load thumbnails for specific frames
 * loadThumbnails([0, 1, 2, 3, 4]);
 *
 * // Check if thumbnail is loaded
 * const thumbnail = thumbnails.get(frameIndex);
 *
 * // Check if thumbnail is loading
 * const isLoading = loading.has(frameIndex);
 * ```
 */

import { useState, useCallback, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { createLogger } from "../utils/logger";
import { processThumbnailResults } from "../utils/thumbnailUtils";

const logger = createLogger("useThumbnail");

/**
 * Cleanup function for pending async operations
 */
type CleanupFunction = () => void;

/**
 * Result from the get_thumbnails Tauri command
 */
export interface ThumbnailResult {
  frame_index: number;
  thumbnail_data: string;
  width: number;
  height: number;
  success: boolean;
  error?: string;
}

/**
 * Thumbnail hook state and methods
 */
export interface UseThumbnailResult {
  /** Map of frame_index to data URL */
  thumbnails: Map<number, string>;
  /** Set of frame indices currently being loaded */
  loading: Set<number>;
  /** Set of frame indices that failed to load */
  errors: Set<number>;
  /**
   * Load thumbnails for the specified frame indices
   * Only loads thumbnails that aren't already cached or currently loading
   */
  loadThumbnails: (indices: number[]) => Promise<void>;
  /** Clear all cached thumbnails */
  clearCache: () => void;
  /** Check if a thumbnail is cached */
  has: (frameIndex: number) => boolean;
  /** Get a thumbnail data URL */
  get: (frameIndex: number) => string | undefined;
  /** Check if a thumbnail failed to load */
  hasError: (frameIndex: number) => boolean;
}

/**
 * Thumbnail hook for loading and caching frame thumbnails
 */
export function useThumbnail(): UseThumbnailResult {
  const [thumbnails, setThumbnails] = useState<Map<number, string>>(new Map());
  const [loading, setLoading] = useState<Set<number>>(new Set());
  const [errors, setErrors] = useState<Set<number>>(new Set());

  // Track if component is mounted to prevent state updates after unmount
  const mountedRef = useRef(true);

  // Use refs to avoid stale closures in callbacks
  const thumbnailsRef = useRef(thumbnails);
  const loadingRef = useRef(loading);
  const errorsRef = useRef(errors);

  useEffect(() => {
    thumbnailsRef.current = thumbnails;
  }, [thumbnails]);

  useEffect(() => {
    loadingRef.current = loading;
  }, [loading]);

  useEffect(() => {
    errorsRef.current = errors;
  }, [errors]);

  // Set mounted to false on unmount
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  /**
   * Load thumbnails for the specified frame indices
   */
  const loadThumbnails = useCallback(
    async (indices: number[]): Promise<void> => {
      if (indices.length === 0) return;

      // Filter out indices that are already cached or currently loading
      const indicesToLoad = indices.filter(
        (i) => !thumbnailsRef.current.has(i) && !loadingRef.current.has(i),
      );

      if (indicesToLoad.length === 0) return;

      // Mark indices as loading and clear previous errors for these indices
      setLoading((prev) => {
        const newSet = new Set(prev);
        indicesToLoad.forEach((i) => newSet.add(i));
        return newSet;
      });

      setErrors((prev) => {
        const newSet = new Set(prev);
        indicesToLoad.forEach((i) => newSet.delete(i));
        return newSet;
      });

      try {
        const results = await invoke<ThumbnailResult[]>("get_thumbnails", {
          frameIndices: indicesToLoad,
        });

        // Only update state if component is still mounted
        if (mountedRef.current) {
          const failedIndices: number[] = [];

          // Process thumbnail results using shared utility
          const processed = processThumbnailResults(results);

          setThumbnails((prev) => {
            const newMap = new Map(prev);
            processed.forEach((dataUrl, frameIndex) => {
              newMap.set(frameIndex, dataUrl);
            });

            // Track failed thumbnails
            results.forEach((result) => {
              if (!result.success) {
                failedIndices.push(result.frame_index);
                logger.warn(
                  `Failed to load thumbnail for frame ${result.frame_index}: ${result.error || "Unknown error"}`,
                );
              }
            });

            return newMap;
          });

          // Update error state for failed thumbnails
          if (failedIndices.length > 0) {
            setErrors((prev) => {
              const newSet = new Set(prev);
              failedIndices.forEach((i) => newSet.add(i));
              return newSet;
            });
          }
        }
      } catch (err) {
        // Log the error and mark all requested indices as failed
        logger.error("Failed to load thumbnails batch:", err);

        if (mountedRef.current) {
          setErrors((prev) => {
            const newSet = new Set(prev);
            indicesToLoad.forEach((i) => newSet.add(i));
            return newSet;
          });
        }
      } finally {
        // Only update state if component is still mounted
        if (mountedRef.current) {
          // Remove indices from loading state
          setLoading((prev) => {
            const newSet = new Set(prev);
            indicesToLoad.forEach((i) => newSet.delete(i));
            return newSet;
          });
        }
      }
    },
    [],
  );

  /**
   * Clear all cached thumbnails
   */
  const clearCache = useCallback(() => {
    setThumbnails(new Map());
    setErrors(new Set());
  }, []);

  /**
   * Check if a thumbnail is cached
   */
  const has = useCallback((frameIndex: number): boolean => {
    return thumbnailsRef.current.has(frameIndex);
  }, []);

  /**
   * Get a thumbnail data URL
   */
  const get = useCallback((frameIndex: number): string | undefined => {
    return thumbnailsRef.current.get(frameIndex);
  }, []);

  /**
   * Check if a thumbnail failed to load
   */
  const hasError = useCallback((frameIndex: number): boolean => {
    return errorsRef.current.has(frameIndex);
  }, []);

  return {
    thumbnails,
    loading,
    errors,
    loadThumbnails,
    clearCache,
    has,
    get,
    hasError,
  };
}

export default useThumbnail;
