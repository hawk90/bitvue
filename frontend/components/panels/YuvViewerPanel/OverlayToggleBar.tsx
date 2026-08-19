/**
 * Overlay Toggle Bar
 *
 * Renders a row of toggle buttons — one per available info overlay for the
 * currently loaded codec.  Buttons are hidden when no codec is loaded.
 *
 * VQ Analyzer parity:
 *   Mode menu → Info Overlays sub-menu items map to these toggles.
 *   Multiple overlays can be active simultaneously.
 */

import { memo } from "react";
import type {
  CodecModeEntry,
  VisualizationMode,
} from "../../../utils/codecModeRegistry";

interface OverlayToggleBarProps {
  availableOverlays: CodecModeEntry[];
  activeOverlays: ReadonlySet<VisualizationMode>;
  onToggle: (mode: VisualizationMode) => void;
}

/** Short abbreviations shown on narrow buttons */
const OVERLAY_ABBREV: Partial<Record<VisualizationMode, string>> = {
  "qp-map": "QP",
  "heat-map": "HM",
  "mv-field": "MV",
  "psnr-overlay": "PSNR",
  "ssim-overlay": "SSIM",
  "block-type": "BT",
  "pu-type": "PU",
  "mb-type": "MB",
  "reference-indices": "RI",
  "efficiency-map": "EFF",
  "inter-memory": "IMR",
  "simple-motion": "SM",
};

export const OverlayToggleBar = memo(function OverlayToggleBar({
  availableOverlays,
  activeOverlays,
  onToggle,
}: OverlayToggleBarProps) {
  if (availableOverlays.length === 0) return null;

  return (
    <div
      className="yuv-toolbar-group yuv-overlay-toggles"
      role="group"
      aria-label="Info overlays"
    >
      <span className="yuv-overlay-label">Overlay:</span>
      {availableOverlays.map((entry) => {
        const isActive = activeOverlays.has(entry.mode);
        const abbrev = OVERLAY_ABBREV[entry.mode] ?? entry.label;
        return (
          <button
            key={entry.mode}
            className={`yuv-overlay-btn${isActive ? " yuv-overlay-btn--active" : ""}`}
            title={entry.description}
            aria-pressed={isActive}
            onClick={() => onToggle(entry.mode)}
          >
            {abbrev}
          </button>
        );
      })}
    </div>
  );
});
