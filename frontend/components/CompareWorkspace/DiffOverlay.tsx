/**
 * Diff Overlay - Difference visualization for A/B compare
 *
 * Real per-pixel A/B diff heatmap (`bitvue_engine::diff_heatmap`, via the `get_diff_frame`
 * sidecar command -- docs/DEVELOPMENT_PHASES.md Phase 7.5), with the same QP-delta / frame-size
 * graceful-degrade fallback chain as before for when a compare workspace isn't ready (no
 * `create_compare_workspace` call yet) or the streams' resolutions don't exactly match. Used to
 * do its own ad-hoc client-side 16x16-block pixel diff here (re-fetching both streams' raw YUV
 * and computing deltas in TS) -- replaced with the real engine call, which also gets real PTS-
 * based A/B frame alignment for free (`get_diff_frame` resolves the aligned B frame itself, so
 * this component no longer needs `frameB`'s frame_index at all for the real-diff tier, only for
 * the QP/size fallback tiers below).
 */

import { memo, useMemo, useState, useEffect } from "react";
import { type FrameInfo } from "../../types/video";
import {
  getDiffFrame,
  type DiffHeatmapResult,
  type DiffMode,
} from "../../services/electronBridgeService";
import "./DiffOverlay.css";

interface DiffOverlayProps {
  frameA: FrameInfo;
  frameB: FrameInfo;
  mode: DiffMode;
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
  // Real A/B diff heatmap from the engine -- resolves the aligned B frame itself.
  const [diffFrame, setDiffFrame] = useState<DiffHeatmapResult | null>(null);

  useEffect(() => {
    let cancelled = false;
    getDiffFrame(frameA.frame_index, mode)
      .catch(() => null)
      .then((result) => {
        if (cancelled) return;
        setDiffFrame(result);
      });
    return () => {
      cancelled = true;
    };
  }, [frameA.frame_index, mode]);

  // Build block-level delta grid: prefer the real diff heatmap → QP grid → size approx
  const diffBlocks = useMemo(() => {
    // 1. Real per-pixel A/B diff heatmap (half-res, one block per heatmap cell).
    if (diffFrame && diffFrame.values.length > 0) {
      const { heatmap_width: gridW, heatmap_height: gridH, values } = diffFrame;
      const maxDelta = Math.max(
        Math.abs(diffFrame.min_value),
        Math.abs(diffFrame.max_value),
        1,
      );
      const deltas: {
        x: number;
        y: number;
        w: number;
        h: number;
        delta: number;
      }[] = [];
      for (let row = 0; row < gridH; row++) {
        for (let col = 0; col < gridW; col++) {
          deltas.push({
            x: col,
            y: row,
            w: 1,
            h: 1,
            delta: values[row * gridW + col] ?? 0,
          });
        }
      }
      return {
        blocks: deltas,
        maxDelta,
        totalW: gridW,
        totalH: gridH,
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
  }, [frameA, frameB, diffFrame]);

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

  const sourceLabel =
    diffBlocks.source === "pixel"
      ? "real diff heatmap"
      : diffBlocks.source === "qp"
        ? "QP delta"
        : "size approx";
  const header =
    mode === "signed"
      ? `Subtraction Map (${sourceLabel})`
      : `Temperature Map (${sourceLabel})`;

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
        {diffFrame && (
          <span className="diff-psnr-note">
            min: {diffFrame.min_value.toFixed(1)} &nbsp; max:{" "}
            {diffFrame.max_value.toFixed(1)}
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
