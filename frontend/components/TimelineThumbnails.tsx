/**
 * Timeline Thumbnails Component
 *
 * Frame bar strip with drag interaction
 */

import { forwardRef, useEffect, useRef } from "react";

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
  onMouseDown: (e: React.MouseEvent<HTMLDivElement>) => void;
  onMouseMove: (e: React.MouseEvent<HTMLDivElement>) => void;
  onMouseLeave: () => void;
  onKeyDown: (e: React.KeyboardEvent<HTMLDivElement>) => void;
  onFrameRefsChange?: (refs: (HTMLDivElement | null)[]) => void;
}

export const TimelineThumbnails = forwardRef<
  HTMLDivElement,
  TimelineThumbnailsProps
>(
  (
    {
      frames,
      highlightedFrameIndex,
      onMouseDown,
      onMouseMove,
      onMouseLeave,
      onKeyDown,
      onFrameRefsChange,
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
        role="slider"
        aria-label="Frame position"
        aria-valuemin={0}
        aria-valuemax={frames.length - 1}
        aria-valuenow={highlightedFrameIndex}
        aria-valuetext={`Frame ${highlightedFrameIndex} of ${frames.length}`}
        tabIndex={0}
        onKeyDown={onKeyDown}
        title="Click to seek, drag to scrub"
      >
        {frames.map((frame, idx) => {
          const isSelected = frame.frame_index === highlightedFrameIndex;

          return (
            <div
              key={frame.frame_index}
              ref={(el) => {
                internalFrameRefs.current[idx] = el;
              }}
              className={`timeline-thumb ${getFrameBarClass(frame.frame_type)} ${
                isSelected ? "selected" : ""
              }`}
              data-frame-index={frame.frame_index}
              title={`Frame ${frame.frame_index}: ${frame.frame_type}`}
              aria-label={`Frame ${frame.frame_index}, type ${frame.frame_type}`}
              aria-current={isSelected ? "true" : undefined}
            />
          );
        })}
      </div>
    );
  },
);
