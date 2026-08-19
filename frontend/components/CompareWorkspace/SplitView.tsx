/**
 * Split View - Single-canvas wipe/slider comparison for A/B compare
 *
 * PARITY_CHECKLIST.md CMP-02 ("Split(H/V)") -- StreamEye's Horizontal/Vertical Split. Renders
 * stream A's decoded frame into one half of a single canvas and stream B's into the other half,
 * split at a draggable divider (0-100%, vertical = left/right, horizontal = top/bottom). This is
 * the "before/after" wipe-slider interaction common to video-QC tools -- distinct from the
 * side-by-side view (two independent panels) and the diff/heatmap view (DiffOverlay.tsx).
 *
 * Reuses the same decoded-YUV pipeline as StreamPlayer.tsx (`getDecodedFrameYuv` +
 * `bridgeYuvToFrame`) and the same `YUVRenderer` pixel path as VideoCanvas.tsx -- no new pixel
 * pipeline. Each stream is rendered to its own offscreen canvas at full resolution via
 * `YUVRenderer`, then the visible canvas composites both offscreen canvases with a clip
 * rectangle on either side of the split position.
 */

import { memo, useCallback, useEffect, useRef, useState } from "react";
import { type FrameInfo } from "../../types/video";
import type { YUVFrame } from "../../types/yuv";
import { Colorspace } from "../../types/yuv";
import { YUVRenderer } from "../../utils/yuvRenderer";
import {
  getDecodedFrameYuv,
  bridgeYuvToFrame,
} from "../../services/electronBridgeService";
import "./SplitView.css";

export type SplitOrientation = "vertical" | "horizontal";

interface SplitViewProps {
  framesA: FrameInfo[];
  framesB: FrameInfo[];
  currentFrameA: number;
  currentFrameB: number;
  orientation: SplitOrientation;
}

const DEFAULT_SPLIT_POSITION = 50;

