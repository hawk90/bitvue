/**
 * Deblocking View Component
 *
 * Visualizes deblocking filter information showing:
 * - Block boundary strength visualization
 * - Filter on/off decisions per boundary
 * - Deblocking edge metadata
 * - Codec-specific deblocking parameters
 */

import { memo, useEffect, useMemo, useState } from "react";
import type { FrameInfo } from "../../../types/video";
import type { YUVFrame } from "../../../types/yuv";
import { Colorspace } from "../../../types/yuv";
import { yuvToImageData } from "../../../utils/yuv";
import { getDeblockingAnalysis } from "../../../services/electronBridgeService";
import { createLogger } from "../../../utils/logger";
import "./DeblockingView.css";

const logger = createLogger("DeblockingView");

interface DeblockingViewProps {
  frame: FrameInfo | null;
  width: number;
  height: number;
  codec?: string;
  /** Decoded YUV pixels for the same frame -- drawn as the boundary visualization's background so
   *  boundaries are shown over the real picture instead of a flat theme-color rect. Optional: the
   *  visualization still renders (just without a picture underneath) if decode hasn't landed yet. */
  yuvData?: YUVFrame;
  colorspace?: Colorspace;
}

/** Converts `yuvData` to a data URL for the SVG `<image>` background -- `yuvToImageData` returns a
 *  plain `ImageData`, which SVG can't reference directly, so this draws it onto a scratch canvas
 *  once and reuses the canvas across calls (this view's frames are always sized within the video's
 *  native resolution, resized as needed). */
