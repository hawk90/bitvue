/**
 * Diff Overlay - Difference visualization for A/B compare
 *
 * Shows QP-delta difference map, or frame-size based approximation.
 */

import { memo, useMemo, useState, useEffect } from "react";
import { type FrameInfo } from "../../types/video";
import {
  getDecodedFrameYuv,
  type BridgeDecodedYuvFrame,
} from "../../services/electronBridgeService";
import "./DiffOverlay.css";

interface DiffOverlayProps {
  frameA: FrameInfo;
  frameB: FrameInfo;
  mode: "difference" | "psnr" | "ssim";
}

function lerp(a: number, b: number, t: number) {
  return a + (b - a) * Math.max(0, Math.min(1, t));
}

function diffColor(delta: number, maxDelta: number): string {
  if (maxDelta === 0) return "rgba(0,100,200,0.25)";
  const t = Math.abs(delta) / maxDelta;
  // Blue (low diff) → yellow → red (high diff)
  if (t < 0.5) {
    const r = Math.round(lerp(0, 255, t * 2));
    const g = Math.round(lerp(100, 220, t * 2));
    return `rgba(${r},${g},20,${0.3 + t * 0.5})`;
  } else {
    const r = 255;
    const g = Math.round(lerp(220, 0, (t - 0.5) * 2));
    return `rgba(${r},${g},0,${0.55 + (t - 0.5) * 0.45})`;
  }
}

