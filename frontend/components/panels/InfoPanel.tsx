/**
 * Info Panel Component
 *
 * Displays basic file and frame information in the bottom panel area
 */

import { memo } from 'react';
import type { FrameInfo } from '../../types/video';
import './InfoPanel.css';

interface InfoPanelProps {
  /** Current file path */
  filePath?: string;
  /** Total number of frames */
  frameCount: number;
  /** Current frame index */
  currentFrameIndex: number;
  /** Current frame data */
  currentFrame: FrameInfo | null;
}

/**
 * Custom comparison for InfoPanel props
 * Performs deep comparison for currentFrame object to prevent unnecessary re-renders
 */
function arePropsEqual(prevProps: InfoPanelProps, nextProps: InfoPanelProps): boolean {
  return (
    prevProps.filePath === nextProps.filePath &&
    prevProps.frameCount === nextProps.frameCount &&
    prevProps.currentFrameIndex === nextProps.currentFrameIndex &&
    prevProps.currentFrame === nextProps.currentFrame // Reference equality for frame object
  );
}

export const InfoPanel = memo(function InfoPanel({
  filePath,
  frameCount,
  currentFrameIndex,
  currentFrame,
}: InfoPanelProps) {
  return (
    <div className="bottom-panel-content">
      <div className="info-grid">
        <span className="info-label">File:</span>
        <span className="info-value">{filePath || 'N/A'}</span>

        <span className="info-label">Frames:</span>
        <span className="info-value">{frameCount}</span>

        <span className="info-label">Current Frame:</span>
        <span className="info-value">{currentFrameIndex}</span>

        <span className="info-label">Frame Type:</span>
        <span className="info-value">{currentFrame?.frame_type || 'N/A'}</span>

        <span className="info-label">Size:</span>
        <span className="info-value">
          {currentFrame?.size ? `${(currentFrame.size / 1024).toFixed(2)} KB` : 'N/A'}
        </span>
      </div>
    </div>
  );
}, arePropsEqual);

export default InfoPanel;
