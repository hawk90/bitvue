/**
 * Frame Navigation Controls Component
 *
 * First/Previous/Next/Last frame buttons and frame input
 */

import { memo } from 'react';

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
        >
          <span className="codicon codicon-chevron-left"></span>
          <span className="codicon codicon-chevron-left"></span>
        </button>
        <button
          onClick={onPrevFrame}
          disabled={currentFrameIndex === 0}
          title="Previous Frame (←)"
        >
          <span className="codicon codicon-chevron-left"></span>
        </button>
      </div>

      <div className="yuv-toolbar-group">
        <input
          type="number"
          min={0}
          max={totalFrames - 1}
          value={currentFrameIndex}
          onChange={(e) => {
            const val = parseInt(e.target.value);
            if (!isNaN(val) && val >= 0 && val < totalFrames) {
              onFrameChange(val);
            }
          }}
          className="yuv-frame-input"
        />
        <span className="yuv-frame-divider">/</span>
        <span className="yuv-frame-total">{totalFrames - 1}</span>
      </div>

      <div className="yuv-toolbar-group">
        <button
          onClick={onNextFrame}
          disabled={currentFrameIndex >= totalFrames - 1}
          title="Next Frame (→)"
        >
          <span className="codicon codicon-chevron-right"></span>
        </button>
        <button
          onClick={onLastFrame}
          disabled={currentFrameIndex >= totalFrames - 1}
          title="Last Frame (End)"
        >
          <span className="codicon codicon-chevron-right"></span>
          <span className="codicon codicon-chevron-right"></span>
        </button>
      </div>
    </>
  );
});
