/**
 * Video Canvas Component
 *
 * Handles canvas rendering with zoom and pan support
 * Includes mouse wheel zoom and drag-to-pan functionality
 * Supports both image-based and YUV-based rendering
 */

import { useRef, useEffect, memo, useMemo } from "react";
import { renderModeOverlay } from "../OverlayRenderer";
import type { VisualizationMode } from "../../../contexts/ModeContext";
import type { OverlayRenderOptionsExtended } from "../OverlayRenderer";
import type { Av1FeaturesData } from "../OverlayRenderer";
import type { FrameInfo } from "../../../types/video";
import {
  YUVRenderer,
  type YUVFrame,
  Colorspace,
  type ChannelMode,
} from "../../../utils/yuvRenderer";
import { createLogger } from "../../../utils/logger";

const logger = createLogger("VideoCanvas");

/** Apply channel isolation by zeroing out the unused planes (neutral chroma = 128). */
function applyChannelMode(frame: YUVFrame, mode: ChannelMode): YUVFrame {
  if (mode === "all") return frame;
  const neutral128 = (len: number) => new Uint8Array(len).fill(128);
  switch (mode) {
    case "Y":
      return {
        ...frame,
        u: neutral128(frame.u.length),
        v: neutral128(frame.v.length),
      };
    case "U": {
      // Upscale U plane to luma size and display as luminance (grayscale)
      const yFromU = new Uint8Array(frame.y.length);
      const scaleX =
        frame.chromaSubsampling === "420" || frame.chromaSubsampling === "422"
          ? 2
          : 1;
      const scaleY = frame.chromaSubsampling === "420" ? 2 : 1;
      for (let py = 0; py < frame.height; py++) {
        for (let px = 0; px < frame.width; px++) {
          const cu = Math.floor(px / scaleX);
          const cv = Math.floor(py / scaleY);
          yFromU[py * frame.yStride + px] = frame.u[cv * frame.uStride + cu];
        }
      }
      return {
        ...frame,
        y: yFromU,
        u: neutral128(frame.u.length),
        v: neutral128(frame.v.length),
      };
    }
    case "V": {
      const yFromV = new Uint8Array(frame.y.length);
      const scaleX =
        frame.chromaSubsampling === "420" || frame.chromaSubsampling === "422"
          ? 2
          : 1;
      const scaleY = frame.chromaSubsampling === "420" ? 2 : 1;
      for (let py = 0; py < frame.height; py++) {
        for (let px = 0; px < frame.width; px++) {
          const cu = Math.floor(px / scaleX);
          const cv = Math.floor(py / scaleY);
          yFromV[py * frame.yStride + px] = frame.v[cv * frame.vStride + cu];
        }
      }
      return {
        ...frame,
        y: yFromV,
        u: neutral128(frame.u.length),
        v: neutral128(frame.v.length),
      };
    }
  }
}

interface VideoCanvasProps {
  frameImage: HTMLImageElement | null;
  currentFrameIndex: number;
  currentFrame: FrameInfo | null;
  currentMode: VisualizationMode;
  zoom: number;
  pan: { x: number; y: number };
  onWheel: (e: React.WheelEvent) => void;
  onMouseDown: (e: React.MouseEvent) => void;
  onMouseMove: (e: React.MouseEvent) => void;
  onMouseUp: (e: React.MouseEvent) => void;
  isDragging: boolean;
  /** Raw YUV data if available (overrides frameImage when present) */
  yuvData?: YUVFrame;
  /** Active info overlays drawn on top of the main mode overlay. */
  activeOverlays?: ReadonlySet<VisualizationMode>;
  /** AV1 advanced feature data for CDEF / LR / film-grain / super-res modes. */
  av1Features?: Av1FeaturesData;
  /** Colorspace for YUV→RGB conversion (default BT.709) */
  colorspace?: Colorspace;
  /** Channel isolation mode (default "all") */
  channelMode?: ChannelMode;
}

