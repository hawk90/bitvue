/**
 * Mode Selector Component
 *
 * Dropdown for selecting visualization mode (F1-F7)
 */

import { memo } from "react";
import { MODES, type VisualizationMode } from "../../../contexts/ModeContext";

interface ModeSelectorProps {
  currentMode: VisualizationMode;
  onModeChange: (mode: VisualizationMode) => void;
}

export const ModeSelector = memo(function ModeSelector({
  currentMode,
  onModeChange,
}: ModeSelectorProps) {
  return (
    <div className="yuv-toolbar-group">
      <span className="yuv-mode-label">Mode:</span>
      <select
        value={currentMode}
        onChange={(e) => {
          const val = e.target.value;
          if (MODES.some((m) => m.key === val))
            onModeChange(val as VisualizationMode);
        }}
        className="yuv-mode-select"
        title="Visualization Mode"
        aria-label="Visualization mode"
      >
        {MODES.map((mode) => (
          <option key={mode.key} value={mode.key}>
            {mode.shortcut} - {mode.label}
          </option>
        ))}
      </select>
    </div>
  );
});
