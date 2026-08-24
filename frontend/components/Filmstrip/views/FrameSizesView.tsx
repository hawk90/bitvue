/**
 * Frame Sizes View Component
 *
 * Bar chart showing frame sizes with dual Y-axes (Bitrate and QP)
 * Includes line graph overlay for bitrate curve
 */

import { useMemo, memo, useCallback } from "react";
import type { FrameInfo } from "../../../types/video";
import { getFrameTypeColor } from "../../../types/video";

export interface SizeMetrics {
  showBitrateBar: boolean;
  showBitrateCurve: boolean;
  showAvgSize: boolean;
  showMinSize: boolean;
  showMaxSize: boolean;
  showMovingAvg: boolean;
  showBlockMinQP: boolean;
  showBlockMaxQP: boolean;
}

interface FrameSizesViewProps {
  frames: FrameInfo[];
  maxSize: number;
  currentFrameIndex: number;
  visibleFrameTypes: Set<string>;
  onFrameClick: (frameIndex: number) => void;
  getFrameTypeColorClass: (frameType: string) => string;
  sizeMetrics: SizeMetrics;
}

// Calculate moving average (21-frame window)
function calculateMovingAvg(
  frames: FrameInfo[],
  windowSize: number = 21,
): number[] {
  return frames.map((_, i) => {
    const start = Math.max(0, i - Math.floor(windowSize / 2));
    const end = Math.min(frames.length, i + Math.ceil(windowSize / 2));
    const slice = frames.slice(start, end);
    const avg = slice.reduce((sum, f) => sum + f.size, 0) / slice.length;
    return avg;
  });
}

// Get QP value from frame (real data if available, otherwise estimate)
function getFrameQP(frame: FrameInfo, maxSize: number): number {
  // Use real QP grid data if available
  if (frame.qp_grid && frame.qp_grid.qp_min !== undefined) {
    // Return average QP for this frame
    return frame.qp_grid.qp_min;
  }

  // Fallback: Estimate QP from frame size (simplified model)
  const normalizedSize = frame.size / maxSize;
  return Math.round(51 * (1 - normalizedSize * 0.7));
}

// Headroom above the tallest bar/curve point, as a multiplier on the true max value used for
// scaling. Without it, a bar for the single largest frame renders at exactly 100% height and
// touches the chart container's top edge flush -- reading as "cut off" (found via a real
// screenshot: frame 0, almost always the biggest frame in any real stream since it's the I-frame,
// sits directly against the ceiling while the "10 KB" axis label above it has visible padding,
// making the two look misaligned even though both are technically correct). The axis labels still
// show the true max/mid/min (unaffected) -- only the bar/line *scaling* gets this margin.
const Y_AXIS_HEADROOM = 1.08;

// Calculate bitrate from frame size and duration
// Returns bitrate in bits per time_unit (arbitrary units for relative comparison)
function calculateBitrate(frame: FrameInfo): number | null {
  if (frame.duration && frame.duration > 0) {
    // bitrate = (size_in_bytes * 8_bits_per_byte) / duration
    return (frame.size * 8) / frame.duration;
  }
  return null;
}