let scratchCanvas: HTMLCanvasElement | null = null;
function yuvFrameToDataUrl(
  frame: YUVFrame,
  colorspace: Colorspace,
): string | null {
  const imageData = yuvToImageData(frame, colorspace);
  if (!scratchCanvas) {
    scratchCanvas = document.createElement("canvas");
  }
  scratchCanvas.width = imageData.width;
  scratchCanvas.height = imageData.height;
  const ctx = scratchCanvas.getContext("2d");
  if (!ctx) return null;
  ctx.putImageData(imageData, 0, 0);
  return scratchCanvas.toDataURL();
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

/** Real AV1 loop_filter_params() fields (spec 5.9.11) -- level[0]/[1] are luma
 *  vertical/horizontal edges, level[2]/[3] are U/V (only set when num_planes > 1). */
interface DeblockingParams {
  level: [number, number, number, number];
  sharpness: number;
  deltaEnabled: boolean;
  refDeltas: number[];
  modeDeltas: number[];
}

const DEFAULT_PARAMS: DeblockingParams = {
  level: [0, 0, 0, 0],
  sharpness: 0,
  deltaEnabled: false,
  refDeltas: [],
  modeDeltas: [],
};

export const DeblockingView = memo(function DeblockingView({
  frame,
  width,
  height,
  codec = "Unknown",
  yuvData,
  colorspace = Colorspace.BT709,
}: DeblockingViewProps) {
  const backgroundImageUrl = useMemo(() => {
    if (!yuvData) return null;
    try {
      return yuvFrameToDataUrl(yuvData, colorspace);
    } catch (err) {
      logger.warn("Failed to render YUV background for deblocking view:", err);
      return null;
    }
  }, [yuvData, colorspace]);

  // `codec` is `activeCodec` as-is from the sidecar (real value is lowercase, e.g. "av1"), not the
  // uppercase display convention the "Codec-Specific Notes" section below compares against --
  // same case-sensitivity gap as useAv1Features.ts's `activeCodec === "AV1"` bug, just lower
  // impact (one explanatory paragraph silently never showing, not a whole missing overlay).
  const normalizedCodec = codec.toUpperCase();
  const [boundaries, setBoundaries] = useState<BoundaryEdge[]>([]);
  const [params, setParams] = useState<DeblockingParams>(DEFAULT_PARAMS);
  const [stats, setStats] = useState({
    totalBoundaries: 0,
    filteredBoundaries: 0,
    strongBoundaries: 0,
    weakBoundaries: 0,
  });

  useEffect(() => {
    if (!frame) {
      setBoundaries([]);
      return;
    }

    let cancelled = false;

    getDeblockingAnalysis(frame.frame_index)
      .then((data) => {
        if (cancelled) return;
        setBoundaries(
          data.edges.map((e) => ({
            x: e.x,
            y: e.y,
            length: e.length,
            orientation: e.orientation,
            strength: e.strength,
            filtered: e.filtered,
            bs: e.boundary_strength,
          })),
        );
        setParams({
          level: data.params.level,
          sharpness: data.params.sharpness,
          deltaEnabled: data.params.delta_enabled,
          refDeltas: data.params.ref_deltas,
          modeDeltas: data.params.mode_deltas,
        });
        setStats({
          totalBoundaries: data.stats.total_edges,
          filteredBoundaries: data.stats.filtered_edges,
          strongBoundaries: data.stats.strong_edges,
          weakBoundaries: data.stats.weak_edges,
        });
      })
      .catch((err) => {
        if (cancelled) return;
        logger.warn(
          "Failed to fetch deblocking analysis for frame",
          frame.frame_index,
          err,
        );
        setBoundaries([]);
      });

    return () => {
      cancelled = true;
    };
  }, [frame]);

  const getEdgeColor = (edge: BoundaryEdge) => {
    if (!edge.filtered) {
      return "rgba(128, 128, 128, 0.2)";
    }

    // AV1 boundary strength is 0/1/2 (spec 7.14.2) -- 2 means an intra edge, 1 means an inter
    // edge with coded residual, differing references, or a large MV difference.
    const intensity = Math.min(edge.strength / 63, 1);
    if (edge.bs >= 2) {
      // Strong (intra) boundary - red to yellow
      return `rgba(255, ${Math.floor(200 * (1 - intensity))}, 0, ${0.5 + intensity * 0.5})`;
    } else {
      // Weak (inter) boundary - blue to cyan
      return `rgba(0, ${Math.floor(200 * intensity)}, 255, ${0.3 + intensity * 0.5})`;
    }
  };

  const getEdgeWidth = (edge: BoundaryEdge) => {
    if (!edge.filtered) return 0.5;
    return 0.5 + (edge.strength / 63) * 2;
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
          <span className="deblocking-stat-label">Strong (BS 2, intra):</span>
          <span className="deblocking-stat-value deblocking-strong">
            {stats.strongBoundaries.toLocaleString()}
          </span>
        </div>
        <div className="deblocking-stat-item">
          <span className="deblocking-stat-label">Weak (BS 1, inter):</span>
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
        <h4>Loop Filter Parameters</h4>
        <div className="deblocking-params-grid">
          <div className="deblocking-param-item">
            <span className="deblocking-param-label">Level (Y vert/horz):</span>
            <span className="deblocking-param-value">
              {params.level[0]} / {params.level[1]}
            </span>
          </div>
          <div className="deblocking-param-item">
            <span className="deblocking-param-label">Level (U / V):</span>
            <span className="deblocking-param-value">
              {params.level[2]} / {params.level[3]}
            </span>
          </div>
          <div className="deblocking-param-item">
            <span className="deblocking-param-label">Sharpness:</span>
            <span className="deblocking-param-value">{params.sharpness}</span>
          </div>
          <div className="deblocking-param-item">
            <span className="deblocking-param-label">Ref/Mode Deltas:</span>
            <span className="deblocking-param-value">
              {params.deltaEnabled ? "Enabled" : "Disabled"}
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
            {/* Background: the real decoded picture, so boundaries are shown over actual content
                instead of a flat theme-color rect -- falls back to the rect while decode hasn't
                landed yet (or on a codec this backend doesn't decode). */}
            {backgroundImageUrl ? (
              <image
                href={backgroundImageUrl}
                width={width}
                height={height}
                preserveAspectRatio="xMidYMid slice"
              />
            ) : (
              <rect
                width={width}
                height={height}
                fill="var(--bitvue-bg-primary)"
              />
            )}

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
            <span>Strong Boundary (BS 2, intra)</span>
          </div>
          <div className="deblocking-legend-item">
            <div
              className="deblocking-legend-box deblocking-weak"
              style={{
                background:
                  "linear-gradient(to right, rgba(0,200,255,0.3), rgba(0,0,255,0.8))",
              }}
            ></div>
            <span>Weak Boundary (BS 1, inter)</span>
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
          {normalizedCodec === "AV1" && (
            <p>
              AV1 uses loop restoration filters including deblocking, CDEF, and
              loop restoration. Deblocking is applied to all block boundaries.
            </p>
          )}
          {normalizedCodec === "HEVC" && (
            <p>
              HEVC deblocking filter operates on 8x8 block boundaries. Boundary
              strength (BS) depends on prediction mode, motion vectors, and
              reference indices.
            </p>
          )}
          {normalizedCodec === "VVC" && (
            <p>
              VVC includes enhanced deblocking with adaptive filter strength and
              supports both luma and chroma filtering.
            </p>
          )}
          {normalizedCodec === "AVC" && (
            <p>
              H.264/AVC deblocking filter operates on 4x4 block boundaries with
              adaptive strength based on QP and boundary conditions.
            </p>
          )}
          {normalizedCodec === "VP9" && (
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
