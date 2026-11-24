/**
 * Status Bar Component
 *
 * Displays frame info, zoom level, current mode, and playback status
 */

import { memo } from "react";
import type { VisualizationMode } from "../../../contexts/ModeContext";
import type { CodecModeEntry } from "../../../utils/codecModeRegistry";
import type { AnalysisProvenance } from "../../../types/video";

interface StatusBarProps {
  currentFrameIndex: number;
  totalFrames: number;
  currentMode: VisualizationMode;
  zoom: number;
  isPlaying: boolean;
  playbackSpeed: number;
  /** Injected from YuvViewerPanel — same codec-aware list ModeSelector uses, so the label/
   *  shortcut shown here always matches what's actually selected. The old `MODES` import (a
   *  single hardcoded, non-per-codec list from before `codecModeRegistry.ts` existed) fell out
   *  of sync with per-codec F-key assignments -- e.g. AV1's F6 (cdef-filter) and F9 (film-grain)
   *  have no entry in that list at all, silently showing "overview ()" instead. */
  availableModes: CodecModeEntry[];
  /** How far the current frame's analysis (overlays, grids) can be trusted; omitted until it has
   *  loaded or for codecs that do not report it. */
  analysisProvenance?: AnalysisProvenance;
}

/** What the status bar says about a frame whose analysis is not fully verified (#549). */
export const ANALYSIS_NOTICES: Record<
  Exclude<AnalysisProvenance, "verified">,
  { label: string; detail: string }
> = {
  unverified: {
    label: "Analysis unverified",
    detail:
      "Decoding of this frame stopped early or did not end where expected, so its overlays and block data may be incomplete or wrong.",
  },
  scaffold: {
    label: "No analysis data",
    detail:
      "This frame could not be decoded, so there are no overlays or block data for it.",
  },
};

export const StatusBar = memo(function StatusBar({
  currentFrameIndex,
  totalFrames,
  currentMode,
  zoom,
  isPlaying,
  playbackSpeed,
  availableModes,
  analysisProvenance,
}: StatusBarProps) {
  const notice =
    analysisProvenance && analysisProvenance !== "verified"
      ? ANALYSIS_NOTICES[analysisProvenance]
      : null;
  const currentModeData = availableModes.find((m) => m.mode === currentMode);

  // Format playback speed - show decimals for values less than 1
  const formattedSpeed = Number.isInteger(playbackSpeed)
    ? playbackSpeed.toString()
    : playbackSpeed.toFixed(2);

  return (
    <div className="yuv-status-bar">
      <span className="status-section">
        Frame {currentFrameIndex + 1} / {totalFrames}
      </span>
      <span className="status-section">Zoom: {Math.round(zoom * 100)}%</span>
      <span className="status-section yuv-mode-indicator">
        {currentModeData?.label.toLowerCase() || "overview"} (
        {currentModeData?.fKey != null ? `F${currentModeData.fKey}` : ""})
      </span>
      {notice && (
        <span
          className="status-section yuv-analysis-notice"
          title={notice.detail}
          role="status"
        >
          <span className="codicon codicon-warning"></span>
          {notice.label}
        </span>
      )}
      <span className="status-section">
        {isPlaying ? (
          <span className="yuv-playing-indicator">
            <span className="codicon codicon-play"></span>
            Playing {formattedSpeed}x
          </span>
        ) : (
          <span className="yuv-paused-indicator">Paused</span>
        )}
      </span>
    </div>
  );
});
