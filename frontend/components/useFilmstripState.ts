/**
 * useFilmstripState Hook
 *
 * Manages all state for the filmstrip component
 * Extracted from Filmstrip.tsx for better separation of concerns
 */

import { useState, useCallback, useMemo, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { FrameInfo } from "../types/video";
import { createLogger } from "../utils/logger";
import { THUMBNAIL_BATCH_SIZE } from "../constants/ui";

const logger = createLogger("useFilmstripState");

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
  const [loadingThumbnails, setLoadingThumbnails] = useState<Set<number>>(
    new Set(),
  );
  const [expandedFrameIndex, setExpandedFrameIndex] = useState<number | null>(
    null,
  );
  const [hoveredFrame, setHoveredFrame] = useState<FrameInfo | null>(null);
  const [mousePosition, setMousePosition] = useState<{
    x: number;
    y: number;
    placement: "left" | "right";
  } | null>(null);

  const thumbnailsRef = useRef(thumbnails);
  const loadingThumbnailsRef = useRef(loadingThumbnails);

  useEffect(() => {
    thumbnailsRef.current = thumbnails;
  }, [thumbnails]);

  useEffect(() => {
    loadingThumbnailsRef.current = loadingThumbnails;
  }, [loadingThumbnails]);

  const loadThumbnails = useCallback(async (indices: number[]) => {
    if (indices.length === 0) return;

    const indicesToLoad = indices.filter(
      (i) =>
        !thumbnailsRef.current.has(i) && !loadingThumbnailsRef.current.has(i),
    );
    if (indicesToLoad.length === 0) return;

    setLoadingThumbnails((prev) => {
      const newSet = new Set(prev);
      indicesToLoad.forEach((i) => newSet.add(i));
      return newSet;
    });

    try {
      const results = await invoke<ThumbnailResult[]>("get_thumbnails", {
        frameIndices: indicesToLoad,
      });

      setThumbnails((prev) => {
        const newMap = new Map(prev);
        results.forEach((result: ThumbnailResult) => {
          if (result.success && result.thumbnail_data) {
            // Backend already returns data URLs (e.g., "data:image/png;base64,...")
            newMap.set(result.frame_index, result.thumbnail_data);
          }
        });
        return newMap;
      });
    } catch (err) {
      logger.error("Failed to load thumbnails:", err);
    } finally {
      setLoadingThumbnails((prev) => {
        const newSet = new Set(prev);
        indicesToLoad.forEach((i) => newSet.delete(i));
        return newSet;
      });
    }
  }, []);

  // Load initial thumbnails
  useEffect(() => {
    if (displayView !== "thumbnails" || frames.length === 0) return;

    // Optimize: combine slice and filter into single pass
    const batchSize = Math.min(THUMBNAIL_BATCH_SIZE, frames.length);
    const indicesToLoad: number[] = [];
    for (let i = 0; i < batchSize; i++) {
      const frameIndex = frames[i].frame_index;
      if (!thumbnails.has(frameIndex)) {
        indicesToLoad.push(frameIndex);
      }
    }

    if (indicesToLoad.length === 0) return;

    loadThumbnails(indicesToLoad);
  }, [displayView, frames, thumbnails, loadThumbnails]);

  const handleToggleExpansion = useCallback(
    (frameIndex: number, e: React.MouseEvent) => {
      e.stopPropagation();
      setExpandedFrameIndex((prev) =>
        prev === frameIndex ? null : frameIndex,
      );
    },
    [],
  );

  const handleHoverFrame = useCallback(
    (frame: FrameInfo | null, x: number, y: number) => {
      setHoveredFrame(frame);
      if (frame) {
        const placement = x > window.innerWidth / 2 ? "left" : "right";
        setMousePosition({ x, y, placement });
      } else {
        setMousePosition(null);
      }
    },
    [],
  );

  // Optimize: avoid intermediate array from map()
  const maxSize = useMemo(() => {
    if (frames.length === 0) return 1;
    let max = frames[0].size;
    for (let i = 1; i < frames.length; i++) {
      if (frames[i].size > max) {
        max = frames[i].size;
      }
    }
    return max > 0 ? max : 1;
  }, [frames]);

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
