/**
 * HRD (Hypothetical Reference Decoder) Buffer Panel - F9
 *
 * Plots the AV1 decoder model's smoothing buffer (spec Annex E): bits in the buffer over time at
 * the level's BitRate, the BufferSize limit, each frame's removal, and the frames that break a
 * conformance requirement (E.6). The model is computed by the sidecar from the sequence header's
 * decoder model, every frame's `buffer_removal_time` and the frame sizes; a stream that signals no
 * timing has no model and the panel says so (#587).
 */

import { useMemo, useRef, useEffect, useState, useCallback, memo } from "react";
import type {
  HrdModelFrameWire,
  HrdModelWire,
} from "../../services/electronBridgeService";
import {
  VIOLATION_LABELS,
  countViolations,
  formatSeconds,
  frameOfPacket,
  nearestFrame,
  occupancyBefore,
  violationsAt,
} from "../../utils/hrdModel";
import "./HRDBufferPanel.css";

export interface HRDBufferPanelProps {
  /** The decoder model run over the stream; `null` when the stream has none. */
  model: HrdModelWire | null;
  /** Why there is no model, when the sidecar said. */
  unavailableReason?: string | null;
  /** Index of the selected frame in the frame list (the IVF packet). */
  currentFrameIndex: number;
}

const MARGIN = { top: 20, right: 10, bottom: 30, left: 56 };
const COLOR_OVERFLOW = "#ff6b6b";
const COLOR_UNDERFLOW = "#ffd43b";
const COLOR_OTHER = "#ff922b";
const MODE_LABEL: Record<HrdModelWire["mode"], string> = {
  decoding_schedule: "Decoding schedule",
  resource_availability: "Resource availability",
};

/** The time and bit ranges of the graph: from 0 to the last removal or arrival, and from the
 *  lowest level (below 0 after an underflow) to the BufferSize limit or the peak. */
function graphBounds(model: HrdModelWire) {
  let endTime = 0;
  let minBits = 0;
  for (const [time, bits] of model.curve) {
    endTime = Math.max(endTime, time);
    minBits = Math.min(minBits, bits);
  }
  const maxBits = Math.max(model.buffer_size_bits, model.peak_bits) * 1.1;
  return { endTime: endTime || 1, minBits: minBits * 1.1, maxBits };
}

function violationColor(kinds: string[]): string {
  if (kinds.includes("smoothing_buffer_overflow")) return COLOR_OVERFLOW;
  if (kinds.includes("smoothing_buffer_underflow")) return COLOR_UNDERFLOW;
  return COLOR_OTHER;
}

