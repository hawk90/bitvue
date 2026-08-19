/**
 * References Tab Component
 *
 * Displays reference frame information for the current frame
 * Shows list of all reference frames with their types and PTS values
 */

import { memo } from "react";
import { FrameTypeBadge } from "../../common/FrameTypeBadge";

interface ReferencesTabProps {
  currentFrame: {
    frame_index: number;
    frame_type: string;
    ref_frames?: number[];
  } | null;
  frames: Array<{
    frame_index: number;
    frame_type: string;
    pts?: number;
  }>;
}

export const ReferencesTab = memo(function ReferencesTab({
  currentFrame,
  frames,
}: ReferencesTabProps) {
  if (!currentFrame) {
    return (
      <div className="syntax-empty">
        <span className="codicon codicon-database"></span>
        <span>No frame selected</span>
      </div>
    );
  }

  const refFrames = currentFrame.ref_frames || [];

  return (
    <div className="syntax-tab-content">
      <div className="syntax-info">
        <span className="syntax-info-label">Reference Frames:</span>
        <span className="syntax-info-value">{refFrames.length}</span>
      </div>
      <div className="panel-divider"></div>

      {refFrames.length === 0 ? (
        <div className="syntax-empty">
          <span className="codicon codicon-circle-slash"></span>
          <span>No reference frames (keyframe or intra-only)</span>
        </div>
      ) : (
        <div className="refs-list">
          {refFrames.map((refIdx, i) => {
            const refFrame = frames[refIdx];
            return (
              <div key={i} className="ref-item">
                <span className="ref-index">[{i}]</span>
                <span className="ref-frame">Frame {refIdx}</span>
                {refFrame && (
                  <>
                    <FrameTypeBadge frameType={refFrame.frame_type} />
                    <span className="ref-pts">
                      PTS: {refFrame.pts ?? "N/A"}
                    </span>
                  </>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
});