export const VideoCanvas = memo(function VideoCanvas({
  frameImage,
  currentFrameIndex,
  currentFrame,
  currentMode,
  zoom,
  pan,
  onWheel,
  onMouseDown,
  onMouseMove,
  onMouseUp,
  isDragging,
  yuvData,
  activeOverlays,
  av1Features,
  colorspace = Colorspace.BT709,
  channelMode = "all",
}: VideoCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const webglCanvasRef = useRef<HTMLCanvasElement>(null);
  const rendererRef = useRef<YUVRenderer | null>(null);

  // Memoize canvas style to avoid creating new object on every render
  const canvasStyle = useMemo(
    () => ({
      transform: `scale(${zoom}) translate(${pan.x / zoom}px, ${pan.y / zoom}px)`,
      transformOrigin: "top left" as const,
    }),
    [zoom, pan.x, pan.y],
  );

  // Memoize container cursor style
  const containerStyle = useMemo(
    () => ({
      cursor: isDragging ? "grabbing" : "grab",
    }),
    [isDragging],
  );

  // Initialize YUV renderer
  useEffect(() => {
    if (canvasRef.current) {
      rendererRef.current = new YUVRenderer(canvasRef.current);
    }
    return () => {
      rendererRef.current?.dispose?.();
      rendererRef.current = null;
    };
  }, []);

  // Render frame to canvas
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) {
      logger.warn("Failed to get 2D context from canvas element");
      return;
    }

    // Determine render mode: YUV or Image
    const useYUV = yuvData && yuvData.y.length > 0;
    const source = useYUV ? yuvData : frameImage;

    if (!source) return;

    // Set canvas size
    const width = useYUV ? yuvData.width : (frameImage?.width ?? 0);
    const height = useYUV ? yuvData.height : (frameImage?.height ?? 0);

    if (canvas.width !== width || canvas.height !== height) {
      canvas.width = width;
      canvas.height = height;
    }

    // Keep WebGL overlay canvas in sync with the main canvas size
    const wgl = webglCanvasRef.current;
    if (wgl && (wgl.width !== width || wgl.height !== height)) {
      wgl.width = width;
      wgl.height = height;
    }

    // Clear canvas
    ctx.fillStyle = "#000";
    ctx.fillRect(0, 0, canvas.width, canvas.height);

    // Render source
    if (useYUV && yuvData && rendererRef.current) {
      const frameToRender = applyChannelMode(yuvData, channelMode);
      rendererRef.current.render(frameToRender, colorspace);
    } else if (frameImage) {
      // Render image as fallback
      ctx.drawImage(frameImage, 0, 0);
    }

    // Render mode overlay + active info overlays on top
    const overlayOpts: OverlayRenderOptionsExtended = {
      mode: currentMode,
      frame: currentFrame,
      canvas,
      ctx,
      activeOverlays,
      av1Features,
      webglCanvas: webglCanvasRef.current ?? undefined,
    };
    renderModeOverlay(overlayOpts);
  }, [
    frameImage,
    yuvData,
    currentMode,
    currentFrame,
    activeOverlays,
    av1Features,
    colorspace,
    channelMode,
  ]);

  return (
    <div
      className="yuv-canvas-container"
      onWheel={onWheel}
      onMouseDown={onMouseDown}
      onMouseMove={onMouseMove}
      onMouseUp={onMouseUp}
      onMouseLeave={onMouseUp}
      style={containerStyle}
    >
      <canvas
        ref={canvasRef}
        width={yuvData?.width ?? frameImage?.width ?? 640}
        height={yuvData?.height ?? frameImage?.height ?? 360}
        className="yuv-canvas"
        style={canvasStyle}
        role="img"
        aria-label={`Video frame ${currentFrameIndex}`}
      />
      <canvas
        ref={webglCanvasRef}
        width={yuvData?.width ?? frameImage?.width ?? 640}
        height={yuvData?.height ?? frameImage?.height ?? 360}
        className="yuv-canvas yuv-canvas--webgl"
        style={{
          ...canvasStyle,
          position: "absolute",
          top: 0,
          left: 0,
          pointerEvents: "none",
        }}
        aria-hidden
      />
    </div>
  );
});
