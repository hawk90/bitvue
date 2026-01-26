/**
 * Diff Overlay - Difference visualization for A/B compare
 *
 * Shows pixel-wise difference, PSNR map, or SSIM map.
 */

import { memo, useMemo } from 'react';
import { type FrameInfo } from '../../types/video';
import './DiffOverlay.css';

interface DiffOverlayProps {
  frameA: FrameInfo;
  frameB: FrameInfo;
  mode: 'difference' | 'psnr' | 'ssim';
}

function DiffOverlay({ frameA, frameB, mode }: DiffOverlayProps) {
  // Calculate difference visualization
  const diffCanvas = useMemo(() => {
    if (!frameA.thumbnail || !frameB.thumbnail) return null;

    // Create canvas for diff visualization
    const canvas = document.createElement('canvas');
    const ctx = canvas.getContext('2d');
    if (!ctx) return null;

    const imgA = new Image();
    const imgB = new Image();

    // This would need actual image data
    // For now, it's a placeholder for the diff visualization

    return canvas;
  }, [frameA, frameB, mode]);

  if (mode === 'difference') {
    return (
      <div className="diff-overlay">
        <div className="diff-header">
          <span>Difference Map</span>
          <span className="diff-legend">
            <span className="legend-item">
              <span className="legend-color diff-none"></span> None
            </span>
            <span className="legend-item">
              <span className="legend-color diff-low"></span> Low
            </span>
            <span className="legend-item">
              <span className="legend-color diff-high"></span> High
            </span>
          </span>
        </div>
        <div className="diff-canvas">
          {/* Placeholder for diff visualization */}
          <div className="diff-placeholder">
            Difference visualization would be rendered here
          </div>
        </div>
      </div>
    );
  }

  if (mode === 'psnr') {
    return (
      <div className="diff-overlay">
        <div className="diff-header">
          <span>PSNR Map</span>
          <span className="diff-legend">
            <span className="legend-item">
              <span className="legend-color psnr-high"></span> High (&gt;40dB)
            </span>
            <span className="legend-item">
              <span className="legend-color psnr-med"></span> Med (30-40dB)
            </span>
            <span className="legend-item">
              <span className="legend-color psnr-low"></span> Low (&lt;30dB)
            </span>
          </span>
        </div>
        <div className="diff-canvas">
          <div className="diff-placeholder">
            PSNR map visualization would be rendered here
          </div>
        </div>
      </div>
    );
  }

  // SSIM mode
  return (
    <div className="diff-overlay">
      <div className="diff-header">
        <span>SSIM Map</span>
        <span className="diff-legend">
          <span className="legend-item">
            <span className="legend-color ssim-high"></span> High (&gt;0.95)
          </span>
          <span className="legend-item">
            <span className="legend-color ssim-med"></span> Med (0.85-0.95)
          </span>
          <span className="legend-item">
            <span className="legend-color ssim-low"></span> Low (&lt;0.85)
          </span>
        </span>
      </div>
      <div className="diff-canvas">
        <div className="diff-placeholder">
          SSIM map visualization would be rendered here
        </div>
      </div>
    </div>
  );
}

export default memo(DiffOverlay);
