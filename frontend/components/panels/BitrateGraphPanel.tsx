/**
 * Bitrate Graph Panel
 *
 * Displays frame sizes and running bitrate over time.
 * Data is loaded from the currently open stream via the bridge's get_frames_chunk.
 */

import { useState, useMemo } from "react";
import { memo } from "react";
import { useGraphFrames } from "../../hooks/useGraphFrames";
import "./BitrateGraphPanel.css";

const FRAME_TYPE_COLORS: Record<string, string> = {
  I: "#3b82f6",
  KEY: "#3b82f6",
  P: "#10b981",
  INTER: "#10b981",
  B: "#f59e0b",
  SKIP: "#6b7280",
  UNKNOWN: "#9ca3af",
};

function getFrameColor(frameType: string): string {
  return FRAME_TYPE_COLORS[frameType] ?? FRAME_TYPE_COLORS.UNKNOWN;
}

// Not mounted in App.tsx yet (#567). Frames come from the sidecar's `get_frames_chunk` through
// `useGraphFrames`; before the stream is indexed (or for a codec the indexer skips) the list is
// empty and the panel shows "No stream loaded".
export const BitrateGraphPanel = memo(function BitrateGraphPanel() {
  const { frames, loading, error, reload: loadFrames } = useGraphFrames();
  const [hoveredIdx, setHoveredIdx] = useState<number | null>(null);
  const [viewMode, setViewMode] = useState<"size" | "bitrate">("size");

  // Compute running average bitrate (bytes per frame, smoothed over 10 frames)
  const smoothedSizes = useMemo(() => {
    if (frames.length === 0) return [];
    const window = 10;
    return frames.map((_, i) => {
      const start = Math.max(0, i - window);
      const end = Math.min(frames.length, i + window + 1);
      const sum = frames.slice(start, end).reduce((acc, f) => acc + f.size, 0);
      return sum / (end - start);
    });
  }, [frames]);

  const maxSize = useMemo(
    () => Math.max(...frames.map((f) => f.size), 1),
    [frames],
  );

  const totalSize = useMemo(
    () => frames.reduce((acc, f) => acc + f.size, 0),
    [frames],
  );

  const avgSize = frames.length > 0 ? totalSize / frames.length : 0;

  const iFrameCount = frames.filter(
    (f) => f.frame_type === "I" || f.frame_type === "KEY",
  ).length;
  const pFrameCount = frames.filter(
    (f) => f.frame_type === "P" || f.frame_type === "INTER",
  ).length;
  const bFrameCount = frames.filter((f) => f.frame_type === "B").length;

  const displayFrames = frames.slice(0, 500); // cap render to 500 bars

  if (loading) {
    return (
      <div className="bitrate-panel">
        <div className="bitrate-loading">Loading frame data...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bitrate-panel">
        <div className="bitrate-error">{error}</div>
        <button className="bitrate-reload-btn" onClick={loadFrames}>
          Retry
        </button>
      </div>
    );
  }

  if (frames.length === 0) {
    return (
      <div className="bitrate-panel">
        <div className="bitrate-empty">No stream loaded</div>
      </div>
    );
  }

  return (
    <div className="bitrate-panel">
      <div className="bitrate-header">
        <h3>Bitrate Graph</h3>
        <div className="bitrate-controls">
          <button
            className={`bitrate-mode-btn${viewMode === "size" ? " active" : ""}`}
            onClick={() => setViewMode("size")}
          >
            Frame Size
          </button>
          <button
            className={`bitrate-mode-btn${viewMode === "bitrate" ? " active" : ""}`}
            onClick={() => setViewMode("bitrate")}
          >
            Smoothed
          </button>
          <button className="bitrate-reload-btn" onClick={loadFrames}>
            Reload
          </button>
        </div>
      </div>

      {/* Stats Row */}
      <div className="bitrate-stats">
        <div className="bitrate-stat">
          <span className="stat-label">Frames</span>
          <span className="stat-value">{frames.length}</span>
        </div>
        <div className="bitrate-stat">
          <span className="stat-label">Total</span>
          <span className="stat-value">{(totalSize / 1024).toFixed(0)} KB</span>
        </div>
        <div className="bitrate-stat">
          <span className="stat-label">Avg</span>
          <span className="stat-value">{(avgSize / 1024).toFixed(1)} KB</span>
        </div>
        <div className="bitrate-stat" style={{ color: "#3b82f6" }}>
          <span className="stat-label">I</span>
          <span className="stat-value">{iFrameCount}</span>
        </div>
        <div className="bitrate-stat" style={{ color: "#10b981" }}>
          <span className="stat-label">P</span>
          <span className="stat-value">{pFrameCount}</span>
        </div>
        <div className="bitrate-stat" style={{ color: "#f59e0b" }}>
          <span className="stat-label">B</span>
          <span className="stat-value">{bFrameCount}</span>
        </div>
      </div>

      {/* Bar Chart */}
      <div className="bitrate-chart-container">
        <div className="bitrate-chart">
          {displayFrames.map((frame, i) => {
            const h =
              viewMode === "size"
                ? (frame.size / maxSize) * 100
                : (smoothedSizes[i] / maxSize) * 100;
            const color = getFrameColor(frame.frame_type);
            const isHovered = hoveredIdx === i;
            return (
              <div
                key={i}
                className={`bitrate-bar${isHovered ? " hovered" : ""}`}
                style={{ height: `${Math.max(h, 1)}%`, backgroundColor: color }}
                onMouseEnter={() => setHoveredIdx(i)}
                onMouseLeave={() => setHoveredIdx(null)}
              />
            );
          })}
        </div>
        {frames.length > 500 && (
          <div className="bitrate-truncated">
            Showing first 500 of {frames.length} frames
          </div>
        )}
      </div>

      {/* Tooltip */}
      {hoveredIdx !== null && displayFrames[hoveredIdx] && (
        <div className="bitrate-tooltip">
          <span>
            Frame {displayFrames[hoveredIdx].frame_index} (
            {displayFrames[hoveredIdx].frame_type})
          </span>
          <span>{(displayFrames[hoveredIdx].size / 1024).toFixed(2)} KB</span>
          {displayFrames[hoveredIdx].pts !== null && (
            <span>PTS: {displayFrames[hoveredIdx].pts}</span>
          )}
        </div>
      )}

      {/* Legend */}
      <div className="bitrate-legend">
        {Object.entries(FRAME_TYPE_COLORS)
          .filter(([k]) => !["INTER", "KEY", "UNKNOWN"].includes(k))
          .map(([type, color]) => (
            <div key={type} className="bitrate-legend-item">
              <div
                className="bitrate-legend-swatch"
                style={{ backgroundColor: color }}
              />
              <span>{type}</span>
            </div>
          ))}
      </div>
    </div>
  );
});
