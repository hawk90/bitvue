/**
 * Details Panel Component
 *
 * Displays detailed frame information in the bottom panel area
 */

import { memo } from 'react';
import './DetailsPanel.css';

export interface FrameDetails {
  temporal_id?: number;
  display_order?: number;
  coding_order?: number;
  ref_frames?: number[];
}

interface DetailsPanelProps {
  /** Current frame detailed information */
  frame: FrameDetails | null;
}

/**
 * Custom comparison for DetailsPanel props
 * Performs deep comparison for frame object to prevent unnecessary re-renders
 */
function arePropsEqual(prevProps: DetailsPanelProps, nextProps: DetailsPanelProps): boolean {
  // Quick null checks
  if (prevProps.frame === nextProps.frame) return true;
  if (!prevProps.frame || !nextProps.frame) return false;

  // Compare individual properties
  return (
    prevProps.frame.temporal_id === nextProps.frame.temporal_id &&
    prevProps.frame.display_order === nextProps.frame.display_order &&
    prevProps.frame.coding_order === nextProps.frame.coding_order &&
    // Deep compare ref_frames arrays
    (prevProps.frame.ref_frames?.length === nextProps.frame.ref_frames?.length) &&
    (prevProps.frame.ref_frames === nextProps.frame.ref_frames ||
      JSON.stringify(prevProps.frame.ref_frames) === JSON.stringify(nextProps.frame.ref_frames))
  );
}

export const DetailsPanel = memo(function DetailsPanel({ frame }: DetailsPanelProps) {
  return (
    <div className="bottom-panel-content">
      <div className="details-grid">
        <span className="details-label">Temporal Layer:</span>
        <span className="details-value">{frame?.temporal_id ?? 'A'}</span>

        <span className="details-label">Display Order:</span>
        <span className="details-value">{frame?.display_order ?? 'N/A'}</span>

        <span className="details-label">Coding Order:</span>
        <span className="details-value">{frame?.coding_order ?? 'N/A'}</span>

        <span className="details-label">References:</span>
        <span className="details-value">
          {frame?.ref_frames?.length ? frame.ref_frames.join(', ') : 'None'}
        </span>
      </div>
    </div>
  );
}, arePropsEqual);

export default DetailsPanel;
