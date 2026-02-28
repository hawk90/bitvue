/**
 * Compare Controls - Sync controls for A/B compare view
 *
 * Per parity:
 * - Sync mode toggle (Off/Playhead/Full)
 * - Manual offset adjustment
 * - Alignment info display
 */

import { memo } from "react";
import {
  SyncMode,
  AlignmentMethod,
  AlignmentConfidence,
} from "../../types/video";
import "./CompareControls.css";

interface CompareControlsProps {
  syncMode: SyncMode;
  manualOffset: number;
  onSyncModeChange: (mode: SyncMode) => void;
  onOffsetChange: (delta: number) => void;
  alignmentInfo: {
    method: AlignmentMethod;
    confidence: AlignmentConfidence;
    gapPercentage: number;
  };
}

function CompareControls({
  syncMode,
  manualOffset,
  onSyncModeChange,
  onOffsetChange,
  alignmentInfo,
}: CompareControlsProps) {
  const handleSyncToggle = () => {
    const modes: SyncMode[] = [SyncMode.Off, SyncMode.Playhead, SyncMode.Full];
    const currentIdx = modes.indexOf(syncMode);
    const nextMode = modes[(currentIdx + 1) % modes.length];
    onSyncModeChange(nextMode);
  };

  const getSyncModeLabel = (mode: SyncMode): string => {
    switch (mode) {
      case SyncMode.Off:
        return "Sync: Off";
      case SyncMode.Playhead:
        return "Sync: Playhead";
      case SyncMode.Full:
        return "Sync: Full";
    }
  };

  const getSyncModeIcon = (mode: SyncMode): string => {
    switch (mode) {
      case SyncMode.Off:
        return "unlink";
      case SyncMode.Playhead:
        return "link";
      case SyncMode.Full:
        return "link-2";
    }
  };

  return (
    <div className="compare-controls">
      {/* Sync mode toggle */}
      <button
        className={`sync-toggle ${syncMode.toLowerCase()}`}
        onClick={handleSyncToggle}
        title={`Current: ${getSyncModeLabel(syncMode)}. Click to cycle.`}
      >
        <span className="sync-icon">{getSyncModeIcon(syncMode)}</span>
        <span className="sync-label">{getSyncModeLabel(syncMode)}</span>
      </button>

      {/* Manual offset controls */}
      <div className="offset-controls">
        <span className="offset-label">Offset:</span>
        <button
          className="offset-btn"
          onClick={() => onOffsetChange(-1)}
          title="Decrease offset (B lags A)"
        >
          −
        </button>
        <span className="offset-value">
          {manualOffset > 0 ? `+${manualOffset}` : manualOffset}
        </span>
        <button
          className="offset-btn"
          onClick={() => onOffsetChange(1)}
          title="Increase offset (B leads A)"
        >
          +
        </button>
        <button
          className="offset-reset"
          onClick={() => onOffsetChange(-manualOffset)}
          title="Reset offset to 0"
        >
          Reset
        </button>
      </div>

      {/* Alignment info badge */}
      <div
        className={`alignment-badge confidence-${alignmentInfo.confidence.toLowerCase()}`}
      >
        <span className="alignment-method">{alignmentInfo.method}</span>
        <span className="alignment-confidence">{alignmentInfo.confidence}</span>
        {alignmentInfo.gapPercentage > 0 && (
          <span className="alignment-gaps">
            {alignmentInfo.gapPercentage.toFixed(1)}% gaps
          </span>
        )}
      </div>
    </div>
  );
}

export default memo(CompareControls);
