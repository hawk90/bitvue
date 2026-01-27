/**
 * Status Bar Component
 *
 * Displays frame info, zoom level, current mode, and playback status
 */

import { memo } from 'react';
import { MODES, type VisualizationMode } from '../../../contexts/ModeContext';

interface StatusBarProps {
  currentFrameIndex: number;
  totalFrames: number;
  currentMode: VisualizationMode;
  zoom: number;
  isPlaying: boolean;
  playbackSpeed: number;
}

export const StatusBar = memo(function StatusBar({
  currentFrameIndex,
  totalFrames,
  currentMode,
  zoom,
  isPlaying,
  playbackSpeed,
}: StatusBarProps) {
  const currentModeData = MODES.find(m => m.key === currentMode);

  // Format playback speed - show decimals for values less than 1
  const formattedSpeed = Number.isInteger(playbackSpeed)
    ? playbackSpeed.toString()
    : playbackSpeed.toFixed(2);

  return (
    <div className="yuv-status-bar">
      <span className="status-section">
        Frame {currentFrameIndex} / {totalFrames}
      </span>
      <span className="status-section">
        Zoom: {Math.round(zoom * 100)}%
      </span>
      <span className="status-section yuv-mode-indicator">
        {currentModeData?.label.toLowerCase() || 'overview'} ({currentModeData?.shortcut})
      </span>
      <span className="status-section">
        {isPlaying ? (
          <span className="yuv-playing-indicator">
            <span className="codicon codicon-loading codicon-spin"></span>
            Playing {formattedSpeed}x
          </span>
        ) : (
          <span className="yuv-paused-indicator">Paused</span>
        )}
      </span>
    </div>
  );
});
