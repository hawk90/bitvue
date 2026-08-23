/**
 * Timeline Thumbnails Component
 *
 * Frame bar strip with drag interaction
 */

import { forwardRef, useEffect, useRef } from "react";
import { TimelineCursor } from "./TimelineCursor";

interface FrameInfo {
  frame_index: number;
  frame_type: string;
  size: number;
  pts?: number;
  poc?: number;
  key_frame?: boolean;
  thumbnail?: string;
  display_order?: number;
  coding_order?: number;
  temporal_id?: number;
  spatial_id?: number;
}

interface TimelineThumbnailsProps {
  frames: FrameInfo[];
  highlightedFrameIndex: number;
  /** Pixel offset for the cursor, relative to this component's own root -- see
   * `TimelineCursor`'s doc for why this must be a DOM child of that (potentially scrolled) root
   * rather than a sibling positioned by percentage. `0` (same fallback the previous
   * percent-based calc used) while it can't yet be measured. */
  cursorPositionPx: number;
  onMouseDown: (e: React.MouseEvent<HTMLDivElement>) => void;
  onMouseMove: (e: React.MouseEvent<HTMLDivElement>) => void;
  onMouseLeave: () => void;
  onKeyDown: (e: React.KeyboardEvent<HTMLDivElement>) => void;
  onFrameRefsChange?: (refs: (HTMLDivElement | null)[]) => void;
  /** INT-02: visual CSS-transform zoom factor (1 = 100%), applied as `scaleX` -- same technique
   *  as `Filmstrip/views/ThumbnailsView.tsx`'s existing wheel-zoom. The wheel listener itself is
   *  NOT wired here as a React `onWheel` prop -- see Timeline.tsx's `thumbnailsRef` doc for why
   *  (needs a real non-passive `addEventListener`, attached via the forwarded `ref` below). */
  zoom?: number;
  /** INT-02: inclusive frame-index range to highlight (live shift-drag preview or the last
   *  committed "range" temporal selection) -- `null` when there's nothing to highlight. */
  rangeSelection?: { start: number; end: number } | null;
}

export const TimelineThumbnails = forwardRef<
  HTMLDivElement,
  TimelineThumbnailsProps
>(
  (
    {
      frames,
      highlightedFrameIndex,
      cursorPositionPx,
      onMouseDown,
      onMouseMove,
      onMouseLeave,
      onKeyDown,
      onFrameRefsChange,
      zoom = 1,
      rangeSelection = null,
    },
    ref,
  ) => {
    const internalFrameRefs = useRef<(HTMLDivElement | null)[]>([]);

    const getFrameBarClass = (frameType: string) => {
      const type = frameType.toLowerCase();
      if (type === "i" || type === "key") return "timeline-bar-i";
      if (type === "p" || type === "inter") return "timeline-bar-p";
      if (type.startsWith("b")) return "timeline-bar-b";
      return "timeline-bar-unknown";
    };

    // Notify parent when frame refs change
    useEffect(() => {
      if (onFrameRefsChange) {
        onFrameRefsChange(internalFrameRefs.current);
      }
    }, [frames.length, highlightedFrameIndex, onFrameRefsChange]);

    return (
      <div
        ref={ref}
        className="timeline-thumbnails"
        onMouseDown={onMouseDown}
        onMouseMove={onMouseMove}
        onMouseLeave={() => onMouseLeave()}
        style={{ transform: `scaleX(${zoom})`, transformOrigin: "left center" }}
        role="slider"
        aria-label="Frame position"
        aria-valuemin={0}
        aria-valuemax={frames.length - 1}
        aria-valuenow={highlightedFrameIndex}
        aria-valuetext={`Frame ${highlightedFrameIndex} of ${frames.length}`}
        tabIndex={0}
        onKeyDown={onKeyDown}
        title="Click to seek, drag to scrub, Shift+drag to select a range, Ctrl/Cmd+wheel to zoom"
      >
        {frames.map((frame, idx) => {
          const isSelected = frame.frame_index === highlightedFrameIndex;
          const isInRange =
            rangeSelection !== null &&
            frame.frame_index >= rangeSelection.start &&
            frame.frame_index <= rangeSelection.end;

          return (
            <div
              key={frame.frame_index}
              ref={(el) => {
                internalFrameRefs.current[idx] = el;
              }}
              className={`timeline-thumb ${getFrameBarClass(frame.frame_type)} ${
                isSelected ? "selected" : ""
              } ${isInRange ? "in-range" : ""}`}
              data-frame-index={frame.frame_index}
              title={`Frame ${frame.frame_index}: ${frame.frame_type}`}
              aria-label={`Frame ${frame.frame_index}, type ${frame.frame_type}`}
              aria-current={isSelected ? "true" : undefined}
            />
          );
        })}

        {/* Rendered as a child of this (potentially horizontally-scrolled) strip, not a sibling
            positioned by percentage of the outer container -- see TimelineCursor's doc. */}
        <TimelineCursor
          positionPx={cursorPositionPx}
          frameIndex={highlightedFrameIndex}
        />
      </div>
    );
  },
);

TimelineThumbnails.displayName = "TimelineThumbnails";
