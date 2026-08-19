/**
 * Timeline Cursor Component
 *
 * Shows current frame position indicator
 */

import { memo } from "react";

interface TimelineCursorProps {
  /** Pixel offset within `.timeline-thumbnails`' own content box (`offsetLeft`-based, not a
   * percentage of the viewport-visible width) -- see that component's doc for why: rendering
   * this as a DOM child of the (potentially horizontally-scrolled) bar strip means a `left: Npx`
   * position scrolls natively together with the bars with no extra scroll-tracking JS needed,
   * unlike a percent-of-container-width position computed once and never recomputed on scroll. */
  positionPx: number;
  frameIndex: number;
}

export const TimelineCursor = memo(function TimelineCursor({
  positionPx,
  frameIndex,
}: TimelineCursorProps) {
  return (
    <div
      className="timeline-cursor"
      style={{ left: `${positionPx}px` }}
      title={`Frame ${frameIndex}`}
      aria-hidden="true"
    />
  );
});