function DiffOverlay({ frameA, frameB, mode }: DiffOverlayProps) {
  // Load YUV data for both streams to enable pixel-level diff
  const [yuvA, setYuvA] = useState<BridgeDecodedYuvFrame | null>(null);
  const [yuvB, setYuvB] = useState<BridgeDecodedYuvFrame | null>(null);

  useEffect(() => {
    let cancelled = false;
    Promise.all([
      getDecodedFrameYuv("A", frameA.frame_index).catch(() => null),
      getDecodedFrameYuv("B", frameB.frame_index).catch(() => null),
    ]).then(([a, b]) => {
      if (cancelled) return;
      setYuvA(a);
      setYuvB(b);
    });
    return () => {
      cancelled = true;
    };
  }, [frameA.frame_index, frameB.frame_index]);

  // Build block-level delta grid: prefer YUV pixel diff → QP grid → size approx
  const diffBlocks = useMemo(() => {
    // 1. Pixel-level YUV diff (16x16 blocks sampled from Y plane)
    if (
      yuvA &&
      yuvB &&
      yuvA.yLen > 0 &&
      yuvB.yLen > 0 &&
      yuvA.width === yuvB.width
    ) {
      const yaData = yuvA.bytes.subarray(0, yuvA.yLen);
      const ybData = yuvB.bytes.subarray(0, yuvB.yLen);
      const w = yuvA.width;
      const h = yuvA.height;
      const blockSize = 16;
      const gridW = Math.ceil(w / blockSize);
      const gridH = Math.ceil(h / blockSize);
      const deltas: {
        x: number;
        y: number;
        w: number;
        h: number;
        delta: number;
      }[] = [];
      let maxDelta = 0;
      for (let row = 0; row < gridH; row++) {
        for (let col = 0; col < gridW; col++) {
          let sumDiff = 0;
          let count = 0;
          for (let dy = 0; dy < blockSize; dy++) {
            const py = row * blockSize + dy;
            if (py >= h) break;
            for (let dx = 0; dx < blockSize; dx++) {
              const px = col * blockSize + dx;
              if (px >= w) break;
              const idx = py * yuvA.yStride + px;
              sumDiff += Math.abs((yaData[idx] ?? 0) - (ybData[idx] ?? 0));
              count++;
            }
          }
          const avgDiff = count > 0 ? sumDiff / count : 0;
          if (avgDiff > maxDelta) maxDelta = avgDiff;
          deltas.push({
            x: col * blockSize,
            y: row * blockSize,
            w: blockSize,
            h: blockSize,
            delta: avgDiff,
          });
        }
      }
      return {
        blocks: deltas,
        maxDelta: Math.max(maxDelta, 1),
        totalW: w,
        totalH: h,
        source: "pixel" as const,
      };
    }

    // 2. QP grid delta
    const qpA = frameA.qp_grid;
    const qpB = frameB.qp_grid;

    if (qpA && qpB && qpA.qp.length > 0 && qpB.qp.length > 0) {
      const gridW = Math.min(qpA.grid_w, qpB.grid_w);
      const gridH = Math.min(qpA.grid_h, qpB.grid_h);
      const blockW = qpA.block_w;
      const blockH = qpA.block_h;

      const deltas: {
        x: number;
        y: number;
        w: number;
        h: number;
        delta: number;
      }[] = [];
      let maxDelta = 0;

      for (let row = 0; row < gridH; row++) {
        for (let col = 0; col < gridW; col++) {
          const idxA = row * qpA.grid_w + col;
          const idxB = row * qpB.grid_w + col;
          const qpValA = qpA.qp[idxA] ?? -1;
          const qpValB = qpB.qp[idxB] ?? -1;
          if (qpValA < 0 || qpValB < 0) continue;
          const delta = qpValB - qpValA;
          if (Math.abs(delta) > maxDelta) maxDelta = Math.abs(delta);
          deltas.push({
            x: col * blockW,
            y: row * blockH,
            w: blockW,
            h: blockH,
            delta,
          });
        }
      }

      const totalW = gridW * blockW;
      const totalH = gridH * blockH;
      return {
        blocks: deltas,
        maxDelta,
        totalW,
        totalH,
        source: "qp" as const,
      };
    }

    // 3. Fallback: single-block size diff approximation
    const sizeDiff = Math.abs(frameB.size - frameA.size);
    const maxSize = Math.max(frameA.size, frameB.size, 1);
    const relDiff = sizeDiff / maxSize;
    return {
      blocks: [{ x: 0, y: 0, w: 320, h: 180, delta: relDiff * 51 }],
      maxDelta: 51,
      totalW: 320,
      totalH: 180,
      source: "size" as const,
    };
  }, [frameA, frameB, yuvA, yuvB]);

  const svgBlocks = useMemo(() => {
    const { blocks, maxDelta } = diffBlocks;
    return blocks.map((b, i) => (
      <rect
        key={i}
        x={b.x}
        y={b.y}
        width={b.w}
        height={b.h}
        fill={diffColor(b.delta, maxDelta)}
        stroke="none"
      />
    ));
  }, [diffBlocks]);

  // PSNR approximation: invert QP delta (higher QP diff → lower PSNR)
  const psnrLabel = useMemo(() => {
    const qpA = frameA.qp_grid;
    const qpB = frameB.qp_grid;
    if (!qpA || !qpB) return null;
    const avgQpA =
      qpA.qp.filter((q) => q >= 0).reduce((a, b) => a + b, 0) /
      (qpA.qp.filter((q) => q >= 0).length || 1);
    const avgQpB =
      qpB.qp.filter((q) => q >= 0).reduce((a, b) => a + b, 0) /
      (qpB.qp.filter((q) => q >= 0).length || 1);
    // Rough approximation: PSNR ≈ 50 - QP * 0.7
    const estPsnrA = Math.max(0, 50 - avgQpA * 0.7);
    const estPsnrB = Math.max(0, 50 - avgQpB * 0.7);
    return {
      a: estPsnrA.toFixed(1),
      b: estPsnrB.toFixed(1),
      delta: (estPsnrB - estPsnrA).toFixed(1),
    };
  }, [frameA, frameB]);

  const sourceLabel =
    diffBlocks.source === "pixel"
      ? "YUV pixel"
      : diffBlocks.source === "qp"
        ? "QP delta"
        : "size approx";
  const header =
    mode === "psnr"
      ? `PSNR Map (${sourceLabel})`
      : mode === "ssim"
        ? `SSIM Map (${sourceLabel})`
        : `Difference Map (${sourceLabel})`;

  return (
    <div className="diff-overlay">
      <div className="diff-header">
        <span>{header}</span>
        {diffBlocks.source === "size" && (
          <span className="diff-approx-note">Size-based approximation</span>
        )}
        {diffBlocks.source === "pixel" && (
          <span className="diff-approx-note diff-pixel-note">
            Pixel-accurate
          </span>
        )}
        {psnrLabel && mode !== "difference" && (
          <span className="diff-psnr-note">
            A: ~{psnrLabel.a} dB &nbsp; B: ~{psnrLabel.b} dB &nbsp; Δ:{" "}
            {Number(psnrLabel.delta) > 0 ? "+" : ""}
            {psnrLabel.delta} dB
          </span>
        )}
        <span className="diff-legend">
          <span className="legend-item">
            <span className="legend-color diff-none"></span> Low
          </span>
          <span className="legend-item">
            <span className="legend-color diff-high"></span> High
          </span>
        </span>
      </div>
      <div className="diff-canvas">
        <svg
          viewBox={`0 0 ${diffBlocks.totalW} ${diffBlocks.totalH}`}
          preserveAspectRatio="xMidYMid meet"
          style={{ width: "100%", height: "100%", display: "block" }}
        >
          {svgBlocks}
        </svg>
      </div>
    </div>
  );
}

export default memo(DiffOverlay);
