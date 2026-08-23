/**
 * HRD (Hypothetical Reference Decoder) Buffer Panel - F9
 *
 * Visualizes CPB (Coded Picture Buffer) fullness over time
 * Shows buffer occupancy, overflow/underflow detection, and target bitrate
 *
 * Per parity: HRD buffer plot with occupancy graph
 */

import { useMemo, useRef, useEffect, useState, useCallback, memo } from "react";
import type { FrameInfo } from "../../types/video";
import "./HRDBufferPanel.css";

export interface HRDBufferPanelProps {
  frames: FrameInfo[];
  currentFrameIndex: number;
  frameRate: number;
  targetBitrate?: number; // in bits per second
  bufferSize?: number; // in bytes (CPB size)
}

interface HRDState {
  occupancy: number; // Current buffer occupancy in bytes
  occupancyHistory: {
    frame: number;
    occupancy: number;
    overflow: boolean;
    underflow: boolean;
  }[];
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
  const [hoverData, setHoverData] = useState<{
    frame: number;
    occupancy: number;
    overflow: boolean;
    underflow: boolean;
  } | null>(null);

  // No caller currently passes a real signaled `targetBitrate` -- AV1's decoder_model_info /
  // operating_parameters_info (the actual HRD bitrate/CPB-size syntax) isn't parsed anywhere in
  // this codebase yet (a real gap, tracked separately, comparable in size to DIV-06's HRD/HDR
  // scoping). Falling back to "drain exactly one frame's own bytes per frame" made the whole
  // curve a mathematical no-op -- `occupancy - frameSize + frameSize` is always the starting
  // value, so the buffer line was *always* perfectly flat regardless of stream content (found via
  // a real Electron screenshot: a genuinely constant 50.0%/488KB line). An honest average-bitrate
  // estimate (real per-frame sizes already loaded, no new backend work) at least reflects actual
  // bursty/quiet frame-size variance; the UI labels it "(est.)" rather than "Target" so it isn't
  // mistaken for a real signaled HRD parameter.
  const isEstimatedBitrate = targetBitrate === undefined;
  const effectiveTargetBitrate = useMemo(() => {
    if (targetBitrate !== undefined) return targetBitrate;
    if (frames.length === 0) return undefined;
    const totalBytes = frames.reduce((sum, f) => sum + (f?.size || 0), 0);
    return (totalBytes / frames.length) * frameRate * 8;
  }, [targetBitrate, frames, frameRate]);

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

      const frameSize = frame.size || 0;

