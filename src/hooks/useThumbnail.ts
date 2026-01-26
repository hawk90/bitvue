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

import { useState, useCallback, useRef, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { createLogger } from '../utils/logger';

const logger = createLogger('useThumbnail');

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
}

/**
 * Convert base64 thumbnail data to data URL
 * Detects PNG vs SVG format
 */
function getDataUrl(base64Data: string): string {
  if (base64Data.startsWith('iVBORw0KGgo') || base64Data.startsWith('<svg')) {
    // PNG data (starts with magic bytes) or SVG
    return `data:image/${base64Data.startsWith('<svg') ? 'svg+xml' : 'png'};base64,${base64Data}`;
  }
  return `data:image/png;base64,${base64Data}`;
}

/**
 * Thumbnail hook for loading and caching frame thumbnails
 */
export function useThumbnail(): UseThumbnailResult {
  const [thumbnails, setThumbnails] = useState<Map<number, string>>(new Map());
  const [loading, setLoading] = useState<Set<number>>(new Set());

  // Use refs to avoid stale closures in callbacks
  const thumbnailsRef = useRef(thumbnails);
  const loadingRef = useRef(loading);

  useEffect(() => {
    thumbnailsRef.current = thumbnails;
  }, [thumbnails]);

  useEffect(() => {
    loadingRef.current = loading;
  }, [loading]);

  /**
   * Load thumbnails for the specified frame indices
   */
  const loadThumbnails = useCallback(async (indices: number[]): Promise<void> => {
    if (indices.length === 0) return;

    // Filter out indices that are already cached or currently loading
    const indicesToLoad = indices.filter(
      (i) => !thumbnailsRef.current.has(i) && !loadingRef.current.has(i)
    );

    if (indicesToLoad.length === 0) return;

    // Mark indices as loading
    setLoading((prev) => {
      const newSet = new Set(prev);
      indicesToLoad.forEach((i) => newSet.add(i));
      return newSet;
    });

    try {
      const results = await invoke<ThumbnailResult[]>('get_thumbnails', {
        frameIndices: indicesToLoad
      });

      setThumbnails((prev) => {
        const newMap = new Map(prev);
        results.forEach((result) => {
          if (result.success && result.thumbnail_data) {
            const dataUrl = getDataUrl(result.thumbnail_data);
            newMap.set(result.frame_index, dataUrl);
          }
        });
        return newMap;
      });
    } catch (err) {
      logger.error('Failed to load thumbnails:', err);
    } finally {
      // Remove indices from loading state
      setLoading((prev) => {
        const newSet = new Set(prev);
        indicesToLoad.forEach((i) => newSet.delete(i));
        return newSet;
      });
    }
  }, []);

  /**
   * Clear all cached thumbnails
   */
  const clearCache = useCallback(() => {
    setThumbnails(new Map());
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

  return {
    thumbnails,
    loading,
    loadThumbnails,
    clearCache,
    has,
    get,
  };
}

export default useThumbnail;
