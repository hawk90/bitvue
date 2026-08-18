/**
 * Timeline Component - Style
 *
 * Shows overall frame structure with I/P/B distribution and frame sizes
 */

import { useState, useCallback, useRef, useEffect, useMemo, memo } from "react";
import { useSelection } from "../contexts/SelectionContext";
import { TimelineHeader } from "./TimelineHeader";
import { TimelineTooltip } from "./TimelineTooltip";
import { TimelineThumbnails } from "./TimelineThumbnails";
import {
  getContextMenuItems,
  type ContextMenuItemWire,
} from "../services/electronBridgeService";
import { useExportEvidenceBundle } from "../hooks/useExportEvidenceBundle";
import { ContextMenu } from "./ContextMenu";
import type { FrameInfo } from "../types/video";
import "./Timeline.css";

interface TimelineProps {
  frames: FrameInfo[];
  className?: string;
}

function Timeline({ frames, className = "" }: TimelineProps) {
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
  // Cache for timeline bounding rect to avoid repeated getBoundingClientRect calls
  const rectCache = useRef<DOMRect | null>(null);

  // Right-click context menu (Phase 7.6, "Timeline" scope) -- see ContextMenu component doc.
  const exportEvidence = useExportEvidenceBundle();
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    items: ContextMenuItemWire[];
  } | null>(null);

  const handleTimelineContextMenu = useCallback(
    (event: React.MouseEvent) => {
      event.preventDefault();
      const hasSelection = selection?.frame?.frameIndex !== undefined;
      const x = event.clientX;
      const y = event.clientY;
      getContextMenuItems("Timeline", hasSelection, false)
        .then((items) => setContextMenu({ x, y, items }))
        .catch(() => setContextMenu(null));
    },
    [selection?.frame?.frameIndex],
  );

  const handleContextMenuSelect = useCallback(
    (command: string) => {
      if (command === "Export.EvidenceBundle") {
        void exportEvidence();
      } else if (
        command === "Copy.Selection" &&
        selection?.frame?.frameIndex !== undefined
      ) {
        const frame = frames[selection.frame.frameIndex];
        const text = frame
          ? `Frame ${frame.frame_index} (pts=${frame.pts})`
          : `Frame ${selection.frame.frameIndex}`;
        void navigator.clipboard.writeText(text);
      }
    },
    [exportEvidence, selection?.frame?.frameIndex, frames],
  );

  // Calculate cursor position based on actual DOM element position (memoized). Pixel-based
  // (`offsetLeft`, relative to `.timeline-thumbnails`' own content box -- the frame bars'
  // `offsetParent`, since that's their nearest `position: relative` ancestor), NOT a percentage
  // of the outer container's width: `.timeline-thumbnails` can now scroll horizontally on its
  // own (see Timeline.css's doc on that selector), and `offsetLeft` stays valid regardless of
  // current scroll position -- a percent-of-viewport-width value computed here would go stale
  // the moment the user scrolls without changing `highlightedFrameIndex` (this `useMemo` only
  // re-runs on that dependency). See TimelineCursor's doc for the other half of this fix
  // (rendering the cursor as a scroll-following DOM child instead of a percent-positioned
  // sibling).
  const cursorPosition = useMemo(() => {
    // Add bounds check to prevent array index out of bounds
    if (
      highlightedFrameIndex >= 0 &&
      highlightedFrameIndex < frameRefs.current.length &&
      frameRefs.current[highlightedFrameIndex]
    ) {
      const frameEl = frameRefs.current[highlightedFrameIndex];
      if (frameEl) {
        return frameEl.offsetLeft + frameEl.offsetWidth / 2;
      }
    }
    return 0;
  }, [highlightedFrameIndex]);

  // Sync highlighted frame with selection from external sources (not during drag)
  useEffect(() => {
    if (!isDraggingRef.current && selection?.frame?.frameIndex !== undefined) {
      setHighlightedFrameIndex(selection.frame.frameIndex);
    }
  }, [selection?.frame?.frameIndex]);

  // Helper function to get cached timeline rect
  const getTimelineRect = useCallback((): DOMRect | null => {
    if (!timelineRef.current) return null;
    rectCache.current ??= timelineRef.current.getBoundingClientRect();
    return rectCache.current;
  }, []);

  // Invalidate rect cache on window resize
  useEffect(() => {
    const handleResize = () => {
      rectCache.current = null;
    };
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, []);

  // Real DOM hit-testing (`document.elementFromPoint`) accounts for `.timeline-thumbnails`'
  // current horizontal scroll position automatically -- unlike the plain `(clientX -
  // containerRect.left) / containerRect.width * frameCount` percent math this falls back to,
  // which silently assumes every frame's bar is evenly spread across the *container's* full
  // width, an assumption that stops holding the moment the strip can scroll (Timeline.css's doc
  // on `.timeline-thumbnails`). `elementFromPoint` isn't implemented in jsdom (this repo's test
  // environment) -- guarded so tests still exercise the percent fallback exactly as before.
  const frameIndexFromPoint = useCallback(
    (clientX: number, clientY: number, containerRect: DOMRect): number => {
      if (typeof document.elementFromPoint === "function") {
        const hit = document
          .elementFromPoint(clientX, clientY)
          ?.closest(".timeline-thumb");
        if (hit) {
          const idx = parseInt(hit.getAttribute("data-frame-index") ?? "", 10);
          if (!Number.isNaN(idx)) return idx;
        }
      }
      const percent = Math.max(
        0,
        Math.min(1, (clientX - containerRect.left) / containerRect.width),
      );
      return Math.min(Math.floor(percent * frames.length), frames.length - 1);
    },
    [frames.length],
  );

  const getFrameIndexFromEvent = useCallback(
    (e: React.MouseEvent<HTMLDivElement>): number => {
      // First check if we clicked directly on a frame element (covers the vast majority of
      // real clicks -- bars only have a 1px gap between them).
      const target = e.target as HTMLElement;
      const frameElement = target.closest(".timeline-thumb");
      if (frameElement) {
        const frameIndex = parseInt(
          frameElement.getAttribute("data-frame-index") || "0",
          10,
        );
        return frameIndex;
      }

      // Otherwise (clicked a gap) -- real hit-testing, falls back to percent math only when
      // unavailable (frameIndexFromPoint's doc).
      const rect = getTimelineRect();
      if (!rect) return 0;
      return frameIndexFromPoint(e.clientX, e.clientY, rect);
    },
    [getTimelineRect, frameIndexFromPoint],
  );

  const handleMouseMove = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      const rect = getTimelineRect();
      if (!rect) return;
      const percent = Math.max(
        0,
        Math.min(1, (e.clientX - rect.left) / rect.width),
      );
      setHoverPosition(percent);
    },
    [getTimelineRect],
  );

  const handleMouseDown = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      const frameIndex = getFrameIndexFromEvent(e);
      setHighlightedFrameIndex(frameIndex);
      dragIndexRef.current = frameIndex;
      isDraggingRef.current = true;

      // Store the timeline element for drag handlers
      const timelineEl = timelineRef.current;
      if (!timelineEl) return;

      // OPTIMIZATION: Cache rect once at drag start to avoid repeated getBoundingClientRect calls
      const timelineRect = timelineEl.getBoundingClientRect();

      // Throttle frame updates for smoother drag
      let lastUpdateFrame = -1;

      // Set up drag handlers
      const handleDragMove = (moveEvent: MouseEvent) => {
        // Real hit-testing (scroll-safe, frameIndexFromPoint's doc) -- the plain percent-of-
        // timelineRect math this used to rely on exclusively would silently desync from the
        // visible bars the moment `.timeline-thumbnails` is scrolled.
        const dragFrameIndex = frameIndexFromPoint(
          moveEvent.clientX,
          moveEvent.clientY,
          timelineRect,
        );
        const dragPercent = Math.max(
          0,
          Math.min(
            1,
            (moveEvent.clientX - timelineRect.left) / timelineRect.width,
          ),
        );

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
        setFrameSelection(
          { stream: "A", frameIndex: dragIndexRef.current },
          "timeline",
        );
        window.removeEventListener("mousemove", handleDragMove);
        window.removeEventListener("mouseup", handleDragUp);
      };

      window.addEventListener("mousemove", handleDragMove, { passive: true });
      window.addEventListener("mouseup", handleDragUp);
    },
    [getFrameIndexFromEvent, setFrameSelection, frames.length],
  );

  const handleMouseLeave = useCallback(() => {
    setHoverPosition(null);
  }, []);

  // Callback to receive frame refs from TimelineThumbnails
  const handleFrameRefsChange = useCallback(
    (refs: (HTMLDivElement | null)[]) => {
      frameRefs.current = refs;
    },
    [],
  );

  // Keyboard navigation handler
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLDivElement>) => {
      if (e.key === "ArrowLeft" && highlightedFrameIndex > 0) {
        e.preventDefault();
        const newIndex = highlightedFrameIndex - 1;
        setHighlightedFrameIndex(newIndex);
        setFrameSelection({ stream: "A", frameIndex: newIndex }, "timeline");
      } else if (
        e.key === "ArrowRight" &&
        highlightedFrameIndex < frames.length - 1
      ) {
        e.preventDefault();
        const newIndex = highlightedFrameIndex + 1;
        setHighlightedFrameIndex(newIndex);
        setFrameSelection({ stream: "A", frameIndex: newIndex }, "timeline");
      }
    },
    [highlightedFrameIndex, frames.length, setFrameSelection],
  );

  const hoverPercent = hoverPosition !== null ? hoverPosition * 100 : null;
  const hoverFrameIndex =
    hoverPosition !== null
      ? Math.min(Math.floor(hoverPosition * frames.length), frames.length - 1)
      : null;

  if (frames.length === 0) {
    return (
      <div
        className={`timeline ${className}`}
        role="region"
        aria-label="Timeline"
      >
        <TimelineHeader currentFrame={0} totalFrames={0} />
        <div className="timeline-content">
          <div
            className="timeline-empty"
            role="status"
            aria-label="No frames loaded"
          >
            <span className="codicon codicon-graph" aria-hidden="true"></span>
            <p>No frames loaded</p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div
      className={`timeline ${className}`}
      role="region"
      aria-label="Timeline"
    >
      <TimelineHeader
        currentFrame={highlightedFrameIndex}
        totalFrames={frames.length}
      />
      {/* Timeline Content */}
      <div
        className="timeline-content"
        ref={timelineRef}
        onContextMenu={handleTimelineContextMenu}
      >
        {/* Compressed thumbnails (Touch Bar style) -- renders the cursor internally as its own
            scroll-following child, see TimelineCursor's doc. */}
        <TimelineThumbnails
          frames={frames}
          highlightedFrameIndex={highlightedFrameIndex}
          cursorPositionPx={cursorPosition}
          onMouseDown={handleMouseDown}
          onMouseMove={handleMouseMove}
          onMouseLeave={handleMouseLeave}
          onKeyDown={handleKeyDown}
          onFrameRefsChange={handleFrameRefsChange}
        />
      </div>

      {/* Hover tooltip */}
      {hoverFrameIndex !== null &&
        hoverFrameIndex < frames.length &&
        hoverPercent !== null && (
          <TimelineTooltip
            frame={frames[hoverFrameIndex]}
            positionPercent={hoverPercent}
          />
        )}

      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          items={contextMenu.items}
          onSelect={handleContextMenuSelect}
          onClose={() => setContextMenu(null)}
        />
      )}
    </div>
  );
}

export const MemoizedTimeline = memo(Timeline, (prevProps, nextProps) => {
  return (
    prevProps.frames === nextProps.frames &&
    prevProps.className === nextProps.className
  );
});

// Export as default for backward compatibility
export default memo(Timeline);