      // Decoder drains at (real or estimated) target bitrate, then the frame's own bytes are
      // added -- a frame bigger than the average causes real net buildup, a smaller one real
      // net drain, unlike the old same-as-frameSize fallback (see doc above).
      const drainPerFrame = effectiveTargetBitrate
        ? effectiveTargetBitrate / frameRate / 8 // bytes drained per frame interval
        : frameSize; // only reachable with zero frames loaded
      currentOccupancy = Math.max(0, currentOccupancy - drainPerFrame);
      currentOccupancy = Math.min(bufferSize, currentOccupancy + frameSize);

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
  }, [frames, bufferSize, effectiveTargetBitrate, frameRate]);

  // Draw HRD buffer graph — wrapped in useCallback so the ResizeObserver can
  // call it directly without re-subscribing on every render.
  const drawCanvas = useCallback(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container || hrdState.occupancyHistory.length === 0) return;

    const ctx = canvas.getContext("2d");
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
    ctx.fillStyle = "#1e1e1e";
    ctx.fillRect(0, 0, width, height);

    // Graph margins
    const margin = { top: 20, right: 10, bottom: 30, left: 50 };
    const graphWidth = width - margin.left - margin.right;
    const graphHeight = height - margin.top - margin.bottom;

    // Draw buffer limit line (100%)
    const maxOccupancy = Math.max(bufferSize, hrdState.maxOccupancy * 1.1);
    ctx.strokeStyle = "#ff6b6b";
    ctx.lineWidth = 1;
    ctx.setLineDash([5, 5]);
    ctx.beginPath();
    ctx.moveTo(
      margin.left,
      margin.top + (bufferSize / maxOccupancy) * graphHeight,
    );
    ctx.lineTo(
      margin.left + graphWidth,
      margin.top + (bufferSize / maxOccupancy) * graphHeight,
    );
    ctx.stroke();
    ctx.setLineDash([]);

    // Draw underflow warning line (10%)
    ctx.strokeStyle = "#ffd43b";
    ctx.lineWidth = 1;
    ctx.setLineDash([3, 3]);
    ctx.beginPath();
    ctx.moveTo(
      margin.left,
      margin.top + ((bufferSize * 0.1) / maxOccupancy) * graphHeight,
    );
    ctx.lineTo(
      margin.left + graphWidth,
      margin.top + ((bufferSize * 0.1) / maxOccupancy) * graphHeight,
    );
    ctx.stroke();
    ctx.setLineDash([]);

    // Draw target/estimated bitrate line
    if (effectiveTargetBitrate) {
      const targetBitsPerFrame = effectiveTargetBitrate / frameRate;
      const targetY =
        margin.top + ((targetBitsPerFrame * 5) / maxOccupancy) * graphHeight;
      ctx.strokeStyle = "#51cf66";
      ctx.lineWidth = 1;
      ctx.setLineDash([2, 2]);
      ctx.beginPath();
      ctx.moveTo(margin.left, targetY);
      ctx.lineTo(margin.left + graphWidth, targetY);
      ctx.stroke();
      ctx.setLineDash([]);
    }

    // Draw occupancy line
    ctx.strokeStyle = "#339af0";
    ctx.lineWidth = 1.5;
    ctx.beginPath();

    const history = hrdState.occupancyHistory;
    const stepX = graphWidth / Math.max(1, history.length - 1);

    history.forEach((point, i) => {
      const x = margin.left + i * stepX;
      const y =
        margin.top +
        graphHeight -
        (point.occupancy / maxOccupancy) * graphHeight;

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
      const y =
        margin.top +
        graphHeight -
        (point.occupancy / maxOccupancy) * graphHeight;

      if (point.overflow) {
        ctx.fillStyle = "rgba(255, 107, 107, 0.8)";
        ctx.beginPath();
        ctx.arc(x, y, 4, 0, Math.PI * 2);
        ctx.fill();
      }

      if (point.underflow) {
        ctx.fillStyle = "rgba(255, 212, 59, 0.8)";
        ctx.beginPath();
        ctx.arc(x, y, 4, 0, Math.PI * 2);
        ctx.fill();
      }
    });

    // Draw current frame marker
    const currentX = margin.left + currentFrameIndex * stepX;

    ctx.strokeStyle = "#ffffff";
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(currentX, margin.top);
    ctx.lineTo(currentX, margin.top + graphHeight);
    ctx.stroke();

    // Draw Y-axis labels
    ctx.fillStyle = "#888";
    ctx.font = "10px monospace";
    ctx.textAlign = "right";
    ctx.textBaseline = "middle";

    // This panel shares the filmstrip's fixed-height strip with the other view modes, so
    // `graphHeight` here is typically small (tens of pixels once header/legend/margins are
    // subtracted) -- a fixed 5-label set (0/25/50/75/100%) doesn't leave enough room for 10px
    // text between adjacent labels and visibly overlaps into an illegible stack. Scale the label
    // count to how much vertical room is actually available instead.
    const minYLabelSpacingPx = 14;
    const maxYLabelCount = Math.max(
      2,
      Math.floor(graphHeight / minYLabelSpacingPx) + 1,
    );
    const yLabelCount = Math.min(5, maxYLabelCount);
    const yLabels = Array.from(
      { length: yLabelCount },
      (_, i) => (100 * i) / (yLabelCount - 1),
    );
    yLabels.forEach((percent) => {
      const y = margin.top + graphHeight - (percent / 100) * graphHeight;
      const value = Math.round((bufferSize * percent) / 100 / 1024);
      ctx.fillText(`${value}KB`, margin.left - 8, y);
    });

    // Draw X-axis labels (frames)
    ctx.textAlign = "center";
    ctx.textBaseline = "top";

    const xSteps = 5;
    const xStep = Math.ceil(history.length / xSteps);
    for (let i = 0; i < history.length; i += xStep) {
      const x = margin.left + i * stepX;
      ctx.fillText(`${history[i].frame}`, x, margin.top + graphHeight + 8);
    }
  }, [
    hrdState,
    currentFrameIndex,
    bufferSize,
    effectiveTargetBitrate,
    frameRate,
  ]);

  // Redraw when data changes
  useEffect(() => {
    drawCanvas();
  }, [drawCanvas]);

  // Redraw canvas on container resize
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const observer = new ResizeObserver(() => drawCanvas());
    observer.observe(container);
    return () => observer.disconnect();
  }, [drawCanvas]);

  // Handle mouse move for tooltip
  const handleMouseMove = useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      const canvas = canvasRef.current;
      if (!canvas) return;

      const rect = canvas.getBoundingClientRect();
      const x = e.clientX - rect.left;

      const margin = { top: 20, right: 10, bottom: 30, left: 50 };
      const graphWidth = rect.width - margin.left - margin.right;

      if (x < margin.left || x > margin.left + graphWidth) {
        setHoverData(null);
        return;
      }

      const history = hrdState.occupancyHistory;
      const stepX = graphWidth / Math.max(1, history.length - 1);
      const frameIndex = Math.round((x - margin.left) / stepX);

      if (frameIndex >= 0 && frameIndex < history.length) {
        setHoverData(history[frameIndex]);
      }
    },
    [hrdState],
  );

  const overflowCount = hrdState.occupancyHistory.filter(
    (p) => p.overflow,
  ).length;
  const underflowCount = hrdState.occupancyHistory.filter(
    (p) => p.underflow,
  ).length;
  // The header "Buffer:" stat must track the currently viewed frame, not the stream's last frame
  // -- `hrdState.occupancy` is left over from the end of the simulation loop regardless of
  // `currentFrameIndex`, which previously made this stat show the same (coincidentally
  // math-guaranteed-to-return-to-baseline) value no matter which frame was selected.
  const currentOccupancyAtFrame =
    hrdState.occupancyHistory[currentFrameIndex]?.occupancy ??
    hrdState.occupancy;
  const currentPercent = (currentOccupancyAtFrame / bufferSize) * 100;

  return (
    <div className="hrd-buffer-panel">
      <div className="hrd-header">
        <h3>HRD Buffer</h3>
        <div className="hrd-stats">
          <span className="hrd-stat">
            <span className="stat-label">Buffer:</span>
            <span className="stat-value">{currentPercent.toFixed(1)}%</span>
          </span>
          {effectiveTargetBitrate !== undefined && (
            <span className="hrd-stat">
              <span className="stat-label">
                {isEstimatedBitrate ? "Avg bitrate (est.):" : "Target:"}
              </span>
              <span className="stat-value">
                {(effectiveTargetBitrate / 1000000).toFixed(2)} Mbps
              </span>
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
              {(hoverData.occupancy / 1024).toFixed(1)} KB (
              {((hoverData.occupancy / bufferSize) * 100).toFixed(1)}%)
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
