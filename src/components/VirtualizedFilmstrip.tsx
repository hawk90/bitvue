/**
 * Virtualized Filmstrip Component
 *
 * Efficiently renders large frame lists using virtualization
 * Only renders visible frames plus buffer
 */

import { useRef, useState, useCallback, useEffect, useMemo, memo } from 'react';
import type { FrameInfo } from '../types/video';
import './VirtualizedFilmstrip.css';

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
  overscan = 5,
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

  // Get frame type color
  const getFrameTypeColor = useCallback((frameType: string) => {
    switch (frameType) {
      case 'I':
      case 'KEY':
        return 'var(--frame-i)';
      case 'P':
        return 'var(--frame-p)';
      case 'B':
        return 'var(--frame-b)';
      default:
        return 'var(--text-secondary)';
    }
  }, []);

  // Handle scroll
  const handleScroll = useCallback((e: React.UIEvent<HTMLDivElement>) => {
    setScrollLeft(e.currentTarget.scrollLeft);
  }, []);

  // Handle click on frame
  const handleFrameClick = useCallback((frameIndex: number) => {
    onFrameChange(frameIndex);
  }, [onFrameChange]);

  // Handle scrubbing
  const handleMouseDown = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    setIsDragging(true);
    const rect = e.currentTarget.getBoundingClientRect();

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const newX = moveEvent.clientX - rect.left;
      const newScrollLeft = Math.max(0, Math.min(totalWidth - containerWidth, newX - containerWidth / 2));
      const frameIndex = Math.min(Math.floor((newScrollLeft + containerWidth / 2) / itemWidth), frames.length - 1);
      onFrameChange(frameIndex);
    };

    const handleMouseUp = () => {
      setIsDragging(false);
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
  }, [containerWidth, totalWidth, itemWidth, frames.length, onFrameChange]);

  // Scroll to current frame when it changes
  useEffect(() => {
    if (containerRef.current && !isDragging) {
      const targetScroll = currentFrameIndex * itemWidth - containerWidth / 2 + itemWidth / 2;
      const maxScroll = Math.max(0, totalWidth - containerWidth);
      const clampedScroll = Math.max(0, Math.min(targetScroll, maxScroll));
      containerRef.current.scrollLeft = clampedScroll;
      setScrollLeft(clampedScroll);
    }
  }, [currentFrameIndex, itemWidth, containerWidth, totalWidth, isDragging]);

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
          className={`virtualized-frame ${isCurrent ? 'current' : ''}`}
          style={{ width: itemWidth }}
          onClick={() => handleFrameClick(i)}
        >
          <div
            className="virtualized-frame-indicator"
            style={{ backgroundColor: getFrameTypeColor(frame.frame_type) }}
          />
          <div className="virtualized-frame-number">{frame.frame_index}</div>
          <div className="virtualized-frame-type">{frame.frame_type}</div>
        </div>
      );
    }
    return result;
  }, [visibleRange, frames, currentFrameIndex, itemWidth, getFrameTypeColor, handleFrameClick]);

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
