/**
 * Timeline Header Component
 *
 * Shows timeline title and frame count
 */

import { memo } from "react";
import { PtsQualityBadge } from "./common/PtsQualityBadge";

interface TimelineHeaderProps {
  currentFrame: number;
  totalFrames: number;
  /** EDGE-03: stream-wide PTS quality, when known. Omit (or `null`) to render no badge. */
  ptsQuality?: "Ok" | "Warn" | "Bad" | null;
}

export const TimelineHeader = memo(function TimelineHeader({
  currentFrame,
  totalFrames,
  ptsQuality,
}: TimelineHeaderProps) {
  return (
    <div className="timeline-header">
      <div className="timeline-title">
        <span className="codicon codicon-graph" aria-hidden="true"></span>
        Timeline
        <PtsQualityBadge ptsQuality={ptsQuality ?? null} />
      </div>
      <div className="timeline-info" role="status" aria-live="polite">
        {currentFrame + 1} / {totalFrames}
      </div>
    </div>
  );
});
