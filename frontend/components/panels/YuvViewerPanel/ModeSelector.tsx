/**
 * Mode Selector Component
 *
 * Dropdown for selecting the visualization mode.
 * Modes are codec-aware: the list changes based on which codec is loaded.
 */

import { memo } from "react";
import type { VisualizationMode } from "../../../contexts/ModeContext";
import type { CodecModeEntry } from "../../../utils/codecModeRegistry";

interface ModeSelectorProps {
  currentMode: VisualizationMode;
  onModeChange: (mode: VisualizationMode) => void;
  /** Injected from YuvViewerPanel — avoids a redundant context read. */
  availableModes: CodecModeEntry[];
}

export const ModeSelector = memo(function ModeSelector({
  currentMode,
  onModeChange,
  availableModes,
}: ModeSelectorProps) {
  // Ensure the displayed value is always valid for the current list
  const safeValue = availableModes.some((m) => m.mode === currentMode)
    ? currentMode
    : (availableModes[0]?.mode ?? "overview");

  return (
    <div className="yuv-toolbar-group">
      <span className="yuv-mode-label">Mode:</span>
      <select
        value={safeValue}
        onChange={(e) => onModeChange(e.target.value as VisualizationMode)}
        className="yuv-mode-select"
        title="Visualization Mode"
        aria-label="Visualization mode"
      >
        {availableModes.map((entry) => (
          <option key={entry.mode} value={entry.mode}>
            {entry.fKey != null ? `F${entry.fKey}` : "  "} — {entry.label}
          </option>
        ))}
      </select>
    </div>
  );
});

/**
 * Codec badge shown next to the mode selector when a file is loaded.
 */
export const CodecBadge = memo(function CodecBadge({
  codec,
}: {
  codec: string | null;
}) {
  if (!codec) return null;
  return (
    <div className="yuv-toolbar-group">
      <span className="yuv-codec-badge" title={`Active codec: ${codec}`}>
        {codec}
      </span>
    </div>
  );
});
