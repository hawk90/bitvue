/**
 * AV1 Features View Component
 *
 * Visualizes AV1-specific advanced features:
 * - CDEF (Constrained Directional Enhancement Filter)
 * - Loop Restoration (Wiener/SgrProj)
 * - Film Grain synthesis
 * - Super Resolution
 */

import { memo, useMemo, useEffect, useState } from "react";
import type { FrameInfo } from "../../../types/video";

interface AV1FeaturesViewProps {
  frame: FrameInfo | null;
  width: number;
  height: number;
  showCdef?: boolean;
  showLoopRestoration?: boolean;
  showFilmGrain?: boolean;
  showSuperRes?: boolean;
}

interface CdefBlock {
  x: number;
  y: number;
  size: number;
  direction: number;
  strength: number;
}

interface RestorationUnit {
  x: number;
  y: number;
  size: number;
  restorationType: number; // 0=None, 1=Wiener, 2=SgrProj, 3=Dual
  filterData?: number[];
}

interface FilmGrainParams {
  enabled: boolean;
  updateOffset: number;
  seed: number;
  scalingShift: number;
  arCoeffLag: number;
  arCoeffsY: number[];
  arCoeffsUV: number[];
  arCoeffShift: number;
  grainScaleShift: number;
  chromaScalingFromLuma: boolean;
  overlap: boolean;
  clipToRestrictedRange: boolean;
}

interface SuperResParams {
  enabled: boolean;
  scaleDenominator: number;
  upscaledWidth: number;
  upscaledHeight: number;
}

