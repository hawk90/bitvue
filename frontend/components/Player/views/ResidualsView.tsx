/**
 * Residuals View Component
 *
 * Visualizes residual (prediction error) information showing:
 * - Residual coefficients heatmap
 * - Coefficient distribution histogram
 * - Energy statistics per block/transform unit
 * - Quantization effects on residuals
 */

import { memo, useMemo, useEffect, useState } from "react";
import type { FrameInfo } from "../../../types/video";
import { getResidualAnalysis } from "../../../services/electronBridgeService";
import { createLogger } from "../../../utils/logger";
import "./ResidualsView.css";

const logger = createLogger("ResidualsView");

interface ResidualsViewProps {
  frame: FrameInfo | null;
  width: number;
  height: number;
  showHeatmap?: boolean;
  showHistogram?: boolean;
}

interface CoefficientStats {
  min: number;
  max: number;
  mean: number;
  variance: number;
  energy: number;
  zeroCount: number;
  nonZeroCount: number;
}

interface BlockResidual {
  x: number;
  y: number;
  width: number;
  height: number;
  energy: number;
  maxCoeff: number;
  nonZeros: number;
}

export const ResidualsView = memo(function ResidualsView({
  frame,
  width,
  height,
  showHeatmap = true,
  showHistogram = true,
}: ResidualsViewProps) {
  const [blockResiduals, setBlockResiduals] = useState<BlockResidual[]>([]);
  const [coefficientStats, setCoefficientStats] =
    useState<CoefficientStats | null>(null);

  useEffect(() => {
    if (!frame) {
      setBlockResiduals([]);
      setCoefficientStats(null);
      return;
    }

    let cancelled = false;

    getResidualAnalysis(frame.frame_index)
      .then((data) => {
        if (cancelled) return;
        setBlockResiduals(
          data.block_residuals.map((b) => ({
            x: b.x,
            y: b.y,
            width: b.width,
            height: b.height,
            energy: b.energy,
            maxCoeff: b.max_coeff,
            nonZeros: b.non_zeros,
          })),
        );
        setCoefficientStats({
          min: data.coefficient_stats.min,
          max: data.coefficient_stats.max,
          mean: data.coefficient_stats.mean,
          variance: data.coefficient_stats.variance,
          energy: data.coefficient_stats.energy,
          zeroCount: data.coefficient_stats.zero_count,
          nonZeroCount: data.coefficient_stats.non_zero_count,
        });
      })
      .catch((err) => {
        if (cancelled) return;
        logger.warn(
          "Failed to fetch residual analysis for frame",
          frame.frame_index,
          err,
        );
        setBlockResiduals([]);
        setCoefficientStats(null);
      });

    return () => {
      cancelled = true;
    };
  }, [frame]);

  const heatmapColors = useMemo(() => {
    if (!blockResiduals.length) return [];
    const maxEnergy = Math.max(...blockResiduals.map((b) => b.energy));
    return blockResiduals.map((block) => {
      const intensity = block.energy / maxEnergy;
      return `hsl(${240 - intensity * 240}, 70%, 50%)`;
    });
  }, [blockResiduals]);

  const histogramData = useMemo(() => {
    if (!coefficientStats) return [];
    const bins = 20;
    const maxVal = coefficientStats.max || 1;
    const binSize = maxVal / bins;
    const histogram = new Array(bins).fill(0);

    blockResiduals.forEach((block) => {
      const binIndex = Math.min(Math.floor(block.maxCoeff / binSize), bins - 1);
      histogram[binIndex]++;
    });

    return histogram.map((count, index) => ({
      binStart: index * binSize,
      binEnd: (index + 1) * binSize,
      count,
    }));
  }, [coefficientStats, blockResiduals]);

  if (!frame) {
    return (
      <div className="residuals-view residuals-view-empty">
        <p>No frame selected</p>
      </div>
    );
  }

  return (
    <div className="residuals-view">
      <div className="residuals-header">
        <h3>Residuals Analysis</h3>
        <div className="residuals-frame-info">
          <span>Frame {frame.frame_index}</span>
          <span className={frame.frame_type.toLowerCase()}>
            {frame.frame_type}
          </span>
          <span
            className="residuals-approx-label"
            title="Real per-block coefficient magnitudes from entropy decode. Uses representative (not neighbor-adaptive) probability contexts, so exact values may differ from a spec-exact decoder -- see bitvue-sidecar's residual_analysis module doc."
          >
            Approximate coefficient decode
          </span>
        </div>
      </div>

      {/* Statistics Panel */}
      {coefficientStats && (
        <div className="residuals-stats">
          <div className="residuals-stat-item">
            <span className="residuals-stat-label">Non-Zero Coeffs:</span>
            <span className="residuals-stat-value">
              {coefficientStats.nonZeroCount.toLocaleString()}
            </span>
          </div>
          <div className="residuals-stat-item">
            <span className="residuals-stat-label">Zero Coeffs:</span>
            <span className="residuals-stat-value">
              {coefficientStats.zeroCount.toLocaleString()}
            </span>
          </div>
          <div className="residuals-stat-item">
            <span className="residuals-stat-label">Mean:</span>
            <span className="residuals-stat-value">
              {coefficientStats.mean.toFixed(2)}
            </span>
          </div>
          <div className="residuals-stat-item">
            <span className="residuals-stat-label">Std Dev:</span>
            <span className="residuals-stat-value">
              {coefficientStats.variance.toFixed(2)}
            </span>
          </div>
          <div className="residuals-stat-item">
            <span className="residuals-stat-label">Energy:</span>
            <span className="residuals-stat-value">
              {coefficientStats.energy.toFixed(0)}
            </span>
          </div>
        </div>
      )}

      <div className="residuals-content">
        {/* Heatmap View */}
        {showHeatmap && (
          <div className="residuals-heatmap-section">
            <h4>Residual Energy Heatmap</h4>
            <div className="residuals-heatmap-container">
              <svg
                width={width}
                height={height}
                className="residuals-heatmap"
                viewBox={`0 0 ${width} ${height}`}
                preserveAspectRatio="xMidYMid meet"
              >
                {blockResiduals.map((block, index) => (
                  <rect
                    key={`${block.x}-${block.y}`}
                    x={block.x}
                    y={block.y}
                    width={block.width}
                    height={block.height}
                    fill={heatmapColors[index] || "#333"}
                    stroke="rgba(255,255,255,0.1)"
                    strokeWidth="0.5"
                    opacity={0.8}
                  >
                    <title>
                      Block ({block.x}, {block.y}){`\n`}
                      Energy: {block.energy.toFixed(2)}
                      {`\n`}
                      Max Coeff: {block.maxCoeff.toFixed(2)}
                      {`\n`}
                      Non-Zeros: {block.nonZeros}
                    </title>
                  </rect>
                ))}
              </svg>
            </div>

            {/* Color Scale Legend */}
            <div className="residuals-heatmap-legend">
              <span>Low</span>
              <div className="residuals-heatmap-scale">
                {Array.from({ length: 10 }).map((_, i) => (
                  <div
                    key={i}
                    className="residuals-heatmap-scale-step"
                    style={{
                      background: `hsl(${240 - (i / 9) * 240}, 70%, 50%)`,
                    }}
                  />
                ))}
              </div>
              <span>High</span>
            </div>
          </div>
        )}

        {/* Histogram View */}
        {showHistogram && (
          <div className="residuals-histogram-section">
            <h4>Coefficient Distribution</h4>
            <div className="residuals-histogram-container">
              <svg width="100%" height="200" className="residuals-histogram">
                {histogramData.map((bin, index) => {
                  const maxCount = Math.max(
                    ...histogramData.map((b) => b.count),
                  );
                  const barHeight =
                    maxCount > 0 ? (bin.count / maxCount) * 180 : 0;
                  const x = (index / histogramData.length) * 100;
                  const barWidth = 100 / histogramData.length - 0.5;

                  return (
                    <g key={index}>
                      <rect
                        x={`${x}%`}
                        y={200 - barHeight}
                        width={`${barWidth}%`}
                        height={barHeight}
                        fill="var(--bitvue-accent)"
                        opacity="0.8"
                        rx="2"
                      >
                        <title>
                          Range: [{bin.binStart.toFixed(1)},{" "}
                          {bin.binEnd.toFixed(1)}]{`\n`}
                          Count: {bin.count}
                        </title>
                      </rect>
                    </g>
                  );
                })}
              </svg>
            </div>
            <div className="residuals-histogram-labels">
              <span>0</span>
              <span>Coefficient Value</span>
              <span>{coefficientStats?.max.toFixed(0) || "0"}</span>
            </div>
          </div>
        )}
      </div>
    </div>
  );
});
