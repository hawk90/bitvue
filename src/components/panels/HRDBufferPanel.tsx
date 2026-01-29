/**
 * HRD (Hypothetical Reference Decoder) Buffer Panel - F9
 *
 * Visualizes CPB (Coded Picture Buffer) fullness over time
 * Shows buffer occupancy, overflow/underflow detection, and target bitrate
 *
 * Per VQAnalyzer parity: HRD buffer plot with occupancy graph
 */

import { useMemo, useRef, useEffect, useState, useCallback, memo } from 'react';
import type { FrameInfo } from '../types/video';
import './HRDBufferPanel.css';

export interface HRDBufferPanelProps {
  frames: FrameInfo[];
  currentFrameIndex: number;
  frameRate: number;
  targetBitrate?: number; // in bits per second
  bufferSize?: number; // in bytes (CPB size)
}

interface HRDState {
  occupancy: number; // Current buffer occupancy in bytes
  occupancyHistory: { frame: number; occupancy: number; overflow: boolean; underflow: boolean }[];
  maxOccupancy: number;
}

const HRDBufferPanelInternal = ({
  frames,
  currentFrameIndex,
  frameRate,
  targetBitrate,
  bufferSize = 1000000, // Default 1MB CPB
}: HRDBufferPanelProps) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [hoveredFrame, setHoveredFrame] = useState<number | null>(null);
  const [hoverData, setHoverData] = useState<{ frame: number; occupancy: number; overflow: boolean; underflow: boolean } | null>(null);

  // Calculate HRD buffer state for each frame
  const hrdState = useMemo(() => {
    const state: HRDState = {
      occupancy: 0,
      occupancyHistory: [],
      maxOccupancy: 0,
    };

    let currentOccupancy = bufferSize * 0.5; // Start half full

    for (let i = 0; i < frames.length; i++) {
      const frame = frames[i];
      if (!frame) continue;

      // Remove frame size from buffer (decoded frame removal)
      const frameSize = frame.size || 0;

      // Check for initial removal delay
      const initialRemovalDelay = frameSize; // Simplified

      // Add current frame to buffer (if it's the frame being decoded)
      // and remove older frames
      currentOccupancy = Math.max(0, currentOccupancy - frameSize);
      currentOccupancy += frameSize;

      // Check overflow
      const overflow = currentOccupancy > bufferSize;
      // Check underflow (buffer less than 10% full)
      const underflow = currentOccupancy < bufferSize * 0.1;

      state.occupancyHistory.push({
        frame: i,
        occupancy: currentOccupancy,
        overflow,
        underflow,
      });

      state.maxOccupancy = Math.max(state.maxOccupancy, currentOccupancy);
    }

    state.occupancy = currentOccupancy;
    return state;
  }, [frames, bufferSize]);

  // Draw HRD buffer graph
  useEffect(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container || hrdState.occupancyHistory.length === 0) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Set canvas size
    const dpr = window.devicePixelRatio || 1;
    const rect = container.getBoundingClientRect();
    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    canvas.style.width = `${rect.width}px`;
    canvas.style.height = `${rect.height}px`;
    ctx.scale(dpr, dpr);

    const width = rect.width;
    const height = rect.height;

    // Clear canvas
    ctx.clearRect(0, 0, width, height);

    // Draw background
    ctx.fillStyle = '#1e1e1e';
    ctx.fillRect(0, 0, width, height);

    // Graph margins
    const margin = { top: 20, right: 10, bottom: 30, left: 50 };
    const graphWidth = width - margin.left - margin.right;
    const graphHeight = height - margin.top - margin.bottom;

    // Draw buffer limit line (100%)
    const maxOccupancy = Math.max(bufferSize, hrdState.maxOccupancy * 1.1);
    ctx.strokeStyle = '#ff6b6b';
    ctx.lineWidth = 1;
    ctx.setLineDash([5, 5]);
    ctx.beginPath();
    ctx.moveTo(margin.left, margin.top + (bufferSize / maxOccupancy) * graphHeight);
    ctx.lineTo(margin.left + graphWidth, margin.top + (bufferSize / maxOccupancy) * graphHeight);
    ctx.stroke();
    ctx.setLineDash([]);

    // Draw underflow warning line (10%)
    ctx.strokeStyle = '#ffd43b';
    ctx.lineWidth = 1;
    ctx.setLineDash([3, 3]);
    ctx.beginPath();
    ctx.moveTo(margin.left, margin.top + ((bufferSize * 0.1) / maxOccupancy) * graphHeight);
    ctx.lineTo(margin.left + graphWidth, margin.top + ((bufferSize * 0.1) / maxOccupancy) * graphHeight);
    ctx.stroke();
    ctx.setLineDash([]);

    // Draw target bitrate line
    if (targetBitrate) {
      const targetBitsPerFrame = targetBitrate / frameRate;
      const targetY = margin.top + ((targetBitsPerFrame * 5) / maxOccupancy) * graphHeight;
      ctx.strokeStyle = '#51cf66';
      ctx.lineWidth = 1;
      ctx.setLineDash([2, 2]);
      ctx.beginPath();
      ctx.moveTo(margin.left, targetY);
      ctx.lineTo(margin.left + graphWidth, targetY);
      ctx.stroke();
      ctx.setLineDash([]);
    }

    // Draw occupancy line
    ctx.strokeStyle = '#339af0';
    ctx.lineWidth = 1.5;
    ctx.beginPath();

    const history = hrdState.occupancyHistory;
    const stepX = graphWidth / Math.max(1, history.length - 1);

    history.forEach((point, i) => {
      const x = margin.left + i * stepX;
      const y = margin.top + graphHeight - (point.occupancy / maxOccupancy) * graphHeight;

      if (i === 0) {
        ctx.moveTo(x, y);
      } else {
        ctx.lineTo(x, y);
      }
    });

    ctx.stroke();

    // Draw overflow/underflow markers
    history.forEach((point, i) => {
      const x = margin.left + i * stepX;
      const y = margin.top + graphHeight - (point.occupancy / maxOccupancy) * graphHeight;

      if (point.overflow) {
        ctx.fillStyle = 'rgba(255, 107, 107, 0.8)';
        ctx.beginPath();
        ctx.arc(x, y, 4, 0, Math.PI * 2);
        ctx.fill();
      }

      if (point.underflow) {
        ctx.fillStyle = 'rgba(255, 212, 59, 0.8)';
        ctx.beginPath();
        ctx.arc(x, y, 4, 0, Math.PI * 2);
        ctx.fill();
      }
    });

    // Draw current frame marker
    const currentX = margin.left + currentFrameIndex * stepX;
    const currentY = margin.top + graphHeight - (hrdState.occupancy / maxOccupancy) * graphHeight;

    ctx.strokeStyle = '#ffffff';
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(currentX, margin.top);
    ctx.lineTo(currentX, margin.top + graphHeight);
    ctx.stroke();

    // Draw Y-axis labels
    ctx.fillStyle = '#888';
    ctx.font = '10px monospace';
    ctx.textAlign = 'right';
    ctx.textBaseline = 'middle';

    const yLabels = [0, 25, 50, 75, 100];
    yLabels.forEach(percent => {
      const y = margin.top + graphHeight - (percent / 100) * graphHeight;
      const value = Math.round((bufferSize * percent / 100) / 1024);
      ctx.fillText(`${value}KB`, margin.left - 8, y);
    });

    // Draw X-axis labels (frames)
    ctx.textAlign = 'center';
    ctx.textBaseline = 'top';

    const xSteps = 5;
    const xStep = Math.ceil(history.length / xSteps);
    for (let i = 0; i < history.length; i += xStep) {
      const x = margin.left + i * stepX;
      ctx.fillText(`${history[i].frame}`, x, margin.top + graphHeight + 8);
    }

  }, [hrdState, currentFrameIndex, bufferSize, targetBitrate, frameRate]);

  // Handle mouse move for tooltip
  const handleMouseMove = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;

    const margin = { top: 20, right: 10, bottom: 30, left: 50 };
    const graphWidth = rect.width - margin.left - margin.right;

    if (x < margin.left || x > margin.left + graphWidth) {
      setHoveredFrame(null);
      setHoverData(null);
      return;
    }

    const history = hrdState.occupancyHistory;
    const stepX = graphWidth / Math.max(1, history.length - 1);
    const frameIndex = Math.round((x - margin.left) / stepX);

    if (frameIndex >= 0 && frameIndex < history.length) {
      setHoveredFrame(frameIndex);
      setHoverData(history[frameIndex]);
    }
  }, [hrdState]);

  const overflowCount = hrdState.occupancyHistory.filter(p => p.overflow).length;
  const underflowCount = hrdState.occupancyHistory.filter(p => p.underflow).length;
  const currentPercent = (hrdState.occupancy / bufferSize) * 100;

  return (
    <div className="hrd-buffer-panel">
      <div className="hrd-header">
        <h3>HRD Buffer</h3>
        <div className="hrd-stats">
          <span className="hrd-stat">
            <span className="stat-label">Buffer:</span>
            <span className="stat-value">{currentPercent.toFixed(1)}%</span>
          </span>
          {targetBitrate && (
            <span className="hrd-stat">
              <span className="stat-label">Target:</span>
              <span className="stat-value">{(targetBitrate / 1000000).toFixed(2)} Mbps</span>
            </span>
          )}
          {overflowCount > 0 && (
            <span className="hrd-stat overflow">
              <span className="stat-label">Overflow:</span>
              <span className="stat-value">{overflowCount}</span>
            </span>
          )}
          {underflowCount > 0 && (
            <span className="hrd-stat underflow">
              <span className="stat-label">Underflow:</span>
              <span className="stat-value">{underflowCount}</span>
            </span>
          )}
        </div>
      </div>

      <div ref={containerRef} className="hrd-canvas-container">
        <canvas
          ref={canvasRef}
          onMouseMove={handleMouseMove}
          onMouseLeave={() => {
            setHoveredFrame(null);
            setHoverData(null);
          }}
        />
      </div>

      {hoverData && (
        <div className="hrd-tooltip">
          <div className="tooltip-row">
            <span className="tooltip-label">Frame:</span>
            <span className="tooltip-value">{hoverData.frame}</span>
          </div>
          <div className="tooltip-row">
            <span className="tooltip-label">Occupancy:</span>
            <span className="tooltip-value">
              {(hoverData.occupancy / 1024).toFixed(1)} KB ({((hoverData.occupancy / bufferSize) * 100).toFixed(1)}%)
            </span>
          </div>
          {hoverData.overflow && (
            <div className="tooltip-row overflow">
              <span className="tooltip-label">⚠ Overflow</span>
            </div>
          )}
          {hoverData.underflow && (
            <div className="tooltip-row underflow">
              <span className="tooltip-label">⚠ Underflow</span>
            </div>
          )}
        </div>
      )}

      <div className="hrd-legend">
        <div className="legend-item">
          <span className="legend-color line-buffer"></span>
          <span className="legend-label">Buffer Limit</span>
        </div>
        <div className="legend-item">
          <span className="legend-color line-target"></span>
          <span className="legend-label">Target</span>
        </div>
        <div className="legend-item">
          <span className="legend-color dot-overflow"></span>
          <span className="legend-label">Overflow</span>
        </div>
        <div className="legend-item">
          <span className="legend-color dot-underflow"></span>
          <span className="legend-label">Underflow</span>
        </div>
      </div>
    </div>
  );
};

export const HRDBufferPanel = memo(HRDBufferPanelInternal);
export default HRDBufferPanel;
