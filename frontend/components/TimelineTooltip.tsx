/**
 * Timeline Tooltip Component
 *
 * Shows frame info on hover
 */

import type { FrameInfo } from '../types/video';
import { memo } from 'react';

interface TimelineTooltipProps {
  frame: FrameInfo;
  positionPercent: number;
}

export const TimelineTooltip = memo(function TimelineTooltip({ frame, positionPercent }: TimelineTooltipProps) {
  return (
    <div
      className="timeline-tooltip"
      style={{
        left: `${positionPercent}%`,
      }}
    >
      <div className="tooltip-frame">#{frame.frame_index}</div>
      <div className="tooltip-type">{frame.frame_type}</div>
      <div className="tooltip-size">{(frame.size / 1024).toFixed(1)} KB</div>
    </div>
  );
});
