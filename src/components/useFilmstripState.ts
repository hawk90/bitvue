/**
 * useFilmstripState Hook
 *
 * Manages all state for the filmstrip component
 * Extracted from Filmstrip.tsx for better separation of concerns
 */

import { useState, useCallback, useMemo, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { FrameInfo } from '../types/video';
import { createLogger } from '../utils/logger';
import { THUMBNAIL_BATCH_SIZE } from '../constants/ui';

const logger = createLogger('useFilmstripState');

/**
 * Result from the get_thumbnails Tauri command
 */
interface ThumbnailResult {
  frame_index: number;
  thumbnail_data: string;
  width: number;
  height: number;
  success: boolean;
  error?: string;
}

interface UseFilmstripStateProps {
  frames: FrameInfo[];
  displayView: string;
}

export function useFilmstripState({
  frames,
  displayView,
}: UseFilmstripStateProps) {
  const [thumbnails, setThumbnails] = useState<Map<number, string>>(new Map());
  const [loadingThumbnails, setLoadingThumbnails] = useState<Set<number>>(new Set());
  const [expandedFrameIndex, setExpandedFrameIndex] = useState<number | null>(null);
  const [hoveredFrame, setHoveredFrame] = useState<FrameInfo | null>(null);
  const [mousePosition, setMousePosition] = useState<{
    x: number;
    y: number;
    placement: 'left' | 'right';
  } | null>(null);

  const thumbnailsRef = useRef(thumbnails);
  const loadingThumbnailsRef = useRef(loadingThumbnails);

  useEffect(() => {
    thumbnailsRef.current = thumbnails;
  }, [thumbnails]);

  useEffect(() => {
    loadingThumbnailsRef.current = loadingThumbnails;
  }, [loadingThumbnails]);

  const getDataUrl = useCallback((base64Data: string): string => {
    if (base64Data.startsWith('iVBORw0KGgo')) {
      return `data:image/png;base64,${base64Data}`;
    }
    return `data:image/svg+xml;base64,${base64Data}`;
  }, []);

  const loadThumbnails = useCallback(async (indices: number[]) => {
    if (indices.length === 0) return;

    const indicesToLoad = indices.filter(
      (i) => !thumbnailsRef.current.has(i) && !loadingThumbnailsRef.current.has(i)
    );
    if (indicesToLoad.length === 0) return;

    setLoadingThumbnails((prev) => {
      const newSet = new Set(prev);
      indicesToLoad.forEach((i) => newSet.add(i));
      return newSet;
    });

    try {
      const results = await invoke<ThumbnailResult[]>('get_thumbnails', { frameIndices: indicesToLoad });

      setThumbnails((prev) => {
        const newMap = new Map(prev);
        results.forEach((result: ThumbnailResult) => {
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
      setLoadingThumbnails((prev) => {
        const newSet = new Set(prev);
        indicesToLoad.forEach((i) => newSet.delete(i));
        return newSet;
      });
    }
  }, [getDataUrl]);

  // Load initial thumbnails
  useEffect(() => {
    if (displayView !== 'thumbnails' || frames.length === 0) return;

    const initialFrames = frames.slice(0, Math.min(THUMBNAIL_BATCH_SIZE, frames.length));
    const framesWithoutThumbnails = initialFrames.filter((f) => !thumbnails.has(f.frame_index));

    if (framesWithoutThumbnails.length === 0) return;

    loadThumbnails(framesWithoutThumbnails.map((f) => f.frame_index));
  }, [displayView, frames, thumbnails, loadThumbnails]);

  const handleToggleExpansion = useCallback((frameIndex: number, e: React.MouseEvent) => {
    e.stopPropagation();
    setExpandedFrameIndex((prev) => (prev === frameIndex ? null : frameIndex));
  }, []);

  const handleHoverFrame = useCallback((frame: FrameInfo | null, x: number, y: number) => {
    setHoveredFrame(frame);
    if (frame) {
      const placement = x > window.innerWidth / 2 ? 'left' : 'right';
      setMousePosition({ x, y, placement });
    } else {
      setMousePosition(null);
    }
  }, []);

  const maxSize = useMemo(() => Math.max(...frames.map((f) => f.size), 1), [frames]);

  return {
    thumbnails,
    loadingThumbnails,
    expandedFrameIndex,
    hoveredFrame,
    mousePosition,
    maxSize,
    loadThumbnails,
    handleToggleExpansion,
    handleHoverFrame,
    setExpandedFrameIndex,
  };
}
