/**
 * Virtualized Filmstrip Component
 *
 * Efficiently renders large frame lists using virtualization
 * Only renders visible frames plus buffer
 */

import { useRef, useState, useCallback, useEffect, useMemo, memo } from "react";
import type { FrameInfo } from "../types/video";
import { getFrameTypeColor } from "../types/video";
import { FILMSTRIP } from "../constants/ui";
import "./VirtualizedFilmstrip.css";

interface VirtualizedFilmstripProps {
  frames: FrameInfo[];
  currentFrameIndex: number;
  onFrameChange: (frameIndex: number) => void;
  itemWidth: number;
  containerWidth: number;
  overscan?: number; // Number of extra items to render outside viewport
}

export const VirtualizedFilmstrip = memo(function VirtualizedFilmstrip({
  frames,
  currentFrameIndex,
  onFrameChange,
  itemWidth,
  containerWidth,
  overscan = FILMSTRIP.VIRTUAL_DEFAULT_OVERSCAN,
}: VirtualizedFilmstripProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [scrollLeft, setScrollLeft] = useState(0);
  const [isDragging, setIsDragging] = useState(false);

  // Calculate total width
  const totalWidth = frames.length * itemWidth;

  // Calculate visible range
  const visibleRange = useMemo(() => {
    const start = Math.floor(scrollLeft / itemWidth);
    const end = Math.ceil((scrollLeft + containerWidth) / itemWidth);
    return {
      start: Math.max(0, start - overscan),
      end: Math.min(frames.length, end + overscan),
    };
  }, [scrollLeft, itemWidth, containerWidth, frames.length, overscan]);

  // Calculate offset for first visible item
  const offset = visibleRange.start * itemWidth;

  // Handle scroll with requestAnimationFrame for smooth updates
  const rafRef = useRef<number>();
  const handleScroll = useCallback((e: React.UIEvent<HTMLDivElement>) => {
    // Cancel any pending animation frame
    if (rafRef.current !== undefined) {
      cancelAnimationFrame(rafRef.current);
    }

    // Schedule state update for next animation frame
    rafRef.current = requestAnimationFrame(() => {
      setScrollLeft(e.currentTarget.scrollLeft);
      rafRef.current = undefined;
    });
  }, []);

  // Cleanup animation frame on unmount
  useEffect(() => {
    return () => {
      if (rafRef.current !== undefined) {
        cancelAnimationFrame(rafRef.current);
      }
    };
  }, []);

  // Handle click on frame
  const handleFrameClick = useCallback(
    (frameIndex: number) => {
      onFrameChange(frameIndex);
    },
    [onFrameChange],
  );

  // Track event listeners for cleanup
  const eventListenersRef = useRef<{
    handleMouseMove?: (e: MouseEvent) => void;
    handleMouseUp?: () => void;
  }>({});

  // Cleanup event listeners on unmount
  useEffect(() => {
    return () => {
      // Clean up any lingering event listeners
      if (eventListenersRef.current.handleMouseMove) {
        window.removeEventListener(
          "mousemove",
          eventListenersRef.current.handleMouseMove,
        );
      }
      if (eventListenersRef.current.handleMouseUp) {
        window.removeEventListener(
          "mouseup",
          eventListenersRef.current.handleMouseUp,
        );
      }
    };
  }, []);

  // Handle scrubbing
  const handleMouseDown = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      setIsDragging(true);
      const rect = e.currentTarget.getBoundingClientRect();

      const handleMouseMove = (moveEvent: MouseEvent) => {
        const newX = moveEvent.clientX - rect.left;
        const newScrollLeft = Math.max(
          0,
          Math.min(totalWidth - containerWidth, newX - containerWidth / 2),
        );
        const frameIndex = Math.min(
          Math.floor((newScrollLeft + containerWidth / 2) / itemWidth),
          frames.length - 1,
        );
        onFrameChange(frameIndex);
      };

      const handleMouseUp = () => {
        setIsDragging(false);
        // Remove event listeners
        window.removeEventListener("mousemove", handleMouseMove);
        window.removeEventListener("mouseup", handleMouseUp);
        // Clear refs
        eventListenersRef.current = {};
      };

      // Store refs for cleanup
      eventListenersRef.current = { handleMouseMove, handleMouseUp };

      window.addEventListener("mousemove", handleMouseMove);
      window.addEventListener("mouseup", handleMouseUp);
    },
    [containerWidth, totalWidth, itemWidth, frames.length, onFrameChange],
  );

  // Scroll to current frame when it changes
  useEffect(() => {
    if (containerRef.current && !isDragging) {
      const targetScroll =
        currentFrameIndex * itemWidth - containerWidth / 2 + itemWidth / 2;
      const maxScroll = Math.max(0, totalWidth - containerWidth);
      const clampedScroll = Math.max(0, Math.min(targetScroll, maxScroll));
      containerRef.current.scrollLeft = clampedScroll;
      setScrollLeft(clampedScroll);
    }
  }, [currentFrameIndex, itemWidth, containerWidth, totalWidth, isDragging]);

  // OPTIMIZATION: Create a memoized lookup table for frame type colors
  // This avoids calling getFrameTypeColor() for every frame on every render
  const frameTypeColorMap = useMemo(() => {
    const colorMap = new Map<string, string>();
    for (let i = visibleRange.start; i < visibleRange.end; i++) {
      const frame = frames[i];
      if (frame && !colorMap.has(frame.frame_type)) {
        colorMap.set(frame.frame_type, getFrameTypeColor(frame.frame_type));
      }
    }
    return colorMap;
  }, [visibleRange, frames]);

  // Render visible frames
  const visibleFrames = useMemo(() => {
    const result: JSX.Element[] = [];
    for (let i = visibleRange.start; i < visibleRange.end; i++) {
      const frame = frames[i];
      if (!frame) continue;

      const isCurrent = i === currentFrameIndex;

      result.push(
        <div
          key={frame.frame_index}
          className={`virtualized-frame ${isCurrent ? "current" : ""}`}
          style={{ width: itemWidth }}
          onClick={() => handleFrameClick(i)}
        >
          <div
            className="virtualized-frame-indicator"
            style={{ backgroundColor: frameTypeColorMap.get(frame.frame_type) }}
          />
          <div className="virtualized-frame-number">{frame.frame_index}</div>
          <div className="virtualized-frame-type">{frame.frame_type}</div>
        </div>,
      );
    }
    return result;
  }, [
    visibleRange,
    frames,
    currentFrameIndex,
    itemWidth,
    handleFrameClick,
    frameTypeColorMap,
  ]);

  return (
    <div className="virtualized-filmstrip">
      <div
        ref={containerRef}
        className="virtualized-filmstrip-viewport"
        style={{ width: containerWidth }}
        onScroll={handleScroll}
        onMouseDown={handleMouseDown}
      >
        <div
          className="virtualized-filmstrip-content"
          style={{ width: totalWidth, transform: `translateX(${offset}px)` }}
        >
          {visibleFrames}
        </div>
      </div>

      {/* Scrollbar */}
      <div className="virtualized-filmstrip-scrollbar">
        <div
          className="virtualized-filmstrip-scrollbar-thumb"
          style={{
            width: `${(containerWidth / totalWidth) * 100}%`,
            left: `${(scrollLeft / totalWidth) * 100}%`,
          }}
        />
      </div>
    </div>
  );
});
