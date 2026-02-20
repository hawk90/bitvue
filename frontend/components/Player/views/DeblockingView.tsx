/**
 * Deblocking View Component
 *
 * Visualizes deblocking filter information showing:
 * - Block boundary strength visualization
 * - Filter on/off decisions per boundary
 * - Deblocking edge metadata
 * - Codec-specific deblocking parameters
 */

import { memo, useMemo, useEffect, useState } from "react";
import type { FrameInfo } from "../../../types/video";

interface DeblockingViewProps {
  frame: FrameInfo | null;
  width: number;
  height: number;
  codec?: string;
}

interface BoundaryEdge {
  x: number;
  y: number;
  length: number;
  orientation: "vertical" | "horizontal";
  strength: number;
  filtered: boolean;
  bs: number; // Boundary strength
}

interface DeblockingParams {
  betaOffset: number;
  tcOffset: number;
  filterStrength: number;
  chromaEdge: boolean;
}

const CODEC_DEFAULT_PARAMS: Record<string, DeblockingParams> = {
  AV1: { betaOffset: 0, tcOffset: 0, filterStrength: 1, chromaEdge: true },
  HEVC: { betaOffset: 0, tcOffset: 0, filterStrength: 1, chromaEdge: true },
  VVC: { betaOffset: 0, tcOffset: 0, filterStrength: 1, chromaEdge: true },
  AVC: { betaOffset: 0, tcOffset: 0, filterStrength: 1, chromaEdge: true },
  VP9: { betaOffset: 0, tcOffset: 0, filterStrength: 1, chromaEdge: false },
  AV3: { betaOffset: 0, tcOffset: 0, filterStrength: 1, chromaEdge: true },
};

