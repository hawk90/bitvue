/**
 * Frame Navigation Controls Component
 *
 * First/Previous/Next/Last frame buttons and frame input
 */

import { memo } from "react";

interface FrameNavigationControlsProps {
  currentFrameIndex: number;
  totalFrames: number;
  onFirstFrame: () => void;
  onPrevFrame: () => void;
  onNextFrame: () => void;
  onLastFrame: () => void;
  onFrameChange: (frameIndex: number) => void;
}

export const FrameNavigationControls = memo(function FrameNavigationControls({
  currentFrameIndex,
  totalFrames,
  onFirstFrame,
  onPrevFrame,
  onNextFrame,
  onLastFrame,
  onFrameChange,
}: FrameNavigationControlsProps) {
  return (
    <>
      <div className="yuv-toolbar-group">
        <button
          onClick={onFirstFrame}
          disabled={currentFrameIndex === 0}
          title="First Frame (Home)"
          aria-label="Go to first frame"
        >
          <span className="codicon codicon-triangle-left"></span>
          <span className="codicon codicon-chevron-left"></span>
        </button>
        <button
          onClick={onPrevFrame}
          disabled={currentFrameIndex === 0}
          title="Previous Frame (←)"
          aria-label="Go to previous frame"
        >
          <span className="codicon codicon-chevron-left"></span>
        </button>
      </div>

      <div className="yuv-toolbar-group">
        <input
          type="number"
          min={1}
          max={totalFrames}
          value={currentFrameIndex + 1}
          onChange={(e) => {
            const val = parseInt(e.target.value);
            if (!isNaN(val) && val >= 1 && val <= totalFrames) {
              onFrameChange(val - 1);
            }
          }}
          className="yuv-frame-input"
          aria-label={`Frame number, 1 to ${totalFrames}`}
          title={`Frame ${currentFrameIndex + 1} of ${totalFrames}`}
        />
        <span className="yuv-frame-divider">/</span>
        <span className="yuv-frame-total">{totalFrames}</span>
      </div>

      <div className="yuv-toolbar-group">
        <button
          onClick={onNextFrame}
          disabled={currentFrameIndex >= totalFrames - 1}
          title="Next Frame (→)"
          aria-label="Go to next frame"
        >
          <span className="codicon codicon-chevron-right"></span>
        </button>
        <button
          onClick={onLastFrame}
          disabled={currentFrameIndex >= totalFrames - 1}
          title="Last Frame (End)"
          aria-label="Go to last frame"
        >
          <span className="codicon codicon-chevron-right"></span>
          <span className="codicon codicon-triangle-right"></span>
        </button>
      </div>
    </>
  );
});
