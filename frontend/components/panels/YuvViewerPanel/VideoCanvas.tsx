/**
 * Video Canvas Component
 *
 * Handles canvas rendering with zoom and pan support
 * Includes mouse wheel zoom and drag-to-pan functionality
 * Supports both image-based and YUV-based rendering
 */

import { useRef, useEffect, useState, memo, useMemo } from "react";
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
import { WebGpuFrameRenderer } from "../../../utils/gpu/frameRenderer";
import { isWebGpuDisabledByFlag } from "../../../utils/gpu/featureFlag";
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
  onContextMenu?: (e: React.MouseEvent) => void;
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
  onContextMenu,
  isDragging,
  yuvData,
  activeOverlays,
  av1Features,
  colorspace = Colorspace.BT709,
  channelMode = "all",
}: VideoCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const webglCanvasRef = useRef<HTMLCanvasElement>(null);
  const webgpuCanvasRef = useRef<HTMLCanvasElement>(null);
  const rendererRef = useRef<YUVRenderer | null>(null);
  const gpuRendererRef = useRef<WebGpuFrameRenderer | null>(null);
  // Decided once at mount (see the attach effect below); starts "canvas2d" so the very first
  // frame never blocks on the async GPU device request, and flips to "webgpu" only once a real
  // device + configured canvas context are confirmed. Never flips back at runtime -- a device
  // lost mid-session is logged (device.ts) but Phase 1 doesn't attempt live backend recovery.
  const [backend, setBackend] = useState<"canvas2d" | "webgpu">("canvas2d");

  // Memoize canvas style to avoid creating new object on every render
  const canvasStyle = useMemo(
    () => ({
      transform: `scale(${zoom}) translate(${pan.x / zoom}px, ${pan.y / zoom}px)`,
      transformOrigin: "top left" as const,
    }),
    [zoom, pan.x, pan.y],
  );

  // Logical (frame-native) display size -- same value the render effect below sizes the canvas's
  // *pixel buffer* from (scaled by devicePixelRatio there). Pinning the element's CSS size to
  // this, separately from that larger buffer, is what keeps the on-screen size unchanged while
  // the internal resolution goes up for a sharp (non-blurry) render on high-DPI displays.
  const logicalWidth = yuvData?.width ?? frameImage?.width ?? 640;
  const logicalHeight = yuvData?.height ?? frameImage?.height ?? 360;
  const mainCanvasStyle = useMemo(
    () => ({
      ...canvasStyle,
      width: `${logicalWidth}px`,
      height: `${logicalHeight}px`,
    }),
    [canvasStyle, logicalWidth, logicalHeight],
  );

  // Memoize container cursor style
  const containerStyle = useMemo(
    () => ({
      cursor: isDragging ? "grabbing" : "grab",
    }),
    [isDragging],
  );

  // Initialize YUV renderer. No explicit teardown needed on unmount -- YUVRenderer only holds
  // canvas/context/ImageData references (no timers, listeners, or GPU handles), so it's plain
  // garbage-collected; the previous `.dispose?.()` call here referenced a method that never
  // existed on the class (silently absorbed by the optional call, a real but harmless bug).
  useEffect(() => {
    if (canvasRef.current) {
      rendererRef.current = new YUVRenderer(canvasRef.current);
    }
    return () => {
      rendererRef.current = null;
    };
  }, []);

  // Attempt the WebGPU backend once at mount. Kept behind a kill switch
  // (isWebGpuDisabledByFlag) since WebGPU support in Electron/Chromium is confirmed flaky on
  // Linux -- a failed/declined attach silently leaves `backend` at "canvas2d", which the render
  // effect below treats identically to today's pre-WebGPU behavior.
  useEffect(() => {
    if (isWebGpuDisabledByFlag()) return;
    const canvas = webgpuCanvasRef.current;
    if (!canvas) return;

    let cancelled = false;
    const renderer = new WebGpuFrameRenderer();
    gpuRendererRef.current = renderer;
    void renderer.attachCanvas(canvas).then((ok) => {
      if (!cancelled && ok) {
        setBackend("webgpu");
      }
    });

    return () => {
      cancelled = true;
      renderer.destroy();
      gpuRendererRef.current = null;
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

    // Logical (frame-native) size -- what every overlay renderer's coordinate math already
    // assumes (frame.partition_grid/prediction_mode_grid/etc. blocks are in this space), and
    // what the canvas element's own CSS size stays pinned to below so it doesn't visually grow.
    const width = useYUV ? yuvData.width : (frameImage?.width ?? 0);
    const height = useYUV ? yuvData.height : (frameImage?.height ?? 0);

    // Size the canvas's actual pixel buffer for the display's real pixel density -- a plain
    // `canvas.width = width` buffer stays crisp only at 1 device pixel per buffer pixel; on a
    // 2x/3x (Retina) display the browser has to upscale that low-res buffer to cover 2x/3x as
    // many physical pixels, which is what made every overlay's block/boundary lines (and the
    // decoded picture itself) look blurry and thick -- confirmed by a real before/after
    // screenshot comparison. `canvasStyle` below sets explicit CSS width/height back to the
    // logical size so the element's on-screen size is unchanged; only its internal resolution
    // goes up.
    const dpr = window.devicePixelRatio || 1;
    const bufferWidth = Math.round(width * dpr);
    const bufferHeight = Math.round(height * dpr);
    if (canvas.width !== bufferWidth || canvas.height !== bufferHeight) {
      canvas.width = bufferWidth;
      canvas.height = bufferHeight;
    }

    // WebGL overlay canvas (MVFieldOverlay's high-density path only) stays at logical size for
    // now -- out of scope for this pass, it has its own GL viewport/coordinate handling that
    // needs separate verification before scaling it too.
    const wgl = webglCanvasRef.current;
    if (wgl && (wgl.width !== width || wgl.height !== height)) {
      wgl.width = width;
      wgl.height = height;
    }

    // WebGPU base-layer canvas -- same dpr-scaled buffer size as the 2D canvas above it (both
    // occupy the same grid cell; see YuvViewerPanel.css's .yuv-canvas-container), sized
    // unconditionally so it's ready the moment the mount effect's attach resolves.
    const gpu = webgpuCanvasRef.current;
    if (gpu && (gpu.width !== bufferWidth || gpu.height !== bufferHeight)) {
      gpu.width = bufferWidth;
      gpu.height = bufferHeight;
    }

    const useGpuForThisFrame =
      useYUV &&
      backend === "webgpu" &&
      (gpuRendererRef.current?.isReady() ?? false);

    // Identity transform for the buffer-resolution clear/video draw below -- both explicitly
    // target canvas.width/canvas.height (physical, already dpr-scaled), so no additional
    // transform scaling should apply here (that would double-scale them).
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    if (useGpuForThisFrame) {
      // The WebGPU canvas underneath (same grid cell) draws the opaque base frame -- this
      // context now only needs to be transparent so that layer shows through, not painted over.
      ctx.clearRect(0, 0, canvas.width, canvas.height);
    } else {
      ctx.fillStyle = "#000";
      ctx.fillRect(0, 0, canvas.width, canvas.height);
    }

    // Render source
    if (useGpuForThisFrame && yuvData) {
      const frameToRender = applyChannelMode(yuvData, channelMode);
      gpuRendererRef.current?.render(frameToRender, colorspace);
    } else if (useYUV && yuvData && rendererRef.current) {
      const frameToRender = applyChannelMode(yuvData, channelMode);
      rendererRef.current.render(frameToRender, colorspace);
    } else if (frameImage) {
      // Render image as fallback -- explicit destination size (canvas.width/height, physical)
      // now that they may differ from the image's own native pixel size.
      ctx.drawImage(frameImage, 0, 0, canvas.width, canvas.height);
    }

    // Render mode overlay + active info overlays on top. Scaled by dpr so every renderer's
    // existing frame-native coordinate math (fillRect/strokeRect/fillText, all vector draws --
    // unlike the video's putImageData-based path above, these *do* respect the canvas transform)
    // fills the higher-res buffer automatically, with no changes needed in any individual
    // renderer file.
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    const overlayOpts: OverlayRenderOptionsExtended = {
      mode: currentMode,
      frame: currentFrame,
      canvas,
      ctx,
      activeOverlays,
      av1Features,
      webglCanvas: webglCanvasRef.current ?? undefined,
      dpr,
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
    backend,
  ]);

  return (
    <div
      className="yuv-canvas-container"
      onWheel={onWheel}
      onMouseDown={onMouseDown}
      onMouseMove={onMouseMove}
      onMouseUp={onMouseUp}
      onMouseLeave={onMouseUp}
      onContextMenu={onContextMenu}
      style={containerStyle}
    >
      <canvas
        ref={webgpuCanvasRef}
        width={yuvData?.width ?? frameImage?.width ?? 640}
        height={yuvData?.height ?? frameImage?.height ?? 360}
        className="yuv-canvas yuv-canvas--webgpu"
        style={mainCanvasStyle}
        aria-hidden
      />
      <canvas
        ref={canvasRef}
        width={yuvData?.width ?? frameImage?.width ?? 640}
        height={yuvData?.height ?? frameImage?.height ?? 360}
        className="yuv-canvas"
        style={mainCanvasStyle}
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
          pointerEvents: "none",
        }}
        aria-hidden
      />
    </div>
  );
});