function FrameSizesView({
  frames,
  maxSize,
  currentFrameIndex,
  visibleFrameTypes,
  onFrameClick,
  getFrameTypeColorClass,
  sizeMetrics,
}: FrameSizesViewProps) {
  // Calculate statistics
  const avgSize = useMemo(() => {
    if (frames.length === 0) return 0;
    return frames.reduce((sum, f) => sum + f.size, 0) / frames.length;
  }, [frames]);

  const minSize = useMemo(() => {
    if (frames.length === 0) return 0;
    return Math.min(...frames.map((f) => f.size));
  }, [frames]);

  const maxFrameSize = useMemo(() => {
    if (frames.length === 0) return maxSize;
    return Math.max(...frames.map((f) => f.size));
  }, [frames, maxSize]);

  const movingAvg = useMemo(() => calculateMovingAvg(frames), [frames]);

  // Calculate bitrate for each frame (for line curve)
  const frameBitrates = useMemo(() => {
    return frames.map((f) => calculateBitrate(f));
  }, [frames]);

  // Calculate max bitrate for scaling the line curve
  const maxBitrate = useMemo(() => {
    const bitrates = frameBitrates.filter((b): b is number => b !== null);
    if (bitrates.length === 0) return 1;
    return Math.max(...bitrates);
  }, [frameBitrates]);

  // Denominators actually used for bar/line height math (see Y_AXIS_HEADROOM) -- axis labels
  // below deliberately keep using the unscaled `maxFrameSize`/`maxBitrate`.
  const scaledMaxFrameSize = maxFrameSize * Y_AXIS_HEADROOM;
  const scaledMaxBitrate = maxBitrate * Y_AXIS_HEADROOM;

  const isFrameTypeVisible = useCallback(
    (frameType: string): boolean => {
      return visibleFrameTypes.has(frameType);
    },
    [visibleFrameTypes],
  );

  // Memoize filtered frames to avoid recalculation on every render
  const filteredFrames = useMemo(() => {
    return frames.filter((f) => isFrameTypeVisible(f.frame_type));
  }, [frames, isFrameTypeVisible]);

  // Calculate QP range for right Y-axis
  const minQP = 0;
  const maxQP = 51;

  return (
    <div className="filmstrip-sizes">
      {/* Left Y-axis - Bitrate (KB) */}
      <div className="frame-sizes-axis axis-left">
        <div className="axis-scale">
          <div className="axis-label max">
            {(maxFrameSize / 1024).toFixed(0)} KB
          </div>
          <div className="axis-label mid">
            {(maxFrameSize / 2 / 1024).toFixed(0)} KB
          </div>
          <div className="axis-label min">0</div>
        </div>
        <div className="axis-title">Bitrate</div>
      </div>

      {/* Chart Container */}
      <div className="frame-sizes-chart">
        {/* Plot area -- must span exactly the same height as each axis's `.axis-scale` (both are
            the sibling of a same-height `.axis-title`/`.frame-sizes-chart-footer` reserving equal
            space below) so a bar/line's 0%/50%/100% lines up with the axis labels marking those
            same points, on both the left and right axis. Before this, `.axis-label`s sat inside a
            4-item `justify-content: space-between` column that also included `.axis-title`, so
            "50%"/"0%" landed wherever a 4-way space-between put them -- not at the plot's actual
            50%/0% -- while this plot area had no such extra item eating into its own 0-100% box.
            Found via the user reporting the Y-axis "looks unaligned" on both the left and right
            side, not just one -- consistent with a shared structural cause, not a one-off. */}
        <div className="frame-sizes-plot">
          {/* Bar Chart */}
          <div className="frame-sizes-bars">
            {filteredFrames.map((frame) => {
              const qp = getFrameQP(frame, maxSize);
              const barHeight = (frame.size / scaledMaxFrameSize) * 100;

              return (
                <div
                  key={frame.frame_index}
                  data-frame-index={frame.frame_index}
                  className={`frame-size-bar ${getFrameTypeColorClass(frame.frame_type)} ${
                    frame.frame_index === currentFrameIndex ? "selected" : ""
                  }`}
                  onClick={() => onFrameClick(frame.frame_index)}
                  style={{
                    height: `${barHeight}%`,
                    backgroundColor: getFrameTypeColor(frame.frame_type),
                  }}
                  title={`Frame ${frame.frame_index}: ${(frame.size / 1024).toFixed(1)} KB, QP: ${qp}`}
                />
              );
            })}
          </div>

          {/* Line Chart Overlay (SVG) - Bitrate Curve */}
          {sizeMetrics.showBitrateCurve && filteredFrames.length > 1 && (
            <svg
              className="frame-sizes-line-chart"
              viewBox="0 0 100 100"
              preserveAspectRatio="none"
            >
              <polyline
                points={filteredFrames
                  .map((frame, idx) => {
                    const x = (idx / (filteredFrames.length - 1)) * 100;
                    const bitrate = calculateBitrate(frame);
                    // Use bitrate for the line, scaled by maxBitrate
                    const y =
                      bitrate !== null
                        ? 100 - (bitrate / scaledMaxBitrate) * 100
                        : 100;
                    return `${x},${y}`;
                  })
                  .join(" ")}
                fill="none"
                stroke="rgba(0, 220, 220, 1)"
                strokeWidth="1"
                vectorEffect="non-scaling-stroke"
              />
            </svg>
          )}

          {/* Moving Average Line - frame size moving average */}
          {sizeMetrics.showMovingAvg && filteredFrames.length > 1 && (
            <svg
              className="frame-sizes-line-chart"
              viewBox="0 0 100 100"
              preserveAspectRatio="none"
            >
              <polyline
                points={filteredFrames
                  .map((frame, idx) => {
                    const x = (idx / (filteredFrames.length - 1)) * 100;
                    // Find the moving avg for this frame (by original index)
                    const originalIdx = frames.indexOf(frame);
                    const avgSize =
                      originalIdx >= 0 ? movingAvg[originalIdx] : 0;
                    const y = 100 - (avgSize / scaledMaxFrameSize) * 100;
                    return `${x},${y}`;
                  })
                  .join(" ")}
                fill="none"
                stroke="rgba(0, 150, 255, 0.8)"
                strokeWidth="0.8"
                vectorEffect="non-scaling-stroke"
              />
            </svg>
          )}

          {/* Average Size Line */}
          {sizeMetrics.showAvgSize && (
            <div
              className="metric-line avg-line"
              style={{ bottom: `${(avgSize / scaledMaxFrameSize) * 100}%` }}
            />
          )}

          {/* Min Size Line */}
          {sizeMetrics.showMinSize && (
            <div
              className="metric-line min-line"
              style={{ bottom: `${(minSize / scaledMaxFrameSize) * 100}%` }}
            />
          )}

          {/* Max Size Line */}
          {sizeMetrics.showMaxSize && (
            <div
              className="metric-line max-line"
              style={{
                bottom: `${(maxFrameSize / scaledMaxFrameSize) * 100}%`,
              }}
            />
          )}
        </div>
        {/* Empty spacer matching each axis's `.axis-title` height exactly (see the comment above
            `.frame-sizes-plot`) -- keeps this plot area's own 0-100% box the same height as the
            axis scale's, with nothing rendered in it. */}
        <div className="frame-sizes-chart-footer" />
      </div>

      {/* Right Y-axis - QP (0-51) */}
      <div className="frame-sizes-axis axis-right">
        <div className="axis-scale">
          <div className="axis-label max">{maxQP}</div>
          <div className="axis-label mid">{Math.round(maxQP / 2)}</div>
          <div className="axis-label min">{minQP}</div>
        </div>
        <div className="axis-title">QP</div>
      </div>
    </div>
  );
}

// Memoize FrameSizesView to prevent unnecessary re-renders
export default memo(FrameSizesView, (prevProps, nextProps) => {
  return (
    prevProps.frames === nextProps.frames &&
    prevProps.maxSize === nextProps.maxSize &&
    prevProps.currentFrameIndex === nextProps.currentFrameIndex &&
    prevProps.sizeMetrics === nextProps.sizeMetrics
  );
});
