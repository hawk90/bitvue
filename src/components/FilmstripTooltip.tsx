/**
 * Filmstrip Tooltip Component
 *
 * Displays detailed frame information on hover
 */

import type { FrameInfo } from '../types/video';
import { memo } from 'react';

interface FilmstripTooltipProps {
  frame: FrameInfo;
  x: number;
  y: number;
  placement: 'left' | 'right';
}

export const FilmstripTooltip = memo(function FilmstripTooltip({ frame, x, y, placement }: FilmstripTooltipProps) {
  // Add offset to avoid overlapping with mouse cursor
  const offsetX = placement === 'left' ? -40 : 40;
  const offsetY = -60; // Move tooltip up to avoid cursor

  return (
    <div
      className={`filmstrip-tooltip ${placement === 'left' ? 'tooltip-left' : 'tooltip-right'}`}
      style={{
        left: `${x + offsetX}px`,
        top: `${y + offsetY}px`,
      }}
    >
      <div className="filmstrip-tooltip-header">
        <span className="frame-number">#{frame.frame_index}</span>
        <span className={`frame-type type-${frame.frame_type.toLowerCase()}`}>
          {frame.frame_type}
        </span>
      </div>
      <div className="filmstrip-tooltip-body">
        <span className="filmstrip-tooltip-label">Size</span>
        <span className="filmstrip-tooltip-value">{(frame.size / 1024).toFixed(1)} KB</span>

        {frame.pts !== undefined && (
          <>
            <span className="filmstrip-tooltip-label">PTS</span>
            <span className="filmstrip-tooltip-value">{frame.pts}</span>
          </>
        )}

        {frame.poc !== undefined && (
          <>
            <span className="filmstrip-tooltip-label">POC</span>
            <span className="filmstrip-tooltip-value">{frame.poc}</span>
          </>
        )}

        {frame.temporal_id !== undefined && (
          <>
            <span className="filmstrip-tooltip-label">Temporal</span>
            <span className="filmstrip-tooltip-value">T{frame.temporal_id}</span>
          </>
        )}

        {frame.ref_frames && frame.ref_frames.length > 0 && (
          <>
            <span className="filmstrip-tooltip-label">References</span>
            <span className="filmstrip-tooltip-value highlight">
              [{frame.ref_frames.join(', ')}]
            </span>
          </>
        )}
      </div>
    </div>
  );
});
