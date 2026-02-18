/**
 * Minimap View Component
 *
 * Displays a compact grid view of all frames
 */

import type { FrameInfo } from "../types/video";
import { memo } from "react";

interface MinimapViewProps {
  frames: FrameInfo[];
  currentFrameIndex: number;
  onFrameClick: (frameIndex: number) => void;
  getFrameTypeColorClass: (frameType: string) => string;
  getFrameTypeColor: (frameType: string) => string;
}

export const MinimapView = memo(function MinimapView({
  frames,
  currentFrameIndex,
  onFrameClick,
  getFrameTypeColorClass,
  getFrameTypeColor,
}: MinimapViewProps) {
  return (
    <div className="filmstrip-minimap">
      <div className="minimap-grid">
        {frames.map((frame) => (
          <div
            key={frame.frame_index}
            data-frame-index={frame.frame_index}
            className={`minimap-cell ${getFrameTypeColorClass(frame.frame_type)} ${
              frame.frame_index === currentFrameIndex ? "selected" : ""
            }`}
            onClick={() => onFrameClick(frame.frame_index)}
            style={{ backgroundColor: getFrameTypeColor(frame.frame_type) }}
            title={`Frame ${frame.frame_index}: ${frame.frame_type}`}
          />
        ))}
      </div>
    </div>
  );
});
