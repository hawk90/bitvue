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
});

export default DetailsPanel;
