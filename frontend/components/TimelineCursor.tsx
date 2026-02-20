/**
 * Timeline Cursor Component
 *
 * Shows current frame position indicator
 */

import { memo } from "react";

interface TimelineCursorProps {
  positionPercent: number;
  frameIndex: number;
}

export const TimelineCursor = memo(function TimelineCursor({
  positionPercent,
  frameIndex,
}: TimelineCursorProps) {
  return (
    <div
      className="timeline-cursor"
      style={{ left: `${positionPercent}%` }}
      title={`Frame ${frameIndex}`}
      aria-hidden="true"
    />
  );
});
