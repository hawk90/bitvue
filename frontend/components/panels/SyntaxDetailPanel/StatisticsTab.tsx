/**
 * Statistics Tab Component
 *
 * Displays frame and stream statistics with SVG charts:
 * - Pie chart: frame type distribution
 * - Bar chart: per-frame sizes (sparkline with current-frame highlight)
 * - Stream stats table
 */

import React, { memo, useMemo } from "react";

// ─── Frame type colour palette ────────────────────────────────────────────────

const FRAME_TYPE_COLORS: Record<string, string> = {
  I: "#4fc3f7",
  KEY: "#4fc3f7",
  P: "#66bb6a",
  B: "#ffa726",
  SP: "#ef5350",
  SI: "#ab47bc",
  BI: "#ff7043",
  S: "#26c6da",
  INTRA: "#4fc3f7",
  INTER: "#66bb6a",
};

const DEFAULT_COLOR = "#78909c";

function frameColor(type: string): string {
  return FRAME_TYPE_COLORS[type.toUpperCase()] ?? DEFAULT_COLOR;
}

// ─── Types ────────────────────────────────────────────────────────────────────

interface FrameRow {
  frame_index: number;
  frame_type: string;
  size: number;
  temporal_id?: number;
  display_order?: number;
  coding_order?: number;
  ref_frames?: number[];
}

interface StatisticsTabProps {
  currentFrame: FrameRow | null;
  frames: FrameRow[];
}

// ─── Pie chart ────────────────────────────────────────────────────────────────

interface PieSlice {
  type: string;
  count: number;
  color: string;
  startAngle: number;
  endAngle: number;
}

function polarToCartesian(
  cx: number,
  cy: number,
  r: number,
  angleDeg: number,
): [number, number] {
  const rad = ((angleDeg - 90) * Math.PI) / 180;
  return [cx + r * Math.cos(rad), cy + r * Math.sin(rad)];
}

function describeArc(
  cx: number,
  cy: number,
  r: number,
  startAngle: number,
  endAngle: number,
): string {
  const [sx, sy] = polarToCartesian(cx, cy, r, startAngle);
  const [ex, ey] = polarToCartesian(cx, cy, r, endAngle);
  const largeArc = endAngle - startAngle > 180 ? 1 : 0;
  return `M ${cx} ${cy} L ${sx} ${sy} A ${r} ${r} 0 ${largeArc} 1 ${ex} ${ey} Z`;
}

interface PieChartProps {
  slices: PieSlice[];
  total: number;
  size?: number;
}