export const AV1FeaturesView = memo(function AV1FeaturesView({
  frame,
  width,
  height,
  showCdef = true,
  showLoopRestoration = true,
  showFilmGrain = true,
  showSuperRes = true,
}: AV1FeaturesViewProps) {
  const [cdefBlocks, setCdefBlocks] = useState<CdefBlock[]>([]);
  const [restorationUnits, setRestorationUnits] = useState<RestorationUnit[]>(
    [],
  );
  const [filmGrain, setFilmGrain] = useState<FilmGrainParams | null>(null);
  const [superRes, setSuperRes] = useState<SuperResParams | null>(null);

  useEffect(() => {
    if (!frame || width === 0 || height === 0) {
      setCdefBlocks([]);
      setRestorationUnits([]);
      setFilmGrain(null);
      setSuperRes(null);
      return;
    }

    // Generate mock CDEF blocks
    const blockSize = 8;
    const gridW = Math.ceil(width / blockSize);
    const gridH = Math.ceil(height / blockSize);

    const blocks: CdefBlock[] = [];
    for (let y = 0; y < gridH; y++) {
      for (let x = 0; x < gridW; x++) {
        blocks.push({
          x: x * blockSize,
          y: y * blockSize,
          size: blockSize,
          direction: Math.floor(Math.random() * 8),
          strength: Math.floor(Math.random() * 16),
        });
      }
    }
    setCdefBlocks(blocks);

    // Generate mock restoration units
    const unitSize = 64;
    const restorationGridW = Math.ceil(width / unitSize);
    const restorationGridH = Math.ceil(height / unitSize);

    const units: RestorationUnit[] = [];
    for (let y = 0; y < restorationGridH; y++) {
      for (let x = 0; x < restorationGridW; x++) {
        const type = Math.floor(Math.random() * 3);
        units.push({
          x: x * unitSize,
          y: y * unitSize,
          size: unitSize,
          restorationType: type,
          filterData:
            type > 0
              ? [
                  Math.floor(Math.random() * 8) - 4,
                  Math.floor(Math.random() * 8) - 4,
                  Math.floor(Math.random() * 8) - 4,
                ]
              : undefined,
        });
      }
    }
    setRestorationUnits(units);

    // Mock film grain params
    setFilmGrain({
      enabled: true,
      updateOffset: 0,
      seed: Math.floor(Math.random() * 10000),
      scalingShift: 11,
      arCoeffLag: 3,
      arCoeffsY: [0, 1, -1, 0],
      arCoeffsUV: [0, 1, 0],
      arCoeffShift: 6,
      grainScaleShift: 0,
      chromaScalingFromLuma: false,
      overlap: true,
      clipToRestrictedRange: true,
    });

    // Mock super res
    setSuperRes({
      enabled: false,
      scaleDenominator: 8,
      upscaledWidth: width,
      upscaledHeight: height,
    });
  }, [frame, width, height]);

  const cdefColors = useMemo(() => {
    return cdefBlocks.map((block) => {
      const directionAngle = (block.direction / 8) * 360;
      const intensity = block.strength / 16;
      return `hsl(${directionAngle}, 70%, ${30 + intensity * 40}%)`;
    });
  }, [cdefBlocks]);

  const restorationColors = useMemo(() => {
    return restorationUnits.map((unit) => {
      switch (unit.restorationType) {
        case 1:
          return "rgba(255, 100, 100, 0.6)"; // Wiener - red
        case 2:
          return "rgba(100, 255, 100, 0.6)"; // SgrProj - green
        case 3:
          return "rgba(100, 100, 255, 0.6)"; // Dual - blue
        default:
          return "rgba(128, 128, 128, 0.3)"; // None - gray
      }
    });
  }, [restorationUnits]);

  if (!frame) {
    return (
      <div className="av1-features-view av1-features-view-empty">
        <p>No frame selected</p>
      </div>
    );
  }

  return (
    <div className="av1-features-view">
      <div className="av1-features-header">
        <h3>AV1 Advanced Features</h3>
        <div className="av1-features-frame-info">
          <span>Frame {frame.frame_index}</span>
          <span className={frame.frame_type.toLowerCase()}>
            {frame.frame_type}
          </span>
        </div>
      </div>

      {/* CDEF Section */}
      {showCdef && (
        <div className="av1-features-section">
          <h4>CDEF (Constrained Directional Enhancement Filter)</h4>
          <div className="av1-features-stats">
            <div className="av1-features-stat">
              <span className="av1-features-stat-label">Blocks:</span>
              <span className="av1-features-stat-value">
                {cdefBlocks.length.toLocaleString()}
              </span>
            </div>
            <div className="av1-features-stat">
              <span className="av1-features-stat-label">Avg Strength:</span>
              <span className="av1-features-stat-value">
                {(
                  cdefBlocks.reduce((s, b) => s + b.strength, 0) /
                  cdefBlocks.length
                ).toFixed(2)}
              </span>
            </div>
          </div>
          <div className="av1-features-visualization">
            <svg
              width={width}
              height={height}
              viewBox={`0 0 ${width} ${height}`}
            >
              {cdefBlocks.map((block, index) => (
                <rect
                  key={`cdef-${index}`}
                  x={block.x}
                  y={block.y}
                  width={block.size}
                  height={block.size}
                  fill={cdefColors[index]}
                  stroke="rgba(255,255,255,0.1)"
                  strokeWidth="0.5"
                >
                  <title>
                    Block ({block.x}, {block.y}){`\n`}
                    Direction: {block.direction} ({(block.direction / 8) * 360}
                    °){`\n`}
                    Strength: {block.strength}
                  </title>
                </rect>
              ))}
            </svg>
          </div>
        </div>
      )}

      {/* Loop Restoration Section */}
      {showLoopRestoration && (
        <div className="av1-features-section">
          <h4>Loop Restoration</h4>
          <div className="av1-features-stats">
            <div className="av1-features-stat">
              <span className="av1-features-stat-label">Wiener:</span>
              <span className="av1-features-stat-value">
                {restorationUnits.filter((u) => u.restorationType === 1).length}
              </span>
            </div>
            <div className="av1-features-stat">
              <span className="av1-features-stat-label">SgrProj:</span>
              <span className="av1-features-stat-value">
                {restorationUnits.filter((u) => u.restorationType === 2).length}
              </span>
            </div>
            <div className="av1-features-stat">
              <span className="av1-features-stat-label">None:</span>
              <span className="av1-features-stat-value">
                {restorationUnits.filter((u) => u.restorationType === 0).length}
              </span>
            </div>
          </div>
          <div className="av1-features-visualization">
            <svg
              width={width}
              height={height}
              viewBox={`0 0 ${width} ${height}`}
            >
              {restorationUnits.map((unit, index) => (
                <rect
                  key={`lr-${index}`}
                  x={unit.x}
                  y={unit.y}
                  width={unit.size}
                  height={unit.size}
                  fill={restorationColors[index]}
                  stroke="rgba(255,255,255,0.1)"
                  strokeWidth="0.5"
                >
                  <title>
                    Unit ({unit.x}, {unit.y}){`\n`}
                    Type:{" "}
                    {
                      ["None", "Wiener", "SgrProj", "Dual"][
                        unit.restorationType
                      ]
                    }
                  </title>
                </rect>
              ))}
            </svg>
          </div>
        </div>
      )}

      {/* Film Grain Section */}
      {showFilmGrain && filmGrain && (
        <div className="av1-features-section">
          <h4>Film Grain Synthesis</h4>
          <div className="av1-features-grid">
            <div className="av1-features-grid-item">
              <span className="av1-features-grid-label">Enabled:</span>
              <span>{filmGrain.enabled ? "Yes" : "No"}</span>
            </div>
            <div className="av1-features-grid-item">
              <span className="av1-features-grid-label">Seed:</span>
              <span>{filmGrain.seed}</span>
            </div>
            <div className="av1-features-grid-item">
              <span className="av1-features-grid-label">AR Coeff Lag:</span>
              <span>{filmGrain.arCoeffLag}</span>
            </div>
            <div className="av1-features-grid-item">
              <span className="av1-features-grid-label">Scaling Shift:</span>
              <span>{filmGrain.scalingShift}</span>
            </div>
            <div className="av1-features-grid-item">
              <span className="av1-features-grid-label">Overlap:</span>
              <span>{filmGrain.overlap ? "Yes" : "No"}</span>
            </div>
            <div className="av1-features-grid-item">
              <span className="av1-features-grid-label">Chroma Scaling:</span>
              <span>
                {filmGrain.chromaScalingFromLuma ? "From Luma" : "Independent"}
              </span>
            </div>
          </div>
        </div>
      )}

      {/* Super Resolution Section */}
      {showSuperRes && superRes && (
        <div className="av1-features-section">
          <h4>Super Resolution</h4>
          <div className="av1-features-grid">
            <div className="av1-features-grid-item">
              <span className="av1-features-grid-label">Enabled:</span>
              <span>{superRes.enabled ? "Yes" : "No"}</span>
            </div>
            <div className="av1-features-grid-item">
              <span className="av1-features-grid-label">Scale:</span>
              <span>1/{superRes.scaleDenominator}</span>
            </div>
            {superRes.enabled && (
              <>
                <div className="av1-features-grid-item">
                  <span className="av1-features-grid-label">
                    Upscaled Width:
                  </span>
                  <span>{superRes.upscaledWidth}px</span>
                </div>
                <div className="av1-features-grid-item">
                  <span className="av1-features-grid-label">
                    Upscaled Height:
                  </span>
                  <span>{superRes.upscaledHeight}px</span>
                </div>
              </>
            )}
          </div>
        </div>
      )}

      {/* Legend */}
      <div className="av1-features-legend">
        <div className="av1-features-legend-item">
          <div
            className="av1-features-legend-box"
            style={{ background: "linear-gradient(to right, red, yellow)" }}
          ></div>
          <span>CDEF Direction/Strength</span>
        </div>
        <div className="av1-features-legend-item">
          <div
            className="av1-features-legend-box"
            style={{ background: "rgba(255, 100, 100, 0.6)" }}
          ></div>
          <span>Wiener Filter</span>
        </div>
        <div className="av1-features-legend-item">
          <div
            className="av1-features-legend-box"
            style={{ background: "rgba(100, 255, 100, 0.6)" }}
          ></div>
          <span>SgrProj Filter</span>
        </div>
        <div className="av1-features-legend-item">
          <div
            className="av1-features-legend-box"
            style={{ background: "rgba(128, 128, 128, 0.3)" }}
          ></div>
          <span>No Filter</span>
        </div>
      </div>
    </div>
  );
});