function SplitView({
  framesA,
  framesB,
  currentFrameA,
  currentFrameB,
  orientation,
}: SplitViewProps) {
  const currentFrameAData = framesA[currentFrameA] || null;
  const currentFrameBData = framesB[currentFrameB] || null;

  const [yuvA, setYuvA] = useState<YUVFrame | null>(null);
  const [yuvB, setYuvB] = useState<YUVFrame | null>(null);
  const [splitPosition, setSplitPosition] = useState(DEFAULT_SPLIT_POSITION);
  const [isDragging, setIsDragging] = useState(false);

  const canvasRef = useRef<HTMLCanvasElement>(null);
  const offscreenARef = useRef<HTMLCanvasElement | null>(null);
  const offscreenBRef = useRef<HTMLCanvasElement | null>(null);
  const rendererARef = useRef<YUVRenderer | null>(null);
  const rendererBRef = useRef<YUVRenderer | null>(null);

  // Lazy-init offscreen canvases + renderers once -- these never touch the DOM, so a plain ref
  // (not React state) is enough; mirrors VideoCanvas's `new YUVRenderer(canvas)` pattern but with
  // a detached canvas instead of `canvasRef.current` from the render tree.
  if (!offscreenARef.current) {
    offscreenARef.current = document.createElement("canvas");
    rendererARef.current = new YUVRenderer(offscreenARef.current);
  }
  if (!offscreenBRef.current) {
    offscreenBRef.current = document.createElement("canvas");
    rendererBRef.current = new YUVRenderer(offscreenBRef.current);
  }

  // Fetch stream A's decoded YUV for the current frame (same pattern as StreamPlayer.tsx).
  useEffect(() => {
    if (!currentFrameAData) {
      setYuvA(null);
      return;
    }
    let cancelled = false;
    getDecodedFrameYuv("A", currentFrameA)
      .then((data) => {
        if (!cancelled) setYuvA(bridgeYuvToFrame(data));
      })
      .catch(() => {
        if (!cancelled) setYuvA(null);
      });
    return () => {
      cancelled = true;
    };
  }, [currentFrameA, currentFrameAData]);

  // Fetch stream B's decoded YUV for the current frame.
  useEffect(() => {
    if (!currentFrameBData) {
      setYuvB(null);
      return;
    }
    let cancelled = false;
    getDecodedFrameYuv("B", currentFrameB)
      .then((data) => {
        if (!cancelled) setYuvB(bridgeYuvToFrame(data));
      })
      .catch(() => {
        if (!cancelled) setYuvB(null);
      });
    return () => {
      cancelled = true;
    };
  }, [currentFrameB, currentFrameBData]);

  // Composite: render A and B to their own offscreen canvases, then draw each into its half of
  // the visible canvas via a clip rect either side of the split position, plus the divider line.
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    // Stream A defines the canvas's pixel geometry (same convention DiffOverlay's QP-grid
    // fallback implicitly uses) -- stream B is drawn scaled into that geometry if its resolution
    // differs, since `drawImage`'s dest width/height can differ from the source's natural size.
    const width = yuvA?.width ?? yuvB?.width ?? 640;
    const height = yuvA?.height ?? yuvB?.height ?? 360;
    if (canvas.width !== width || canvas.height !== height) {
      canvas.width = width;
      canvas.height = height;
    }

    ctx.fillStyle = "#000";
    ctx.fillRect(0, 0, width, height);

    if (yuvA && rendererARef.current) {
      rendererARef.current.render(yuvA, Colorspace.BT709);
    }
    if (yuvB && rendererBRef.current) {
      rendererBRef.current.render(yuvB, Colorspace.BT709);
    }

    const splitX =
      orientation === "vertical" ? (width * splitPosition) / 100 : width;
    const splitY =
      orientation === "horizontal" ? (height * splitPosition) / 100 : height;

    if (yuvA && offscreenARef.current) {
      ctx.save();
      ctx.beginPath();
      if (orientation === "vertical") {
        ctx.rect(0, 0, splitX, height);
      } else {
        ctx.rect(0, 0, width, splitY);
      }
      ctx.clip();
      ctx.drawImage(offscreenARef.current, 0, 0, width, height);
      ctx.restore();
    }

    if (yuvB && offscreenBRef.current) {
      ctx.save();
      ctx.beginPath();
      if (orientation === "vertical") {
        ctx.rect(splitX, 0, width - splitX, height);
      } else {
        ctx.rect(0, splitY, width, height - splitY);
      }
      ctx.clip();
      ctx.drawImage(offscreenBRef.current, 0, 0, width, height);
      ctx.restore();
    }

    // Divider line + handle knob, drawn last so it sits on top of both halves.
    const lineWidth = Math.max(1.5, width / 500);
    ctx.strokeStyle = "#00d4ff";
    ctx.lineWidth = lineWidth;
    ctx.beginPath();
    if (orientation === "vertical") {
      ctx.moveTo(splitX, 0);
      ctx.lineTo(splitX, height);
    } else {
      ctx.moveTo(0, splitY);
      ctx.lineTo(width, splitY);
    }
    ctx.stroke();

    const knobX = orientation === "vertical" ? splitX : width / 2;
    const knobY = orientation === "horizontal" ? splitY : height / 2;
    const knobRadius = Math.max(8, width / 100);
    ctx.beginPath();
    ctx.arc(knobX, knobY, knobRadius, 0, Math.PI * 2);
    ctx.fillStyle = "#00d4ff";
    ctx.fill();
    ctx.strokeStyle = "#003844";
    ctx.lineWidth = Math.max(1, lineWidth / 2);
    ctx.stroke();
  }, [yuvA, yuvB, splitPosition, orientation]);

  // Split position from a pointer's client coordinates, relative to the canvas's rendered
  // (CSS) box -- clamped to [0, 100]. Canvas CSS size and its backing-store size (width/height
  // attrs, set above) can differ (object-fit: contain), so this uses the bounding rect, not the
  // backing-store pixel dimensions.
  const updateSplitFromClientPos = useCallback(
    (clientX: number, clientY: number) => {
      const canvas = canvasRef.current;
      if (!canvas) return;
      const rect = canvas.getBoundingClientRect();
      if (rect.width === 0 || rect.height === 0) return;
      const pct =
        orientation === "vertical"
          ? ((clientX - rect.left) / rect.width) * 100
          : ((clientY - rect.top) / rect.height) * 100;
      setSplitPosition(Math.min(100, Math.max(0, pct)));
    },
    [orientation],
  );

  // Drag the divider anywhere on the canvas (not just a thin hit target on the line itself --
  // matches the common "before/after" image-compare-slider convention, and a 1.5px line would be
  // an impractically small drag target). Same window-listener drag pattern as Timeline.tsx's
  // scrubber (`handleMouseDown`/`handleDragMove`/`handleDragUp`).
  const handleMouseDown = useCallback(
    (e: React.MouseEvent<HTMLCanvasElement>) => {
      setIsDragging(true);
      updateSplitFromClientPos(e.clientX, e.clientY);

      const handleDragMove = (moveEvent: MouseEvent) => {
        updateSplitFromClientPos(moveEvent.clientX, moveEvent.clientY);
      };
      const handleDragUp = () => {
        setIsDragging(false);
        window.removeEventListener("mousemove", handleDragMove);
        window.removeEventListener("mouseup", handleDragUp);
      };
      window.addEventListener("mousemove", handleDragMove);
      window.addEventListener("mouseup", handleDragUp);
    },
    [updateSplitFromClientPos],
  );

  // Keyboard nudge for accessibility -- arrow keys move the divider by 1% (5% with Shift).
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLCanvasElement>) => {
      const decreaseKey = orientation === "vertical" ? "ArrowLeft" : "ArrowUp";
      const increaseKey =
        orientation === "vertical" ? "ArrowRight" : "ArrowDown";
      if (e.key !== decreaseKey && e.key !== increaseKey) return;
      e.preventDefault();
      const step = e.shiftKey ? 5 : 1;
      const delta = e.key === increaseKey ? step : -step;
      setSplitPosition((prev) => Math.min(100, Math.max(0, prev + delta)));
    },
    [orientation],
  );

  return (
    <div className={`split-view split-view--${orientation}`}>
      <div className="split-view-labels">
        <span className="split-view-label split-view-label-a">
          A: {currentFrameA + 1} / {framesA.length}
        </span>
        <span className="split-view-label split-view-label-b">
          B: {currentFrameB + 1} / {framesB.length}
        </span>
      </div>
      <div className="split-view-canvas-wrap">
        {yuvA || yuvB ? (
          <canvas
            ref={canvasRef}
            className={`split-view-canvas${isDragging ? " split-view-canvas--dragging" : ""}`}
            width={yuvA?.width ?? yuvB?.width ?? 640}
            height={yuvA?.height ?? yuvB?.height ?? 360}
            role="slider"
            tabIndex={0}
            aria-label={`A/B split divider (${orientation})`}
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={Math.round(splitPosition)}
            onMouseDown={handleMouseDown}
            onKeyDown={handleKeyDown}
          />
        ) : (
          <div className="split-view-placeholder">
            <span>No frame data</span>
          </div>
        )}
      </div>
    </div>
  );
}

export default memo(SplitView);
