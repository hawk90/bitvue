/**
 * Status Bar Component
 *
 * Displays frame info, zoom level, current mode, and playback status
 */

import { memo } from "react";
import type { VisualizationMode } from "../../../contexts/ModeContext";
import type { CodecModeEntry } from "../../../utils/codecModeRegistry";

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
}

export const StatusBar = memo(function StatusBar({
  currentFrameIndex,
  totalFrames,
  currentMode,
  zoom,
  isPlaying,
  playbackSpeed,
  availableModes,
}: StatusBarProps) {
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
