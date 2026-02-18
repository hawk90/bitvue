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
import type { FrameInfo } from "../../../types/video";
import {
  YUVRenderer,
  type YUVFrame,
  Colorspace,
} from "../../../utils/yuvRenderer";
import { createLogger } from "../../../utils/logger";

const logger = createLogger("VideoCanvas");

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
}: VideoCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
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
    const width = useYUV ? yuvData.width : frameImage.width;
    const height = useYUV ? yuvData.height : frameImage.height;

    if (canvas.width !== width || canvas.height !== height) {
      canvas.width = width;
      canvas.height = height;
    }

    // Clear canvas
    ctx.fillStyle = "#000";
    ctx.fillRect(0, 0, canvas.width, canvas.height);

    // Render source
    if (useYUV && yuvData && rendererRef.current) {
      // Render YUV data using the renderer
      rendererRef.current.render(yuvData, Colorspace.BT709);
    } else if (frameImage) {
      // Render image as fallback
      ctx.drawImage(frameImage, 0, 0);
    }

    // Render mode overlay on top
    renderModeOverlay({
      mode: currentMode,
      frame: currentFrame,
      canvas,
      ctx,
    });
  }, [frameImage, yuvData, currentMode, currentFrame, currentFrameIndex]);

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
      />
    </div>
  );
});
