/**
 * Deblocking View Component
 *
 * Visualizes the loop filter of a frame (AV1: spec 7.14):
 * - luma transform edges drawn over the picture, colored by the largest filter each allows
 * - which edges are filtered at all
 * - the frame's loop filter parameters
 */

import { memo, useEffect, useMemo, useState } from "react";
import { useBridgeQuery } from "../../../hooks/useBridgeQuery";
import {
  deblockingAnalysisFromWire,
  type DeblockingAnalysis,
  type DeblockingBoundaryEdge,
  type DeblockingFilterParams,
  type DeblockingFilterStats,
} from "../../../services/bridge/frameAnalysis";
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

type FilterSize = 4 | 8 | 16;

/** Edge colors: the largest filter an edge allows (green 16, yellow 8, orange 4 samples), and red
 *  for an edge the loop filter considers but does not filter (level 0, or a transform edge inside a
 *  skipped inter block). The filter actually applied to each sample line also depends on the
 *  picture's content, so these are upper bounds. */
export const FILTER_SIZE_COLORS: Record<FilterSize, string> = {
  16: "#2da44e",
  8: "#d4a717",
  4: "#e07b39",
};
export const UNFILTERED_EDGE_COLOR = "#e03131";

type BoundaryEdge = DeblockingBoundaryEdge;
type DeblockingParams = DeblockingFilterParams;

const DEFAULT_PARAMS: DeblockingParams = {
  level: [0, 0, 0, 0],
  sharpness: 0,
  deltaEnabled: false,
  refDeltas: [],
  modeDeltas: [],
};

const EMPTY_STATS: DeblockingFilterStats = {
  edgeSegments: 0,
  filteredSegments: 0,
  filtered16: 0,
  filtered8: 0,
  filtered4: 0,
  unfilteredSegments: 0,
};

const NO_EDGES: BoundaryEdge[] = [];

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
  const { data } = useBridgeQuery(
    async () =>
      deblockingAnalysisFromWire(
        await getDeblockingAnalysis(frame!.frame_index),
      ),
    [frame],
    {
      enabled: frame !== null,
      onError: (err) =>
        logger.warn(
          "Failed to fetch deblocking analysis for frame",
          frame?.frame_index,
          err,
        ),
    },
  );
  const boundaries = data?.edges ?? NO_EDGES;
  // Only the edge list is cleared when there is no frame or a request fails; the parameters and
  // statistics keep the last successful answer (this view's behavior before useBridgeQuery).
  const [{ params, stats }, setLastGood] = useState<
    Pick<DeblockingAnalysis, "params" | "stats">
  >({ params: DEFAULT_PARAMS, stats: EMPTY_STATS });
  useEffect(() => {
    if (data) setLastGood({ params: data.params, stats: data.stats });
  }, [data]);

  const getEdgeColor = (edge: BoundaryEdge) =>
    edge.filtered ? FILTER_SIZE_COLORS[edge.filterSize] : UNFILTERED_EDGE_COLOR;

  // One-sample-thick lines for every edge, like the reference tool; an unfiltered edge is
  // fainter so it does not drown the filtered ones.
  const getEdgeOpacity = (edge: BoundaryEdge) => (edge.filtered ? 0.9 : 0.45);

  if (!frame) {
    return (
      <div className="deblocking-view deblocking-view-empty">
        <p>No frame selected</p>
      </div>
    );
  }

  const edgeTitle = (edge: BoundaryEdge) =>
    `${edge.orientation === "vertical" ? "Vertical" : "Horizontal"} edge (${edge.x}, ${edge.y})\n` +
    `Largest filter: ${edge.filterSize} samples\n` +
    `Level: ${edge.level}\n` +
    `Filtered: ${edge.filtered ? "Yes" : "No"}`;

  const pct = (n: number) =>
    stats.edgeSegments > 0 ? ((n / stats.edgeSegments) * 100).toFixed(1) : "0";

  return (
    <div className="deblocking-view">
      <div className="deblocking-header">
        <h3>Loop Filter</h3>
        <div className="deblocking-frame-info">
          <span>Frame {frame.frame_index}</span>
          <span className={frame.frame_type.toLowerCase()}>
            {frame.frame_type}
          </span>
          <span className="codec-badge">{codec}</span>
        </div>
      </div>

      {/* Boundary Visualization: first, so it is never pushed below the fold by the numbers */}
      <div className="deblocking-visualization">
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

            {boundaries.map((edge, index) => (
              <line
                key={`${edge.orientation[0]}-${index}`}
                x1={edge.x}
                y1={edge.y}
                x2={
                  edge.orientation === "vertical"
                    ? edge.x
                    : edge.x + edge.length
                }
                y2={
                  edge.orientation === "vertical"
                    ? edge.y + edge.length
                    : edge.y
                }
                stroke={getEdgeColor(edge)}
                strokeOpacity={getEdgeOpacity(edge)}
                strokeWidth={1}
                shapeRendering="crispEdges"
              >
                <title>{edgeTitle(edge)}</title>
              </line>
            ))}
          </svg>
        </div>

        <div className="deblocking-legend">
          {([16, 8, 4] as const).map((size) => (
            <div className="deblocking-legend-item" key={size}>
              <div
                className="deblocking-legend-box"
                style={{ background: FILTER_SIZE_COLORS[size] }}
              ></div>
              <span>Up to {size}-sample filter</span>
            </div>
          ))}
          <div className="deblocking-legend-item">
            <div
              className="deblocking-legend-box"
              style={{ background: UNFILTERED_EDGE_COLOR }}
            ></div>
            <span>Considered, not filtered</span>
          </div>
        </div>
        <p className="deblocking-note">
          Luma transform edges. Colors show the largest filter an edge allows;
          the filter applied to each sample line also depends on the picture
          content.
        </p>
      </div>

      {/* Statistics Panel (counts in 4-sample edge segments) */}
      <div className="deblocking-stats">
        <div className="deblocking-stat-item">
          <span className="deblocking-stat-label">Edge segments:</span>
          <span className="deblocking-stat-value">
            {stats.edgeSegments.toLocaleString()}
          </span>
        </div>
        <div className="deblocking-stat-item">
          <span className="deblocking-stat-label">Filtered:</span>
          <span className="deblocking-stat-value">
            {stats.filteredSegments.toLocaleString()} (
            {pct(stats.filteredSegments)}%)
          </span>
        </div>
        <div className="deblocking-stat-item">
          <span className="deblocking-stat-label">Up to 16 samples:</span>
          <span className="deblocking-stat-value">
            {stats.filtered16.toLocaleString()}
          </span>
        </div>
        <div className="deblocking-stat-item">
          <span className="deblocking-stat-label">Up to 8 samples:</span>
          <span className="deblocking-stat-value">
            {stats.filtered8.toLocaleString()}
          </span>
        </div>
        <div className="deblocking-stat-item">
          <span className="deblocking-stat-label">Up to 4 samples:</span>
          <span className="deblocking-stat-value">
            {stats.filtered4.toLocaleString()}
          </span>
        </div>
        <div className="deblocking-stat-item">
          <span className="deblocking-stat-label">Not filtered:</span>
          <span className="deblocking-stat-value">
            {stats.unfilteredSegments.toLocaleString()}
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
