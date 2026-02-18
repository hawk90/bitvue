/**
 * DPB View Tab Component
 *
 * Displays Decoded Picture Buffer state
 */

import { memo } from "react";

interface FrameInfo {
  frame_index: number;
  frame_type: string;
  pts?: number;
}

interface DpbViewTabProps {
  currentFrame: {
    frame_index: number;
    frame_type: string;
    pts?: number;
    ref_frames?: number[];
  } | null;
  frames: FrameInfo[];
}

export const DpbViewTab = memo(function DpbViewTab({
  currentFrame,
  frames,
}: DpbViewTabProps) {
  if (!currentFrame) {
    return (
      <div className="hex-empty">
        <span className="codicon codicon-database"></span>
        <span>No frame selected</span>
      </div>
    );
  }

  const refFrames = currentFrame.ref_frames || [];

  return (
    <div className="dpb-info-content">
      <div className="dpb-header">
        <span className="codicon codicon-database"></span>
        Decoded Picture Buffer
      </div>

      <div className="dpb-info">
        <span className="dpb-info-label">Current Frame:</span>
        <span className="dpb-info-value">
          #{currentFrame.frame_index} ({currentFrame.frame_type})
        </span>
      </div>

      <div className="dpb-table">
        <div className="dpb-row dpb-header-row">
          <span>Idx</span>
          <span>Frame</span>
          <span>Type</span>
          <span>PTS</span>
          <span>Status</span>
        </div>

        {/* Current frame in DPB */}
        <div className="dpb-row dpb-current">
          <span>-</span>
          <span>{currentFrame.frame_index}</span>
          <span
            className={`frame-type-${currentFrame.frame_type.toLowerCase()}`}
          >
            {currentFrame.frame_type}
          </span>
          <span>{currentFrame.pts ?? "N/A"}</span>
          <span className="dpb-current">Current</span>
        </div>

        {/* Reference frames */}
        {refFrames.map((refIdx, i) => {
          const refFrame = frames.find((f) => f.frame_index === refIdx);
          return (
            <div key={i} className="dpb-row">
              <span>{i}</span>
              <span>{refIdx}</span>
              {refFrame ? (
                <>
                  <span
                    className={`frame-type-${refFrame.frame_type.toLowerCase()}`}
                  >
                    {refFrame.frame_type}
                  </span>
                  <span>{refFrame.pts ?? "N/A"}</span>
                </>
              ) : (
                <>
                  <span>-</span>
                  <span>-</span>
                </>
              )}
              <span className="dpb-ref">Reference</span>
            </div>
          );
        })}

        {refFrames.length === 0 && (
          <div className="dpb-row dpb-empty-row">
            <span>No reference frames (keyframe or intra-only)</span>
          </div>
        )}
      </div>

      <div className="dpb-note">
        <span className="codicon codicon-info"></span>
        DPB shows current frame and its reference frames
      </div>
    </div>
  );
});
