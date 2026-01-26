/**
 * Statistics Tab Component
 *
 * Displays frame and stream statistics
 * Shows current frame info, stream statistics, and frame type distribution
 */

import { memo } from 'react';
import React from 'react';

interface StatisticsTabProps {
  currentFrame: {
    frame_index: number;
    frame_type: string;
    size: number;
    temporal_id?: number;
    display_order?: number;
    coding_order?: number;
    ref_frames?: number[];
  } | null;
  frames: Array<{
    frame_index: number;
    frame_type: string;
    size: number;
  }>;
}

export const StatisticsTab = memo(function StatisticsTab({
  currentFrame,
  frames,
}: StatisticsTabProps) {
  const totalFrames = frames.length;
  const frameTypes = frames.reduce((acc, frame) => {
    acc[frame.frame_type] = (acc[frame.frame_type] || 0) + 1;
    return acc;
  }, {} as Record<string, number>);

  const avgSize = totalFrames > 0
    ? frames.reduce((sum, f) => sum + f.size, 0) / totalFrames
    : 0;

  return (
    <div className="syntax-tab-content">
      <div className="stats-section">
        <div className="stats-header">Current Frame</div>
        <div className="stats-grid">
          <span className="stats-label">Index:</span>
          <span className="stats-value">{currentFrame?.frame_index ?? 'N/A'}</span>
          <span className="stats-label">Type:</span>
          <span className="stats-value">{currentFrame?.frame_type ?? 'N/A'}</span>
          <span className="stats-label">Size:</span>
          <span className="stats-value">
            {currentFrame ? `${(currentFrame.size / 1024).toFixed(2)} KB` : 'N/A'}
          </span>
        </div>
      </div>

      <div className="stats-section">
        <div className="stats-header">Stream Statistics</div>
        <div className="stats-grid">
          <span className="stats-label">Total Frames:</span>
          <span className="stats-value">{totalFrames}</span>
          <span className="stats-label">Avg Size:</span>
          <span className="stats-value">{(avgSize / 1024).toFixed(2)} KB</span>
          {Object.entries(frameTypes).map(([type, count]) => (
            <React.Fragment key={type}>
              <span className="stats-label">{type} Frames:</span>
              <span className="stats-value">{count}</span>
            </React.Fragment>
          ))}
        </div>
      </div>

      {currentFrame && (
        <div className="stats-section">
          <div className="stats-header">Frame Info</div>
          <div className="stats-grid">
            {currentFrame.temporal_id !== undefined && (
              <>
                <span className="stats-label">Temporal ID:</span>
                <span className="stats-value">{currentFrame.temporal_id}</span>
              </>
            )}
            {currentFrame.display_order !== undefined && (
              <>
                <span className="stats-label">Display Order:</span>
                <span className="stats-value">{currentFrame.display_order}</span>
              </>
            )}
            {currentFrame.coding_order !== undefined && (
              <>
                <span className="stats-label">Coding Order:</span>
                <span className="stats-value">{currentFrame.coding_order}</span>
              </>
            )}
            <span className="stats-label">References:</span>
            <span className="stats-value">{currentFrame.ref_frames?.length || 0}</span>
          </div>
        </div>
      )}
    </div>
  );
});
