/**
 * Frame View Tab Component
 *
 * Displays detailed frame information
 */

import { memo } from 'react';

interface FrameViewTabProps {
  frame: {
    frame_index: number;
    frame_type: string;
    size: number;
    pts?: number;
    temporal_id?: number;
    display_order?: number;
    coding_order?: number;
    ref_frames?: number[];
  } | null;
}

export const FrameViewTab = memo(function FrameViewTab({ frame }: FrameViewTabProps) {
  if (!frame) {
    return (
      <div className="hex-empty">
        <span className="codicon codicon-file"></span>
        <span>No frame selected</span>
      </div>
    );
  }

  return (
    <div className="frame-info-content">
      <div className="frame-info-header">
        <span className="frame-info-type-badge">{frame.frame_type}</span>
        <span className="frame-info-title">Frame {frame.frame_index}</span>
      </div>

      <div className="frame-info-grid">
        <div className="frame-info-row">
          <span className="frame-info-label">Frame Index:</span>
          <span className="frame-info-value">{frame.frame_index}</span>
        </div>
        <div className="frame-info-row">
          <span className="frame-info-label">Frame Type:</span>
          <span className={`frame-info-value frame-type-${frame.frame_type.toLowerCase()}`}>
            {frame.frame_type}
          </span>
        </div>
        <div className="frame-info-row">
          <span className="frame-info-label">Size:</span>
          <span className="frame-info-value">
            {(frame.size / 1024).toFixed(2)} KB ({frame.size} bytes)
          </span>
        </div>
        {frame.pts !== undefined && (
          <div className="frame-info-row">
            <span className="frame-info-label">PTS:</span>
            <span className="frame-info-value">{frame.pts}</span>
          </div>
        )}
        {frame.temporal_id !== undefined && (
          <div className="frame-info-row">
            <span className="frame-info-label">Temporal ID:</span>
            <span className="frame-info-value">{frame.temporal_id}</span>
          </div>
        )}
        {frame.display_order !== undefined && (
          <div className="frame-info-row">
            <span className="frame-info-label">Display Order:</span>
            <span className="frame-info-value">{frame.display_order}</span>
          </div>
        )}
        {frame.coding_order !== undefined && (
          <div className="frame-info-row">
            <span className="frame-info-label">Coding Order:</span>
            <span className="frame-info-value">{frame.coding_order}</span>
          </div>
        )}
        <div className="frame-info-row">
          <span className="frame-info-label">References:</span>
          <span className="frame-info-value">
            {frame.ref_frames?.length || 0} frame{(frame.ref_frames?.length || 0) !== 1 ? 's' : ''}
          </span>
        </div>
        {frame.ref_frames && frame.ref_frames.length > 0 && (
          <div className="frame-info-row">
            <span className="frame-info-label">Ref Frames:</span>
            <span className="frame-info-value">
              {frame.ref_frames.join(', ')}
            </span>
          </div>
        )}
      </div>

      <div className="frame-info-note">
        <span className="codicon codicon-info"></span>
        Full bitstream data available after parser integration
      </div>
    </div>
  );
});
