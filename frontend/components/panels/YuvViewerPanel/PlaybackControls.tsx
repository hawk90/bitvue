/**
 * Playback Controls Component
 *
 * Play/pause button and playback speed selector
 */

import { memo } from 'react';

interface PlaybackControlsProps {
  isPlaying: boolean;
  playbackSpeed: number;
  onTogglePlay: () => void;
  onSpeedChange: (speed: number) => void;
}

export const PlaybackControls = memo(function PlaybackControls({
  isPlaying,
  playbackSpeed,
  onTogglePlay,
  onSpeedChange,
}: PlaybackControlsProps) {
  return (
    <div className="yuv-toolbar-group">
      <button
        onClick={onTogglePlay}
        title={isPlaying ? 'Pause (Space)' : 'Play (Space)'}
        className={isPlaying ? 'active' : ''}
      >
        <span className={`codicon codicon-${isPlaying ? 'debug-pause' : 'play'}`}></span>
      </button>
      <select
        value={playbackSpeed}
        onChange={(e) => onSpeedChange(parseFloat(e.target.value))}
        className="yuv-speed-select"
        title="Playback Speed"
      >
        <option value={0.25}>0.25x</option>
        <option value={0.5}>0.5x</option>
        <option value={1}>1x</option>
        <option value={2}>2x</option>
        <option value={4}>4x</option>
      </select>
    </div>
  );
});
