/**
 * Timeline View Component
 *
 * Wrapper for the existing Timeline component for Filmstrip integration
 */

import { memo } from "react";
import { MemoizedTimeline } from "../../Timeline";
import type { FrameInfo } from "../../../types/video";
import "./TimelineView.css";

interface TimelineViewProps {
  frames: FrameInfo[];
  currentFrameIndex: number;
  onFrameClick: (frameIndex: number) => void;
  getFrameTypeColorClass: (frameType: string) => string;
}

export const TimelineView = memo(function TimelineView({
  frames,
  currentFrameIndex,
  onFrameClick,
}: TimelineViewProps) {
  if (frames.length === 0) {
    return (
      <div className="timeline-view" role="region" aria-label="Timeline">
        <div
          className="timeline-empty"
          role="status"
          aria-label="No frames loaded"
        >
          <span className="codicon codicon-graph" aria-hidden="true"></span>
          <p>No frames loaded</p>
        </div>
      </div>
    );
  }

  return (
    <div className="timeline-view-filmstrip">
      <MemoizedTimeline frames={frames} />
    </div>
  );
});