const PieChart = memo(function PieChart({
  slices,
  total,
  size = 100,
}: PieChartProps) {
  const cx = size / 2;
  const cy = size / 2;
  const r = size / 2 - 4;

  if (total === 0) {
    return (
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`}>
        <circle
          cx={cx}
          cy={cy}
          r={r}
          fill="none"
          stroke="#444"
          strokeWidth={1}
        />
        <text x={cx} y={cy + 4} textAnchor="middle" fontSize={9} fill="#888">
          No data
        </text>
      </svg>
    );
  }

  // Single-type: draw a full circle
  if (slices.length === 1) {
    return (
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`}>
        <circle cx={cx} cy={cy} r={r} fill={slices[0].color} />
      </svg>
    );
  }

  return (
    <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`}>
      {slices.map((s) => (
        <path
          key={s.type}
          d={describeArc(cx, cy, r, s.startAngle, s.endAngle)}
          fill={s.color}
          stroke="var(--bg-app)"
          strokeWidth={1}
        >
          <title>
            {s.type}: {s.count} ({((s.count / total) * 100).toFixed(1)}%)
          </title>
        </path>
      ))}
    </svg>
  );
});

// ─── Bar chart (frame-size sparkline) ────────────────────────────────────────

interface BarChartProps {
  frames: FrameRow[];
  currentFrameIndex: number;
  height?: number;
}

const BarChart = memo(function BarChart({
  frames,
  currentFrameIndex,
  height = 52,
}: BarChartProps) {
  const maxSize = useMemo(
    () => Math.max(...frames.map((f) => f.size), 1),
    [frames],
  );

  if (frames.length === 0) {
    return <div className="stats-chart-empty">No frame data</div>;
  }

  const barW = Math.max(1, Math.min(8, Math.floor(240 / frames.length)));
  const gap = barW > 2 ? 1 : 0;
  const svgW = frames.length * (barW + gap);
  const usableH = height - 2;

  return (
    <svg
      width="100%"
      height={height}
      viewBox={`0 0 ${svgW} ${height}`}
      preserveAspectRatio="none"
      style={{ display: "block" }}
    >
      {frames.map((f, i) => {
        const barH = Math.max(1, (f.size / maxSize) * usableH);
        const x = i * (barW + gap);
        const y = height - barH;
        const isCurrent = i === currentFrameIndex;
        const color = isCurrent ? "#ffffff" : frameColor(f.frame_type);
        return (
          <rect
            key={i}
            x={x}
            y={y}
            width={barW}
            height={barH}
            fill={color}
            opacity={isCurrent ? 1 : 0.75}
          >
            <title>
              #{f.frame_index} {f.frame_type} — {(f.size / 1024).toFixed(2)} KB
            </title>
          </rect>
        );
      })}
    </svg>
  );
});

// ─── Main component ───────────────────────────────────────────────────────────

export const StatisticsTab = memo(function StatisticsTab({
  currentFrame,
  frames,
}: StatisticsTabProps) {
  const totalFrames = frames.length;

  // Aggregate frame type counts
  const typeCounts = useMemo(
    () =>
      frames.reduce(
        (acc, f) => {
          const t = f.frame_type.toUpperCase();
          acc[t] = (acc[t] || 0) + 1;
          return acc;
        },
        {} as Record<string, number>,
      ),
    [frames],
  );

  // Build pie slices
  const pieSlices = useMemo<PieSlice[]>(() => {
    let angle = 0;
    return Object.entries(typeCounts)
      .sort((a, b) => b[1] - a[1])
      .map(([type, count]) => {
        const span = (count / totalFrames) * 360;
        const slice: PieSlice = {
          type,
          count,
          color: frameColor(type),
          startAngle: angle,
          endAngle: angle + span,
        };
        angle += span;
        return slice;
      });
  }, [typeCounts, totalFrames]);

  const totalBytes = useMemo(
    () => frames.reduce((s, f) => s + f.size, 0),
    [frames],
  );
  const avgSize = totalFrames > 0 ? totalBytes / totalFrames : 0;
  const maxFrame = useMemo(
    () =>
      frames.length > 0
        ? frames.reduce((a, b) => (a.size > b.size ? a : b))
        : null,
    [frames],
  );
  const minFrame = useMemo(
    () =>
      frames.length > 0
        ? frames.reduce((a, b) => (a.size < b.size ? a : b))
        : null,
    [frames],
  );

  return (
    <div className="syntax-tab-content stats-tab">
      {/* ── Current frame ── */}
      {currentFrame && (
        <div className="stats-section">
          <div className="stats-header">Current Frame</div>
          <div className="stats-grid">
            <span className="stats-label">Index:</span>
            <span className="stats-value">{currentFrame.frame_index}</span>
            <span className="stats-label">Type:</span>
            <span
              className="stats-value"
              style={{ color: frameColor(currentFrame.frame_type) }}
            >
              {currentFrame.frame_type}
            </span>
            <span className="stats-label">Size:</span>
            <span className="stats-value">
              {(currentFrame.size / 1024).toFixed(2)} KB
            </span>
            {currentFrame.temporal_id !== undefined && (
              <>
                <span className="stats-label">Temporal ID:</span>
                <span className="stats-value">{currentFrame.temporal_id}</span>
              </>
            )}
            {currentFrame.display_order !== undefined && (
              <>
                <span className="stats-label">Display Order:</span>
                <span className="stats-value">
                  {currentFrame.display_order}
                </span>
              </>
            )}
          </div>
        </div>
      )}

      {/* ── Frame type distribution ── */}
      {totalFrames > 0 && (
        <div className="stats-section">
          <div className="stats-header">Frame Type Distribution</div>
          <div className="stats-chart-row">
            <PieChart slices={pieSlices} total={totalFrames} size={96} />
            <div className="stats-legend">
              {pieSlices.map((s) => (
                <div key={s.type} className="stats-legend-item">
                  <span
                    className="stats-legend-dot"
                    style={{ background: s.color }}
                  />
                  <span className="stats-legend-type">{s.type}</span>
                  <span className="stats-legend-count">{s.count}</span>
                  <span className="stats-legend-pct">
                    {((s.count / totalFrames) * 100).toFixed(1)}%
                  </span>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}

      {/* ── Frame sizes sparkline ── */}
      {totalFrames > 0 && (
        <div className="stats-section">
          <div className="stats-header">Frame Sizes</div>
          <div className="stats-chart-bar">
            <BarChart
              frames={frames}
              currentFrameIndex={currentFrame?.frame_index ?? -1}
              height={54}
            />
          </div>
          <div className="stats-chart-labels">
            <span>0</span>
            <span>{totalFrames - 1}</span>
          </div>
        </div>
      )}

      {/* ── Stream statistics ── */}
      <div className="stats-section">
        <div className="stats-header">Stream Statistics</div>
        <div className="stats-grid">
          <span className="stats-label">Total Frames:</span>
          <span className="stats-value">{totalFrames}</span>
          <span className="stats-label">Total Size:</span>
          <span className="stats-value">
            {totalBytes >= 1024 * 1024
              ? `${(totalBytes / 1024 / 1024).toFixed(2)} MB`
              : `${(totalBytes / 1024).toFixed(2)} KB`}
          </span>
          <span className="stats-label">Avg Size:</span>
          <span className="stats-value">{(avgSize / 1024).toFixed(2)} KB</span>
          {maxFrame && (
            <>
              <span className="stats-label">Max Frame:</span>
              <span className="stats-value">
                #{maxFrame.frame_index} ({(maxFrame.size / 1024).toFixed(2)} KB)
              </span>
            </>
          )}
          {minFrame && (
            <>
              <span className="stats-label">Min Frame:</span>
              <span className="stats-value">
                #{minFrame.frame_index} ({(minFrame.size / 1024).toFixed(2)} KB)
              </span>
            </>
          )}
          {Object.entries(typeCounts)
            .sort((a, b) => b[1] - a[1])
            .map(([type, count]) => (
              <React.Fragment key={type}>
                <span className="stats-label">{type} Frames:</span>
                <span
                  className="stats-value"
                  style={{ color: frameColor(type) }}
                >
                  {count}
                </span>
              </React.Fragment>
            ))}
        </div>
      </div>
    </div>
  );
});