const HRDBufferPanelInternal = ({
  model,
  currentFrameIndex,
}: {
  model: HrdModelWire;
  currentFrameIndex: number;
}) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [hoverFrame, setHoverFrame] = useState<HrdModelFrameWire | null>(null);

  const bounds = useMemo(() => graphBounds(model), [model]);
  const counts = useMemo(
    () => countViolations(model.violations),
    [model.violations],
  );
  const currentFrame = useMemo(
    () => frameOfPacket(model.frames, currentFrameIndex),
    [model.frames, currentFrameIndex],
  );
  // The header "Buffer:" stat follows the selected frame: the level just before it is removed.
  const currentBits = currentFrame
    ? occupancyBefore(model.curve, currentFrame.removal)
    : undefined;

  const drawCanvas = useCallback(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const rect = container.getBoundingClientRect();
    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    canvas.style.width = `${rect.width}px`;
    canvas.style.height = `${rect.height}px`;
    ctx.scale(dpr, dpr);

    ctx.fillStyle = "#1e1e1e";
    ctx.fillRect(0, 0, rect.width, rect.height);

    const graphWidth = rect.width - MARGIN.left - MARGIN.right;
    const graphHeight = rect.height - MARGIN.top - MARGIN.bottom;
    const { endTime, minBits, maxBits } = bounds;
    const x = (time: number) => MARGIN.left + (time / endTime) * graphWidth;
    const y = (bits: number) =>
      MARGIN.top +
      graphHeight -
      ((bits - minBits) / (maxBits - minBits)) * graphHeight;

    // BufferSize: the smoothing buffer must never hold more
    ctx.strokeStyle = COLOR_OVERFLOW;
    ctx.lineWidth = 1;
    ctx.setLineDash([5, 5]);
    ctx.beginPath();
    ctx.moveTo(MARGIN.left, y(model.buffer_size_bits));
    ctx.lineTo(MARGIN.left + graphWidth, y(model.buffer_size_bits));
    ctx.stroke();
    ctx.setLineDash([]);

    // empty buffer: a removal that takes the level below it is an underflow
    ctx.strokeStyle = "#555";
    ctx.beginPath();
    ctx.moveTo(MARGIN.left, y(0));
    ctx.lineTo(MARGIN.left + graphWidth, y(0));
    ctx.stroke();

    // occupancy
    ctx.strokeStyle = "#339af0";
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    model.curve.forEach(([time, bits], i) => {
      if (i === 0) ctx.moveTo(x(time), y(bits));
      else ctx.lineTo(x(time), y(bits));
    });
    ctx.stroke();

    // frames that break a requirement, at the level just before their removal
    for (const frame of model.frames) {
      const kinds = violationsAt(model, frame.dfg).map((v) => v.kind);
      if (kinds.length === 0) continue;
      ctx.fillStyle = violationColor(kinds);
      ctx.beginPath();
      ctx.arc(
        x(frame.removal),
        y(occupancyBefore(model.curve, frame.removal)),
        4,
        0,
        Math.PI * 2,
      );
      ctx.fill();
    }

    // selected frame's removal
    if (currentFrame) {
      ctx.strokeStyle = "#ffffff";
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(x(currentFrame.removal), MARGIN.top);
      ctx.lineTo(x(currentFrame.removal), MARGIN.top + graphHeight);
      ctx.stroke();
    }

    // axes: kbit on Y, seconds on X
    ctx.fillStyle = "#888";
    ctx.font = "10px monospace";
    ctx.textAlign = "right";
    ctx.textBaseline = "middle";
    const yLabelCount = Math.min(
      5,
      Math.max(2, Math.floor(graphHeight / 14) + 1),
    );
    for (let i = 0; i < yLabelCount; i++) {
      const bits = minBits + ((maxBits - minBits) * i) / (yLabelCount - 1);
      ctx.fillText(`${Math.round(bits / 1000)}kb`, MARGIN.left - 8, y(bits));
    }
    ctx.textAlign = "center";
    ctx.textBaseline = "top";
    for (let i = 0; i <= 5; i++) {
      const time = (endTime * i) / 5;
      ctx.fillText(
        `${time.toFixed(2)}s`,
        x(time),
        MARGIN.top + graphHeight + 8,
      );
    }
  }, [model, bounds, currentFrame]);

  useEffect(() => {
    drawCanvas();
  }, [drawCanvas]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const observer = new ResizeObserver(() => drawCanvas());
    observer.observe(container);
    return () => observer.disconnect();
  }, [drawCanvas]);

  const handleMouseMove = useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      const canvas = canvasRef.current;
      if (!canvas) return;
      const rect = canvas.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const graphWidth = rect.width - MARGIN.left - MARGIN.right;
      if (x < MARGIN.left || x > MARGIN.left + graphWidth) {
        setHoverFrame(null);
        return;
      }
      const time = ((x - MARGIN.left) / graphWidth) * bounds.endTime;
      setHoverFrame(nearestFrame(model.frames, time) ?? null);
    },
    [model.frames, bounds.endTime],
  );

  return (
    <div className="hrd-buffer-panel">
      <div className="hrd-header">
        <h3>HRD Buffer</h3>
        <div className="hrd-stats">
          <span className="hrd-stat">
            <span className="stat-label">Mode:</span>
            <span className="stat-value">
              {MODE_LABEL[model.mode]}
              {model.low_delay_mode ? " (low delay)" : ""}
            </span>
          </span>
          {currentBits !== undefined && (
            <span className="hrd-stat">
              <span className="stat-label">Buffer:</span>
              <span className="stat-value">
                {((currentBits / model.buffer_size_bits) * 100).toFixed(1)}%
              </span>
            </span>
          )}
          <span className="hrd-stat">
            <span className="stat-label">BitRate:</span>
            <span className="stat-value">
              {(model.bit_rate_bps / 1_000_000).toFixed(2)} Mbps
            </span>
          </span>
          {counts.overflow > 0 && (
            <span className="hrd-stat overflow">
              <span className="stat-label">Overflow:</span>
              <span className="stat-value">{counts.overflow}</span>
            </span>
          )}
          {counts.underflow > 0 && (
            <span className="hrd-stat underflow">
              <span className="stat-label">Underflow:</span>
              <span className="stat-value">{counts.underflow}</span>
            </span>
          )}
          {counts.other > 0 && (
            <span className="hrd-stat violation">
              <span className="stat-label">Other violations:</span>
              <span className="stat-value">{counts.other}</span>
            </span>
          )}
        </div>
      </div>

      <div ref={containerRef} className="hrd-canvas-container">
        <canvas
          ref={canvasRef}
          onMouseMove={handleMouseMove}
          onMouseLeave={() => setHoverFrame(null)}
        />
      </div>

      {hoverFrame && (
        <div className="hrd-tooltip">
          <div className="tooltip-row">
            <span className="tooltip-label">Frame:</span>
            <span className="tooltip-value">
              {hoverFrame.packet} ({hoverFrame.frame_type})
            </span>
          </div>
          <div className="tooltip-row">
            <span className="tooltip-label">Size:</span>
            <span className="tooltip-value">
              {(hoverFrame.coded_bits / 8 / 1024).toFixed(1)} KB
            </span>
          </div>
          <div className="tooltip-row">
            <span className="tooltip-label">Arrives:</span>
            <span className="tooltip-value">
              {formatSeconds(hoverFrame.first_bit_arrival)} to{" "}
              {formatSeconds(hoverFrame.last_bit_arrival)}
            </span>
          </div>
          <div className="tooltip-row">
            <span className="tooltip-label">Removed:</span>
            <span className="tooltip-value">
              {formatSeconds(hoverFrame.removal)}
              {hoverFrame.removal > hoverFrame.scheduled_removal
                ? ` (due ${formatSeconds(hoverFrame.scheduled_removal)})`
                : ""}
            </span>
          </div>
          <div className="tooltip-row">
            <span className="tooltip-label">Occupancy:</span>
            <span className="tooltip-value">
              {(
                occupancyBefore(model.curve, hoverFrame.removal) / 1000
              ).toFixed(1)}{" "}
              kb (
              {(
                (occupancyBefore(model.curve, hoverFrame.removal) /
                  model.buffer_size_bits) *
                100
              ).toFixed(1)}
              %)
            </span>
          </div>
          {violationsAt(model, hoverFrame.dfg).map((v) => (
            <div key={v.kind} className="tooltip-row violation">
              <span className="tooltip-label">
                ⚠ {VIOLATION_LABELS[v.kind]}
              </span>
            </div>
          ))}
        </div>
      )}

      {model.violations
        .filter((v) => v.dfg === null)
        .map((v) => (
          <div key={v.kind} className="hrd-stream-violation" role="alert">
            ⚠ {VIOLATION_LABELS[v.kind]}
          </div>
        ))}

      <div className="hrd-legend">
        <div className="legend-item">
          <span className="legend-color line-buffer"></span>
          <span className="legend-label">BufferSize</span>
        </div>
        <div className="legend-item">
          <span className="legend-color dot-overflow"></span>
          <span className="legend-label">Overflow</span>
        </div>
        <div className="legend-item">
          <span className="legend-color dot-underflow"></span>
          <span className="legend-label">Underflow</span>
        </div>
        <div className="legend-item">
          <span className="legend-color dot-violation"></span>
          <span className="legend-label">Other violation</span>
        </div>
      </div>
    </div>
  );
};

// A stream with no decoder model the view can run (other codecs, no timing_info, an undefined
// level) has nothing signaled to show: the view says so instead of drawing numbers the stream
// never gave (#461), matching the reference analyzer's greyed-out HRD view.
const HRDBufferPanelGate = ({
  model,
  unavailableReason,
  currentFrameIndex,
}: HRDBufferPanelProps) => {
  if (!model) {
    return (
      <div className="hrd-buffer-panel hrd-unavailable" role="status">
        <div className="hrd-header">
          <h3>HRD Buffer</h3>
        </div>
        <p className="hrd-unavailable-message">
          No HRD information in this stream.
        </p>
        {unavailableReason && (
          <p className="hrd-unavailable-reason">{unavailableReason}</p>
        )}
      </div>
    );
  }
  return (
    <HRDBufferPanelInternal
      model={model}
      currentFrameIndex={currentFrameIndex}
    />
  );
};

export const HRDBufferPanel = memo(HRDBufferPanelGate);
export default HRDBufferPanel;
