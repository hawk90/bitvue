/**
 * Timeline Component - VQAnalyzer Style
 *
 * Shows overall frame structure with I/P/B distribution and frame sizes
 */

import { useState, useCallback, useRef, useEffect, useMemo, memo } from 'react';
import { useSelection } from '../contexts/SelectionContext';
import { TimelineHeader } from './TimelineHeader';
import { TimelineCursor } from './TimelineCursor';
import { TimelineTooltip } from './TimelineTooltip';
import { TimelineThumbnails } from './TimelineThumbnails';
import type { FrameInfo } from '../types/video';
import './Timeline.css';

interface TimelineProps {
  frames: FrameInfo[];
  className?: string;
}

function Timeline({ frames, className = '' }: TimelineProps) {
  const { selection, setFrameSelection } = useSelection();
  const [hoverPosition, setHoverPosition] = useState<number | null>(null);
  // Local state for the highlighted frame - single source of truth
  const [highlightedFrameIndex, setHighlightedFrameIndex] = useState<number>(0);
  // Track if we're dragging to ignore external selection changes
  const isDraggingRef = useRef(false);
  // Ref to store current drag index for the closure
  const dragIndexRef = useRef<number>(0);

  // Ref to the timeline container
  const timelineRef = useRef<HTMLDivElement>(null);
  // Refs to each frame element
  const frameRefs = useRef<(HTMLDivElement | null)[]>([]);

  // Calculate cursor position based on actual DOM element position (memoized)
  const cursorPosition = useMemo(() => {
    // Add bounds check to prevent array index out of bounds
    if (
      timelineRef.current &&
      highlightedFrameIndex >= 0 &&
      highlightedFrameIndex < frameRefs.current.length &&
      frameRefs.current[highlightedFrameIndex]
    ) {
      const frameEl = frameRefs.current[highlightedFrameIndex];
      const containerEl = timelineRef.current;
      if (frameEl && containerEl) {
        const frameRect = frameEl.getBoundingClientRect();
        const containerRect = containerEl.getBoundingClientRect();
        // Calculate the center of the frame relative to the container
        const frameCenter = frameRect.left - containerRect.left + (frameRect.width / 2);
        return (frameCenter / containerRect.width) * 100;
      }
    }
    return 0;
  }, [highlightedFrameIndex, frames.length]);

  // Sync highlighted frame with selection from external sources (not during drag)
  useEffect(() => {
    if (!isDraggingRef.current && selection?.frame?.frameIndex !== undefined) {
      setHighlightedFrameIndex(selection.frame.frameIndex);
    }
  }, [selection?.frame?.frameIndex]);

  const getFrameIndexFromEvent = useCallback((e: React.MouseEvent<HTMLDivElement>): number => {
    // First check if we clicked directly on a frame element
    const target = e.target as HTMLElement;
    const frameElement = target.closest('.timeline-thumb');
    if (frameElement) {
      const frameIndex = parseInt(frameElement.getAttribute('data-frame-index') || '0', 10);
      return frameIndex;
    }

    // Otherwise calculate from position
    const rect = e.currentTarget.getBoundingClientRect();
    const percent = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    return Math.min(Math.floor(percent * frames.length), frames.length - 1);
  }, [frames.length]);

  const handleMouseMove = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const percent = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    setHoverPosition(percent);
  }, []);

  const handleMouseDown = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    const frameIndex = getFrameIndexFromEvent(e);
    setHighlightedFrameIndex(frameIndex);
    dragIndexRef.current = frameIndex;
    isDraggingRef.current = true;

    // Store the timeline element for drag handlers
    const timelineEl = timelineRef.current;
    if (!timelineEl) return;

    // Throttle frame updates for smoother drag
    let lastUpdateFrame = -1;

    // Set up drag handlers
    const handleDragMove = (moveEvent: MouseEvent) => {
      const rect = timelineEl.getBoundingClientRect();
      const dragPercent = Math.max(0, Math.min(1, (moveEvent.clientX - rect.left) / rect.width));
      const dragFrameIndex = Math.min(Math.floor(dragPercent * frames.length), frames.length - 1);

      // Only update if frame actually changed
      if (dragFrameIndex !== lastUpdateFrame) {
        lastUpdateFrame = dragFrameIndex;
        dragIndexRef.current = dragFrameIndex;
        setHighlightedFrameIndex(dragFrameIndex);
      }
      setHoverPosition(dragPercent);
    };

    const handleDragUp = () => {
      isDraggingRef.current = false;
      // Update SelectionContext with final position from ref
      setFrameSelection({ stream: 'A', frameIndex: dragIndexRef.current }, 'timeline');
      window.removeEventListener('mousemove', handleDragMove);
      window.removeEventListener('mouseup', handleDragUp);
    };

    window.addEventListener('mousemove', handleDragMove, { passive: true });
    window.addEventListener('mouseup', handleDragUp);
  }, [getFrameIndexFromEvent, setFrameSelection, frames.length]);

  const handleMouseLeave = useCallback(() => {
    setHoverPosition(null);
  }, []);

  // Callback to receive frame refs from TimelineThumbnails
  const handleFrameRefsChange = useCallback((refs: (HTMLDivElement | null)[]) => {
    frameRefs.current = refs;
  }, []);

  // Keyboard navigation handler
  const handleKeyDown = useCallback((e: React.KeyboardEvent<HTMLDivElement>) => {
    if (e.key === 'ArrowLeft' && highlightedFrameIndex > 0) {
      const newIndex = highlightedFrameIndex - 1;
      setHighlightedFrameIndex(newIndex);
      setFrameSelection({ stream: 'A', frameIndex: newIndex }, 'timeline');
    } else if (e.key === 'ArrowRight' && highlightedFrameIndex < frames.length - 1) {
      const newIndex = highlightedFrameIndex + 1;
      setHighlightedFrameIndex(newIndex);
      setFrameSelection({ stream: 'A', frameIndex: newIndex }, 'timeline');
    }
  }, [highlightedFrameIndex, frames.length, setFrameSelection]);

  const hoverPercent = hoverPosition !== null ? hoverPosition * 100 : null;
  const hoverFrameIndex = hoverPosition !== null ? Math.min(Math.floor(hoverPosition * frames.length), frames.length - 1) : null;

  if (frames.length === 0) {
    return (
      <div className={`timeline ${className}`} role="region" aria-label="Timeline">
        <TimelineHeader currentFrame={0} totalFrames={0} />
        <div className="timeline-content">
          <div className="timeline-empty" role="status" aria-label="No frames loaded">
            <span className="codicon codicon-graph" aria-hidden="true"></span>
            <p>No frames loaded</p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className={`timeline ${className}`} role="region" aria-label="Timeline">
      <TimelineHeader currentFrame={highlightedFrameIndex} totalFrames={frames.length} />
      {/* Timeline Content */}
      <div className="timeline-content" ref={timelineRef}>
        {/* Compressed thumbnails (Touch Bar style) */}
        <TimelineThumbnails
          frames={frames}
          highlightedFrameIndex={highlightedFrameIndex}
          onMouseDown={handleMouseDown}
          onMouseMove={handleMouseMove}
          onMouseLeave={handleMouseLeave}
          onKeyDown={handleKeyDown}
          onFrameRefsChange={handleFrameRefsChange}
        />

        {/* Current Position Cursor */}
        <TimelineCursor positionPercent={cursorPosition} frameIndex={highlightedFrameIndex} />
      </div>

      {/* Hover tooltip */}
      {hoverFrameIndex !== null && hoverFrameIndex < frames.length && hoverPercent !== null && (
        <TimelineTooltip frame={frames[hoverFrameIndex]} positionPercent={hoverPercent} />
      )}
    </div>
  );
}

export const MemoizedTimeline = memo(Timeline, (prevProps, nextProps) => {
  return prevProps.frames === nextProps.frames && prevProps.className === nextProps.className;
});

// Export as default for backward compatibility
export default memo(Timeline);
