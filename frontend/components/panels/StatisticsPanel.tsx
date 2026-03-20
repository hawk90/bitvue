/**
 * Statistics Panel
 *
 * Stream statistics and metrics
 * Reference: Statistics
 */

import { useMemo, memo } from "react";
import { useFrameData } from "../../contexts/FrameDataContext";
import { BarChart } from "../charts/BarChart";
import { getCssVar } from "../../utils/css";
import "./StatisticsPanel.css";

/**
 * Frame size range categories for statistics
 */
const FRAME_SIZE_RANGES = {
  SMALL: "< 10KB",
  MEDIUM_SMALL: "10-50KB",
  MEDIUM: "50-100KB",
  MEDIUM_LARGE: "100-500KB",
  LARGE: "> 500KB",
} as const;

/**
 * Size thresholds in KB for categorizing frames
 */
const SIZE_THRESHOLDS = {
  SMALL: 10,
  MEDIUM_SMALL: 50,
  MEDIUM: 100,
  MEDIUM_LARGE: 500,
} as const;

interface StatisticsPanelProps {
  /** Frame rate in fps used for bitrate calculation. Defaults to 30 if not provided. */
  frameRate?: number;
}

export const StatisticsPanel = memo(function StatisticsPanel({
  frameRate,
}: StatisticsPanelProps) {
  const { frames, getFrameStats } = useFrameData();

  const stats = useMemo(() => getFrameStats(), [getFrameStats]);

  // Derive frame rate from PTS delta when available; fall back to prop or 30fps default
  const effectiveFrameRate = useMemo(() => {
    if (frameRate !== undefined && frameRate > 0) return frameRate;
    if (frames.length >= 2) {
      const pts0 = frames[0]?.pts;
      const pts1 = frames[1]?.pts;
      if (pts0 !== undefined && pts1 !== undefined && pts1 > pts0) {
        const ptsDelta = pts1 - pts0;
        // PTS is typically in 90kHz units for MPEG or 1 unit per frame — guard against implausible values
        const derived = 1 / ptsDelta;
        if (derived >= 1 && derived <= 240) return derived;
      }
    }
    return 30;
  }, [frameRate, frames]);

  // Calculate frame size distribution for histogram
  const frameSizes = useMemo(() => {
    if (frames.length === 0) return {};

    // Group by size ranges
    const ranges: Record<string, number> = {
      [FRAME_SIZE_RANGES.SMALL]: 0,
      [FRAME_SIZE_RANGES.MEDIUM_SMALL]: 0,
      [FRAME_SIZE_RANGES.MEDIUM]: 0,
      [FRAME_SIZE_RANGES.MEDIUM_LARGE]: 0,
      [FRAME_SIZE_RANGES.LARGE]: 0,
    };

    frames.forEach((frame) => {
      const sizeKB = frame.size / 1024;
      if (sizeKB < SIZE_THRESHOLDS.SMALL) ranges[FRAME_SIZE_RANGES.SMALL]++;
      else if (sizeKB < SIZE_THRESHOLDS.MEDIUM_SMALL)
        ranges[FRAME_SIZE_RANGES.MEDIUM_SMALL]++;
      else if (sizeKB < SIZE_THRESHOLDS.MEDIUM)
        ranges[FRAME_SIZE_RANGES.MEDIUM]++;
      else if (sizeKB < SIZE_THRESHOLDS.MEDIUM_LARGE)
        ranges[FRAME_SIZE_RANGES.MEDIUM_LARGE]++;
      else ranges[FRAME_SIZE_RANGES.LARGE]++;
    });

    return ranges;
  }, [frames]);

  const maxSizeRange = useMemo(() => {
    const values = Object.values(frameSizes) as number[];
    return values.length > 0 ? Math.max(...values) : 0;
  }, [frameSizes]);

  return (
    <div className="statistics-panel">
      <div className="panel-header">
        <span className="panel-title">Statistics</span>
      </div>

      <div className="statistics-content">
        {/* Frame Type Distribution */}
        <section className="stats-section">
          <h3 className="stats-section-title">Frame Types</h3>
          <div className="stats-section-content">
            <BarChart
              data={stats.frameTypes}
              maxValue={Math.max(...Object.values(stats.frameTypes))}
            />
            <div className="stats-summary">
              <span className="stats-summary-item">
                <span className="stats-label">Total:</span>
                <span className="stats-value">{stats.totalFrames}</span>
              </span>
              <span className="stats-summary-item">
                <span className="stats-label">Keyframes:</span>
                <span className="stats-value">{stats.keyFrames}</span>
              </span>
              <span className="stats-summary-item">
                <span className="stats-label">Avg Size:</span>
                <span className="stats-value">
                  {(stats.avgSize / 1024).toFixed(2)} KB
                </span>
              </span>
            </div>
          </div>
        </section>

        {/* Frame Size Distribution */}
        <section className="stats-section">
          <h3 className="stats-section-title">Frame Sizes</h3>
          <div className="stats-section-content">
            <BarChart
              data={frameSizes}
              maxValue={maxSizeRange}
              colors={{
                "< 10KB": getCssVar("--color-info"),
                "10-50KB": getCssVar("--frame-p"),
                "50-100KB": getCssVar("--color-success"),
                "100-500KB": getCssVar("--accent-primary-light"),
                "> 500KB": getCssVar("--frame-b"),
              }}
            />
          </div>
        </section>

        {/* Bitrate Info */}
        <section className="stats-section">
          <h3 className="stats-section-title">Bitrate</h3>
          <div className="stats-section-content">
            <div className="bitrate-info">
              <div className="bitrate-label">Total Size:</div>
              <div className="bitrate-value">
                {(stats.totalSize / 1024 / 1024).toFixed(2)} MB
              </div>
            </div>
            <div className="bitrate-info">
              <div className="bitrate-label">Avg Bitrate:</div>
              <div className="bitrate-value">
                {frames.length > 0
                  ? (
                      ((stats.totalSize / frames.length) * effectiveFrameRate) /
                      1000
                    ).toFixed(2)
                  : "0"}{" "}
                Mbps
              </div>
            </div>
            <div className="bitrate-note">
              Assuming {Math.round(effectiveFrameRate)}fps
            </div>
          </div>
        </section>
      </div>
    </div>
  );
});
