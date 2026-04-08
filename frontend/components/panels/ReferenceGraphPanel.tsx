/**
 * Reference Graph Panel
 *
 * Displays GOP structure and frame dependency visualization.
 * Shows I/P/B frame layout, reference chains, and GOP boundaries.
 */

import { useState, useEffect, useCallback, useMemo } from "react";
import { memo } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./ReferenceGraphPanel.css";

interface FrameData {
  frame_index: number;
  frame_type: string;
  size: number;
  pts: number | null;
  ref_frames: number[] | null;
  temporal_id: number | null;
}

const FRAME_COLOR: Record<string, string> = {
  I: "#3b82f6",
  KEY: "#3b82f6",
  P: "#10b981",
  INTER: "#10b981",
  B: "#f59e0b",
  SKIP: "#6b7280",
};

function frameColor(type: string): string {
  return FRAME_COLOR[type] ?? "#9ca3af";
}

function frameLabel(type: string): string {
  if (type === "KEY") return "I";
  if (type === "INTER") return "P";
  return type || "?";
}

interface GopInfo {
  startIdx: number;
  frames: FrameData[];
}

export const ReferenceGraphPanel = memo(function ReferenceGraphPanel() {
  const [frames, setFrames] = useState<FrameData[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedFrame, setSelectedFrame] = useState<number | null>(null);
  const [visibleGop, setVisibleGop] = useState(0);

  const loadFrames = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await invoke<FrameData[]>("get_frames");
      setFrames(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadFrames();
  }, [loadFrames]);

  // Split frames into GOPs (groups starting with I/KEY frames)
  const gops: GopInfo[] = useMemo(() => {
    const result: GopInfo[] = [];
    let current: FrameData[] = [];
    for (const frame of frames) {
      const isIntra = frame.frame_type === "I" || frame.frame_type === "KEY";
      if (isIntra && current.length > 0) {
        result.push({ startIdx: current[0].frame_index, frames: current });
        current = [];
      }
      current.push(frame);
    }
    if (current.length > 0) {
      result.push({ startIdx: current[0].frame_index, frames: current });
    }
    return result;
  }, [frames]);

  const currentGop = gops[visibleGop] ?? null;

  // Statistics
  const totalFrames = frames.length;
  const iCount = frames.filter(
    (f) => f.frame_type === "I" || f.frame_type === "KEY",
  ).length;
  const avgGopSize =
    gops.length > 0
      ? Math.round(gops.reduce((a, g) => a + g.frames.length, 0) / gops.length)
      : 0;

  if (loading) {
    return (
      <div className="refgraph-panel">
        <div className="refgraph-center">Loading...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="refgraph-panel">
        <div className="refgraph-error">{error}</div>
        <button onClick={loadFrames} className="refgraph-btn">
          Retry
        </button>
      </div>
    );
  }

  if (frames.length === 0) {
    return (
      <div className="refgraph-panel">
        <div className="refgraph-center">No stream loaded</div>
      </div>
    );
  }

  return (
    <div className="refgraph-panel">
      <div className="refgraph-header">
        <h3>Reference Graph</h3>
        <button onClick={loadFrames} className="refgraph-btn">
          Reload
        </button>
      </div>

      {/* Summary stats */}
      <div className="refgraph-stats">
        <div className="rg-stat">
          <span className="rg-stat-label">Frames</span>
          <span className="rg-stat-value">{totalFrames}</span>
        </div>
        <div className="rg-stat">
          <span className="rg-stat-label">GOPs</span>
          <span className="rg-stat-value">{gops.length}</span>
        </div>
        <div className="rg-stat">
          <span className="rg-stat-label">I-Frames</span>
          <span className="rg-stat-value" style={{ color: "#3b82f6" }}>
            {iCount}
          </span>
        </div>
        <div className="rg-stat">
          <span className="rg-stat-label">Avg GOP</span>
          <span className="rg-stat-value">{avgGopSize}</span>
        </div>
      </div>

      {/* GOP navigation */}
      {gops.length > 1 && (
        <div className="refgraph-nav">
          <button
            className="refgraph-btn"
            disabled={visibleGop === 0}
            onClick={() => setVisibleGop((v) => Math.max(0, v - 1))}
          >
            ‹
          </button>
          <span className="refgraph-nav-label">
            GOP {visibleGop + 1} / {gops.length}
            {currentGop && (
              <span className="refgraph-nav-range">
                {" "}
                (frames {currentGop.startIdx}–
                {currentGop.startIdx + currentGop.frames.length - 1})
              </span>
            )}
          </span>
          <button
            className="refgraph-btn"
            disabled={visibleGop === gops.length - 1}
            onClick={() =>
              setVisibleGop((v) => Math.min(gops.length - 1, v + 1))
            }
          >
            ›
          </button>
        </div>
      )}

      {/* GOP frame display */}
      {currentGop && (
        <div className="refgraph-gop">
          <div className="refgraph-frames">
            {currentGop.frames.map((frame) => {
              const color = frameColor(frame.frame_type);
              const isSelected = selectedFrame === frame.frame_index;
              return (
                <div
                  key={frame.frame_index}
                  className={`rg-frame${isSelected ? " selected" : ""}`}
                  style={{ borderColor: color }}
                  onClick={() =>
                    setSelectedFrame(isSelected ? null : frame.frame_index)
                  }
                >
                  <div
                    className="rg-frame-header"
                    style={{ backgroundColor: color }}
                  >
                    {frameLabel(frame.frame_type)}
                  </div>
                  <div className="rg-frame-idx">{frame.frame_index}</div>
                  <div className="rg-frame-size">
                    {(frame.size / 1024).toFixed(1)}K
                  </div>
                  {frame.temporal_id !== null && frame.temporal_id > 0 && (
                    <div className="rg-frame-tid">T{frame.temporal_id}</div>
                  )}
                </div>
              );
            })}
          </div>

          {/* Selected frame detail */}
          {selectedFrame !== null &&
            (() => {
              const f = currentGop.frames.find(
                (fr) => fr.frame_index === selectedFrame,
              );
              if (!f) return null;
              return (
                <div className="rg-detail">
                  <div className="rg-detail-row">
                    <span>Index</span>
                    <span>{f.frame_index}</span>
                  </div>
                  <div className="rg-detail-row">
                    <span>Type</span>
                    <span style={{ color: frameColor(f.frame_type) }}>
                      {f.frame_type}
                    </span>
                  </div>
                  <div className="rg-detail-row">
                    <span>Size</span>
                    <span>{(f.size / 1024).toFixed(2)} KB</span>
                  </div>
                  {f.pts !== null && (
                    <div className="rg-detail-row">
                      <span>PTS</span>
                      <span>{f.pts}</span>
                    </div>
                  )}
                  {f.ref_frames && f.ref_frames.length > 0 && (
                    <div className="rg-detail-row">
                      <span>Refs</span>
                      <span>{f.ref_frames.join(", ")}</span>
                    </div>
                  )}
                </div>
              );
            })()}
        </div>
      )}

      {/* Mini overview: all frames as colored dots */}
      <div className="refgraph-overview">
        {frames.slice(0, 300).map((frame, i) => (
          <div
            key={i}
            className="rg-dot"
            style={{ backgroundColor: frameColor(frame.frame_type) }}
            title={`Frame ${frame.frame_index} (${frame.frame_type})`}
          />
        ))}
        {frames.length > 300 && (
          <span className="rg-dot-more">+{frames.length - 300}</span>
        )}
      </div>
    </div>
  );
});