export const DeblockingView = memo(function DeblockingView({
  frame,
  width,
  height,
  codec = "Unknown",
}: DeblockingViewProps) {
  const [boundaries, setBoundaries] = useState<BoundaryEdge[]>([]);
  const [params, setParams] = useState<DeblockingParams>(
    CODEC_DEFAULT_PARAMS[codec] || CODEC_DEFAULT_PARAMS["AV1"],
  );
  const [stats, setStats] = useState({
    totalBoundaries: 0,
    filteredBoundaries: 0,
    strongBoundaries: 0,
    weakBoundaries: 0,
  });

  // Generate mock boundary data
  useEffect(() => {
    if (!frame || width === 0 || height === 0) {
      setBoundaries([]);
      return;
    }

    const blockSize = 8;
    const edges: BoundaryEdge[] = [];
    const qp = 26; // Default QP value

    // Generate vertical edges
    for (let y = 0; y < height; y += blockSize) {
      for (let x = blockSize; x < width; x += blockSize) {
        const bs = Math.floor(Math.random() * 5); // Boundary strength 0-4
        const strength = bs > 0 ? (bs / 4) * (4 + Math.random()) : 0;
        const filtered = bs > 0 && Math.random() > 0.3;

        edges.push({
          x,
          y,
          length: blockSize,
          orientation: "vertical",
          strength,
          filtered,
          bs,
        });
      }
    }

    // Generate horizontal edges
    for (let y = blockSize; y < height; y += blockSize) {
      for (let x = 0; x < width; x += blockSize) {
        const bs = Math.floor(Math.random() * 5);
        const strength = bs > 0 ? (bs / 4) * (4 + Math.random()) : 0;
        const filtered = bs > 0 && Math.random() > 0.3;

        edges.push({
          x,
          y,
          length: blockSize,
          orientation: "horizontal",
          strength,
          filtered,
          bs,
        });
      }
    }

    setBoundaries(edges);

    // Calculate statistics
    const filteredCount = edges.filter((e) => e.filtered).length;
    const strongCount = edges.filter((e) => e.bs >= 3).length;
    const weakCount = edges.filter((e) => e.bs > 0 && e.bs < 3).length;

    setStats({
      totalBoundaries: edges.length,
      filteredBoundaries: filteredCount,
      strongBoundaries: strongCount,
      weakBoundaries: weakCount,
    });

    setParams(CODEC_DEFAULT_PARAMS[codec] || CODEC_DEFAULT_PARAMS["AV1"]);
  }, [frame, width, height, codec]);

  const getEdgeColor = (edge: BoundaryEdge) => {
    if (!edge.filtered) {
      return "rgba(128, 128, 128, 0.2)";
    }

    const intensity = edge.strength / 4;
    if (edge.bs >= 3) {
      // Strong boundary - red to yellow
      return `rgba(255, ${Math.floor(200 * (1 - intensity))}, 0, ${0.5 + intensity * 0.5})`;
    } else {
      // Weak boundary - blue to cyan
      return `rgba(0, ${Math.floor(200 * intensity)}, 255, ${0.3 + intensity * 0.5})`;
    }
  };

  const getEdgeWidth = (edge: BoundaryEdge) => {
    if (!edge.filtered) return 0.5;
    return 0.5 + edge.strength * 0.5;
  };

  if (!frame) {
    return (
      <div className="deblocking-view deblocking-view-empty">
        <p>No frame selected</p>
      </div>
    );
  }

  return (
    <div className="deblocking-view">
      <div className="deblocking-header">
        <h3>Deblocking Filter Analysis</h3>
        <div className="deblocking-frame-info">
          <span>Frame {frame.frame_index}</span>
          <span className={frame.frame_type.toLowerCase()}>
            {frame.frame_type}
          </span>
          <span className="codec-badge">{codec}</span>
        </div>
      </div>

      {/* Statistics Panel */}
      <div className="deblocking-stats">
        <div className="deblocking-stat-item">
          <span className="deblocking-stat-label">Total Boundaries:</span>
          <span className="deblocking-stat-value">
            {stats.totalBoundaries.toLocaleString()}
          </span>
        </div>
        <div className="deblocking-stat-item">
          <span className="deblocking-stat-label">Filtered:</span>
          <span className="deblocking-stat-value">
            {stats.filteredBoundaries.toLocaleString()}
          </span>
        </div>
        <div className="deblocking-stat-item">
          <span className="deblocking-stat-label">Strong (BS≥3):</span>
          <span className="deblocking-stat-value deblocking-strong">
            {stats.strongBoundaries.toLocaleString()}
          </span>
        </div>
        <div className="deblocking-stat-item">
          <span className="deblocking-stat-label">Weak (BS 1-2):</span>
          <span className="deblocking-stat-value deblocking-weak">
            {stats.weakBoundaries.toLocaleString()}
          </span>
        </div>
        <div className="deblocking-stat-item">
          <span className="deblocking-stat-label">Filter Rate:</span>
          <span className="deblocking-stat-value">
            {stats.totalBoundaries > 0
              ? (
                  (stats.filteredBoundaries / stats.totalBoundaries) *
                  100
                ).toFixed(1)
              : "0"}
            %
          </span>
        </div>
      </div>

      {/* Deblocking Parameters */}
      <div className="deblocking-params">
        <h4>Deblocking Parameters</h4>
        <div className="deblocking-params-grid">
          <div className="deblocking-param-item">
            <span className="deblocking-param-label">β Offset:</span>
            <span className="deblocking-param-value">{params.betaOffset}</span>
          </div>
          <div className="deblocking-param-item">
            <span className="deblocking-param-label">tc Offset:</span>
            <span className="deblocking-param-value">{params.tcOffset}</span>
          </div>
          <div className="deblocking-param-item">
            <span className="deblocking-param-label">Filter Strength:</span>
            <span className="deblocking-param-value">
              {params.filterStrength}
            </span>
          </div>
          <div className="deblocking-param-item">
            <span className="deblocking-param-label">Chroma Edge:</span>
            <span className="deblocking-param-value">
              {params.chromaEdge ? "Enabled" : "Disabled"}
            </span>
          </div>
        </div>
      </div>

      {/* Boundary Visualization */}
      <div className="deblocking-visualization">
        <h4>Block Boundary Visualization</h4>
        <div className="deblocking-canvas-container">
          <svg
            width={width}
            height={height}
            className="deblocking-canvas"
            viewBox={`0 0 ${width} ${height}`}
            preserveAspectRatio="xMidYMid meet"
          >
            {/* Background */}
            <rect
              width={width}
              height={height}
              fill="var(--bitvue-bg-primary)"
            />

            {/* Draw edges */}
            {boundaries.map((edge, index) => {
              if (edge.orientation === "vertical") {
                return (
                  <line
                    key={`v-${index}`}
                    x1={edge.x}
                    y1={edge.y}
                    x2={edge.x}
                    y2={edge.y + edge.length}
                    stroke={getEdgeColor(edge)}
                    strokeWidth={getEdgeWidth(edge)}
                  >
                    <title>
                      Vertical Edge ({edge.x}, {edge.y}){`\n`}
                      BS: {edge.bs}
                      {`\n`}
                      Strength: {edge.strength.toFixed(2)}
                      {`\n`}
                      Filtered: {edge.filtered ? "Yes" : "No"}
                    </title>
                  </line>
                );
              } else {
                return (
                  <line
                    key={`h-${index}`}
                    x1={edge.x}
                    y1={edge.y}
                    x2={edge.x + edge.length}
                    y2={edge.y}
                    stroke={getEdgeColor(edge)}
                    strokeWidth={getEdgeWidth(edge)}
                  >
                    <title>
                      Horizontal Edge ({edge.x}, {edge.y}){`\n`}
                      BS: {edge.bs}
                      {`\n`}
                      Strength: {edge.strength.toFixed(2)}
                      {`\n`}
                      Filtered: {edge.filtered ? "Yes" : "No"}
                    </title>
                  </line>
                );
              }
            })}
          </svg>
        </div>

        {/* Legend */}
        <div className="deblocking-legend">
          <div className="deblocking-legend-item">
            <div
              className="deblocking-legend-box deblocking-strong"
              style={{
                background:
                  "linear-gradient(to right, rgba(255,200,0,0.5), rgba(255,0,0,1))",
              }}
            ></div>
            <span>Strong Boundary (BS 3-4)</span>
          </div>
          <div className="deblocking-legend-item">
            <div
              className="deblocking-legend-box deblocking-weak"
              style={{
                background:
                  "linear-gradient(to right, rgba(0,200,255,0.3), rgba(0,0,255,0.8))",
              }}
            ></div>
            <span>Weak Boundary (BS 1-2)</span>
          </div>
          <div className="deblocking-legend-item">
            <div
              className="deblocking-legend-box"
              style={{ background: "rgba(128,128,128,0.2)" }}
            ></div>
            <span>Not Filtered</span>
          </div>
        </div>
      </div>

      {/* Codec-Specific Notes */}
      <div className="deblocking-notes">
        <h4>Codec-Specific Notes</h4>
        <div className="deblocking-notes-content">
          {codec === "AV1" && (
            <p>
              AV1 uses loop restoration filters including deblocking, CDEF, and
              loop restoration. Deblocking is applied to all block boundaries.
            </p>
          )}
          {codec === "HEVC" && (
            <p>
              HEVC deblocking filter operates on 8x8 block boundaries. Boundary
              strength (BS) depends on prediction mode, motion vectors, and
              reference indices.
            </p>
          )}
          {codec === "VVC" && (
            <p>
              VVC includes enhanced deblocking with adaptive filter strength and
              supports both luma and chroma filtering.
            </p>
          )}
          {codec === "AVC" && (
            <p>
              H.264/AVC deblocking filter operates on 4x4 block boundaries with
              adaptive strength based on QP and boundary conditions.
            </p>
          )}
          {codec === "VP9" && (
            <p>
              VP9 deblocking filter operates on 8x8 block boundaries for luma
              and 4x4 for chroma (when enabled).
            </p>
          )}
        </div>
      </div>
    </div>
  );
});
