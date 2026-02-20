/**
 * Timeline Header Component
 *
 * Shows timeline title and frame count
 */

import { memo } from "react";

interface TimelineHeaderProps {
  currentFrame: number;
  totalFrames: number;
}

export const TimelineHeader = memo(function TimelineHeader({
  currentFrame,
  totalFrames,
}: TimelineHeaderProps) {
  return (
    <div className="timeline-header">
      <div className="timeline-title">
        <span className="codicon codicon-graph" aria-hidden="true"></span>
        Timeline
      </div>
      <div className="timeline-info" role="status" aria-live="polite">
        {currentFrame + 1} / {totalFrames}
      </div>
    </div>
  );
});
