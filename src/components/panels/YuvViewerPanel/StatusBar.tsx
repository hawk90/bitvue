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

  return (
    <div className="yuv-status">
      <span>Frame {currentFrameIndex} / {totalFrames - 1}</span>
      <span>Zoom: {Math.round(zoom * 100)}%</span>
      <span className="yuv-mode-indicator">
        {currentModeData?.label || 'Overview'} ({currentModeData?.shortcut})
      </span>
      {isPlaying && (
        <span className="yuv-playing-indicator">
          <span className="codicon codicon-loading codicon-spin"></span>
          Playing {playbackSpeed}x
        </span>
      )}
    </div>
  );
});
