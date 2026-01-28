/**
 * Thumbnail Context
 *
 * Manages thumbnail loading state and caching
 * Separated from StreamDataContext for better separation of concerns
 */

import { createContext, useContext, useState, useCallback, ReactNode, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { createLogger } from '../utils/logger';
import { processThumbnailResults } from '../utils/thumbnailUtils';
import { TIMING } from '../constants/ui';

const logger = createLogger('ThumbnailContext');

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

const ThumbnailContext = createContext<ThumbnailContextType | undefined>(undefined);

export function ThumbnailProvider({ children }: { children: ReactNode }) {
  const [thumbnails, setThumbnails] = useState<Map<number, string>>(new Map());
  const [loading, setLoading] = useState<Set<number>>(new Set());

  // Use refs to avoid stale closures in callbacks
  const thumbnailsRef = useRef(thumbnails);
  const loadingRef = useRef(loading);

  // Refs for debouncing thumbnail loads
  const debounceTimeoutRef = useRef<number | null>(null);
  const pendingLoadRef = useRef<Set<number>>(new Set());

  // Keep refs in sync with state
  const updateThumbnailsRef = useCallback((newThumbnails: Map<number, string>) => {
    thumbnailsRef.current = newThumbnails;
  }, []);

  const updateLoadingRef = useCallback((newLoading: Set<number>) => {
    loadingRef.current = newLoading;
  }, []);

  // Internal function to perform the actual thumbnail loading
  const performLoad = useCallback(async (indicesToLoad: number[]): Promise<void> => {
    if (indicesToLoad.length === 0) {
      return;
    }

    // Mark as loading
    const newLoading = new Set(loadingRef.current);
    indicesToLoad.forEach((i) => newLoading.add(i));
    setLoading(newLoading);
    updateLoadingRef(newLoading);

    try {
      const results = await invoke<ThumbnailResult[]>('get_thumbnails', {
        frameIndices: indicesToLoad
      });

      // Process results using shared utility
      const processed = processThumbnailResults(results);

      // Update thumbnails map
      const newThumbnails = new Map(thumbnailsRef.current);
      processed.forEach((dataUrl, frameIndex) => {
        newThumbnails.set(frameIndex, dataUrl);
      });

      setThumbnails(newThumbnails);
      updateThumbnailsRef(newThumbnails);
    } catch (error) {
      logger.error('Failed to load thumbnails:', error);
    } finally {
      // Remove from loading state
      const newLoading = new Set(loadingRef.current);
      indicesToLoad.forEach((i) => newLoading.delete(i));
      setLoading(newLoading);
      updateLoadingRef(newLoading);
    }
  }, [updateLoadingRef, updateThumbnailsRef]);

  // Load thumbnails for the given frame indices (debounced)
  const loadThumbnails = useCallback((indices: number[]): void => {
    // Filter out already loaded or currently loading thumbnails
    const indicesToLoad = indices.filter(
      (i) => !thumbnailsRef.current.has(i) && !loadingRef.current.has(i)
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
  }, [performLoad]);

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
    // Clear pending load set
    pendingLoadRef.current.clear();
    // Clear thumbnails map
    setThumbnails(new Map());
    updateThumbnailsRef(new Map());
  }, [updateThumbnailsRef]);

  // Preload thumbnails for a range of frames
  const preloadRange = useCallback((startIndex: number, count: number): void => {
    const indices = Array.from({ length: count }, (_, i) => startIndex + i);
    loadThumbnails(indices);
  }, [loadThumbnails]);

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
    throw new Error('useThumbnails must be used within a ThumbnailProvider');
  }
  return context;
}
