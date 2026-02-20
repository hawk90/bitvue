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

  // Generate mock block residuals based on frame info
  useEffect(() => {
    if (!frame || width === 0 || height === 0) {
      setBlockResiduals([]);
      setCoefficientStats(null);
      return;
    }

    const blockSize = 16;
    const blocks: BlockResidual[] = [];
    const qp = 26; // Default QP value

    // Generate mock residual data
    const gridW = Math.ceil(width / blockSize);
    const gridH = Math.ceil(height / blockSize);

    for (let y = 0; y < gridH; y++) {
      for (let x = 0; x < gridW; x++) {
        const energy = Math.random() * (100 - qp);
        const maxCoeff = Math.random() * (255 - qp * 2);
        const nonZeros = Math.floor(
          Math.random() * ((blockSize * blockSize) / 4),
        );

        blocks.push({
          x: x * blockSize,
          y: y * blockSize,
          width: blockSize,
          height: blockSize,
          energy,
          maxCoeff,
          nonZeros,
        });
      }
    }

    setBlockResiduals(blocks);

    // Calculate coefficient statistics
    const allCoeffs = blocks.flatMap((b) =>
      Array(b.nonZeros)
        .fill(0)
        .map(() => Math.random() * b.maxCoeff),
    );

    if (allCoeffs.length > 0) {
      const mean = allCoeffs.reduce((a, b) => a + b, 0) / allCoeffs.length;
      const variance =
        allCoeffs.reduce((sum, val) => sum + Math.pow(val - mean, 2), 0) /
        allCoeffs.length;
      const energy = allCoeffs.reduce((sum, val) => sum + val * val, 0);

      setCoefficientStats({
        min: Math.min(...allCoeffs),
        max: Math.max(...allCoeffs),
        mean,
        variance: Math.sqrt(variance),
        energy,
        zeroCount: blocks.length * blockSize * blockSize - allCoeffs.length,
        nonZeroCount: allCoeffs.length,
      });
    }
  }, [frame, width, height]);

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
