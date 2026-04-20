/**
 * Main Video Viewer Panel
 *
 * Core video display with frame navigation
 * Main viewer
 *
 * Split into subcomponents for better maintainability:
 * - VideoCanvas: Canvas rendering with zoom/pan
 * - FrameNavigationControls: Frame navigation buttons and input
 * - PlaybackControls: Play/pause and speed controls
 * - ModeSelector: Visualization mode dropdown
 * - ZoomControls: Zoom in/out/reset buttons
 * - StatusBar: Bottom status bar with info
 */

import { useState, useRef, useEffect, useCallback, memo, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useMode } from "../../../contexts/ModeContext";
import { CodecBadge } from "./ModeSelector";
import { OverlayToggleBar } from "./OverlayToggleBar";
import { CodingFlowView } from "../../Player/views/CodingFlowView";
import { DeblockingView } from "../../Player/views/DeblockingView";
import { ResidualsView } from "../../Player/views/ResidualsView";
import { AV1FeaturesView } from "../../Player/views/AV1FeaturesView";
import { useFrameData } from "../../../contexts/FrameDataContext";
import { createLogger } from "../../../utils/logger";
import { useCanvasInteraction } from "../../../hooks/useCanvasInteraction";
import { useAv1Features } from "../../../hooks/useAv1Features";
import { ZOOM, TIMING } from "../../../constants/ui";
import { VideoCanvas } from "./VideoCanvas";
import {
  YUVFrame,
  Colorspace,
  type ChannelMode,
} from "../../../utils/yuvRenderer";
import { FrameNavigationControls } from "./FrameNavigationControls";
import { PlaybackControls } from "./PlaybackControls";
import { ModeSelector } from "./ModeSelector";
import { ZoomControls } from "./ZoomControls";
import { StatusBar } from "./StatusBar";
import type {
  DecodedFrameData,
  FrameAnalysisData,
  YUVFrameData,
} from "../../../types/video";

import "./YuvViewerPanel.css";

const logger = createLogger("YuvViewerPanel");

/**
 * Convert YUVFrameData from backend to YUVFrame for renderer
 * Decodes base64 strings to Uint8Array
 */
function convertYUVDataToYUVFrame(data: YUVFrameData): YUVFrame {
  const base64ToUint8 = (base64: string): Uint8Array => {
    const binaryString = atob(base64);
    const bytes = new Uint8Array(binaryString.length);
    for (let i = 0; i < binaryString.length; i++) {
      bytes[i] = binaryString.charCodeAt(i);
    }
    return bytes;
  };

  // Extract chroma subsampling from data (default to '420' if not provided)
  const chromaSubsampling: "420" | "422" | "444" =
    (data as YUVFrameData & { chroma_subsampling?: "420" | "422" | "444" })
      .chroma_subsampling || "420";

  // Handle null U/V planes - create empty arrays instead of null
  const uPlane = data.u_plane ? base64ToUint8(data.u_plane) : new Uint8Array(0);
  const vPlane = data.v_plane ? base64ToUint8(data.v_plane) : new Uint8Array(0);

  const frame: YUVFrame = {
    width: data.width,
    height: data.height,
    y: base64ToUint8(data.y_plane),
    u: uPlane,
    v: vPlane,
    yStride: data.y_stride,
    uStride: data.u_stride,
    vStride: data.v_stride,
    chromaSubsampling,
  };

  logger.debug("convertYUVDataToYUVFrame:", {
    width: frame.width,
    height: frame.height,
    yLength: frame.y.length,
    uLength: frame.u.length,
    vLength: frame.v.length,
    yStride: frame.yStride,
    uStride: frame.uStride,
    vStride: frame.vStride,
    chromaSubsampling: frame.chromaSubsampling,
    bitDepth: data.bit_depth,
  });

  return frame;
}

interface YuvViewerPanelProps {
  currentFrameIndex: number;
  totalFrames: number;
  onFrameChange: (frameIndex: number) => void;
}

export const YuvViewerPanel = memo(function YuvViewerPanel({
  currentFrameIndex,
  totalFrames,
  onFrameChange,
}: YuvViewerPanelProps) {
  const {
    currentMode,
    setMode,
    availableModes,
    availableOverlays,
    activeOverlays,
    toggleOverlay,
    activeCodec,
    handleFKey,
  } = useMode();
  const { frames, setFrames } = useFrameData();

  // Image and loading state
  const [frameImage, setFrameImage] = useState<HTMLImageElement | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  // Retry counter — incrementing triggers a reload via useEffect
  const [retryCount, setRetryCount] = useState(0);

  // YUV data state (more efficient than RGB conversion)
  const [yuvData, setYuvData] = useState<YUVFrameData | null>(null);

  // Analysis data state
  const [, setFrameAnalysis] = useState<FrameAnalysisData | null>(null);

  // Canvas interaction (zoom, pan, drag)
  const {
    zoom,
    pan,
    isDragging,
    zoomIn,
    zoomOut,
    resetZoom,
    handlers: canvasHandlers,
  } = useCanvasInteraction({
    minZoom: ZOOM.MIN,
    maxZoom: ZOOM.MAX,
    zoomStep: ZOOM.STEP,
    requireModifierKey: true,
  });

  // Playback state
  const [isPlaying, setIsPlaying] = useState(false);
  const [isLooping, setIsLooping] = useState(false);
  const [playbackSpeed, setPlaybackSpeed] = useState(1);
  const playbackTimerRef = useRef<number | null>(null);

  // Color space and channel display
  const [colorspace, setColorspace] = useState<Colorspace>(Colorspace.BT709);
  const [channelMode, setChannelMode] = useState<ChannelMode>("all");

  // Load frame and analysis data when currentFrameIndex changes
  useEffect(() => {
    let cancelled = false;

    const loadFrame = async (frameIndex: number) => {
      setIsLoading(true);
      setLoadError(null);
      try {
        // Try YUV first (more efficient)
        const yuvResult = await invoke<YUVFrameData>("get_decoded_frame_yuv", {
          frameIndex,
        });

        if (cancelled) return;

        if (yuvResult && yuvResult.success && yuvResult.y_plane) {
          // Successfully got YUV data
          setYuvData(yuvResult);
          setFrameImage(null); // Clear RGB image
          logger.debug(
            "Loaded YUV frame:",
            frameIndex,
            "size:",
            yuvResult.width,
            "x",
            yuvResult.height,
          );
        } else {
          // Fallback to RGB
          logger.debug("YUV not available, falling back to RGB");
          const result = await invoke<DecodedFrameData>("get_decoded_frame", {
            frameIndex,
          });

          if (cancelled) return;

          if (result && result.success && result.frame_data) {
            const img = new Image();
            img.onload = () => {
              if (!cancelled) {
                setFrameImage(img);
                setIsLoading(false);
              }
            };
            img.onerror = () => {
              if (!cancelled) {
                logger.error(
                  "Failed to decode frame image for frame:",
                  frameIndex,
                );
                setIsLoading(false);
                setFrameImage(null);
                setLoadError("Failed to decode frame image");
              }
            };
            img.src = `data:image/png;base64,${result.frame_data}`;
            // Return early — isLoading will be cleared in img callbacks
            return;
          } else {
            logger.error("Failed to load frame:", result.error);
            setLoadError(result.error || "Failed to load frame");
          }
        }
      } catch (error) {
        if (cancelled) return;
        logger.error("Failed to load frame:", error);
        setLoadError("Failed to load frame — check console for details");
      } finally {
        if (!cancelled) setIsLoading(false);
      }
    };

    const loadFrameAnalysis = async (frameIndex: number) => {
      try {
        const result = await invoke<FrameAnalysisData>("get_frame_analysis", {
          frameIndex,
        });

        if (cancelled) return;

        // Update frame with analysis data
        setFrameAnalysis(result);

        // Merge analysis data into frames context
        setFrames((prevFrames) => {
          const newFrames = [...prevFrames];
          if (newFrames[frameIndex]) {
            newFrames[frameIndex] = {
              ...newFrames[frameIndex],
              qp_grid: result.qp_grid,
              mv_grid: result.mv_grid,
              partition_grid: result.partition_grid,
              prediction_mode_grid: result.prediction_mode_grid,
              transform_grid: result.transform_grid,
              width: result.width,
              height: result.height,
            };
          }
          return newFrames;
        });
      } catch (error) {
        if (cancelled) return;
        logger.error("Failed to load frame analysis:", error);
      }
    };

    // Run both loads in parallel
    Promise.all([
      loadFrame(currentFrameIndex),
      loadFrameAnalysis(currentFrameIndex),
    ]).catch((err) => {
      logger.error("Frame load error:", err);
    });

    return () => {
      cancelled = true;
    };
  }, [currentFrameIndex, retryCount, setFrames]);

  // Frame navigation callbacks
  const goToPrevFrame = useCallback(() => {
    if (currentFrameIndex > 0) {
      onFrameChange(currentFrameIndex - 1);
    }
  }, [currentFrameIndex, onFrameChange]);

  const goToNextFrame = useCallback(() => {
    if (currentFrameIndex < totalFrames - 1) {
      onFrameChange(currentFrameIndex + 1);
    }
  }, [currentFrameIndex, totalFrames, onFrameChange]);

  const goToFirstFrame = useCallback(() => {
    onFrameChange(0);
  }, [onFrameChange]);

  const goToLastFrame = useCallback(() => {
    onFrameChange(totalFrames - 1);
  }, [totalFrames, onFrameChange]);

  // Playback control
  const togglePlay = useCallback(() => {
    setIsPlaying((prev) => !prev);
  }, []);

  // Handle playback with timer
  useEffect(() => {
    if (!isPlaying) {
      if (playbackTimerRef.current) {
        clearTimeout(playbackTimerRef.current);
        playbackTimerRef.current = null;
      }
      return;
    }

    const interval = TIMING.AUTO_PLAY_INTERVAL / playbackSpeed;
    playbackTimerRef.current = setTimeout(() => {
      if (currentFrameIndex < totalFrames - 1) {
        onFrameChange(currentFrameIndex + 1);
      } else if (isLooping) {
        onFrameChange(0);
      } else {
        setIsPlaying(false);
      }
    }, interval) as unknown as number;

    return () => {
      if (playbackTimerRef.current) {
        clearTimeout(playbackTimerRef.current);
      }
    };
  }, [
    isPlaying,
    isLooping,
    currentFrameIndex,
    totalFrames,
    playbackSpeed,
    onFrameChange,
  ]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (playbackTimerRef.current) {
        clearTimeout(playbackTimerRef.current);
      }
    };
  }, []);

  // Color space, channel mode, and loop events from menus/keyboard
  useEffect(() => {
    const onColorBT601 = () => setColorspace(Colorspace.BT601);
    const onColorBT709 = () => setColorspace(Colorspace.BT709);
    const onColorBT2020 = () => setColorspace(Colorspace.BT2020);
    const onColorYuvRgb = () => setColorspace(Colorspace.BT709); // YUV-as-RGB: same matrix but skip offset
    const onColorYuvGbr = () => setColorspace(Colorspace.BT709); // placeholder
    const onChannelY = () => setChannelMode((m) => (m === "Y" ? "all" : "Y"));
    const onChannelU = () => setChannelMode((m) => (m === "U" ? "all" : "U"));
    const onChannelV = () => setChannelMode((m) => (m === "V" ? "all" : "V"));
    const onChannelAll = () => setChannelMode("all");
    const onLoopPlayback = () => setIsLooping((v) => !v);

    window.addEventListener("menu-color-bt601", onColorBT601);
    window.addEventListener("menu-color-bt709", onColorBT709);
    window.addEventListener("menu-color-bt2020", onColorBT2020);
    window.addEventListener("menu-color-yuv-rgb", onColorYuvRgb);
    window.addEventListener("menu-color-yuv-gbr", onColorYuvGbr);
    window.addEventListener("viewer-channel-y", onChannelY);
    window.addEventListener("viewer-channel-u", onChannelU);
    window.addEventListener("viewer-channel-v", onChannelV);
    window.addEventListener("viewer-channel-all", onChannelAll);
    window.addEventListener("menu-loop-playback", onLoopPlayback);
    return () => {
      window.removeEventListener("menu-color-bt601", onColorBT601);
      window.removeEventListener("menu-color-bt709", onColorBT709);
      window.removeEventListener("menu-color-bt2020", onColorBT2020);
      window.removeEventListener("menu-color-yuv-rgb", onColorYuvRgb);
      window.removeEventListener("menu-color-yuv-gbr", onColorYuvGbr);
      window.removeEventListener("viewer-channel-y", onChannelY);
      window.removeEventListener("viewer-channel-u", onChannelU);
      window.removeEventListener("viewer-channel-v", onChannelV);
      window.removeEventListener("viewer-channel-all", onChannelAll);
      window.removeEventListener("menu-loop-playback", onLoopPlayback);
    };
  }, []);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      if (["INPUT", "SELECT", "TEXTAREA"].includes(target.tagName)) return;
      switch (e.key) {
        case " ":
          if (!e.ctrlKey && !e.metaKey && !e.shiftKey) {
            e.preventDefault();
            togglePlay();
          }
          break;
        case "ArrowLeft":
          e.preventDefault();
          goToPrevFrame();
          break;
        case "ArrowRight":
          e.preventDefault();
          goToNextFrame();
          break;
        case "Home":
          e.preventDefault();
          goToFirstFrame();
          break;
        case "End":
          e.preventDefault();
          goToLastFrame();
          break;
        case "+":
        case "=":
          e.preventDefault();
          zoomIn();
          break;
        case "-":
          e.preventDefault();
          zoomOut();
          break;
        case "0":
          if (e.ctrlKey || e.metaKey) {
            e.preventDefault();
            resetZoom();
          }
          break;
        case "F1":
        case "F2":
        case "F3":
        case "F4":
        case "F5":
        case "F6":
        case "F7":
        case "F8":
        case "F9":
        case "F10":
        case "F11":
        case "F12": {
          const fNum = parseInt(e.key.slice(1), 10);
          if (handleFKey(fNum)) e.preventDefault();
          break;
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [
    togglePlay,
    goToPrevFrame,
    goToNextFrame,
    goToFirstFrame,
    goToLastFrame,
    zoomIn,
    zoomOut,
    resetZoom,
    handleFKey,
  ]);

  // Memoize YUV conversion to avoid re-running on every render
  const convertedYuvFrame = useMemo(
    () => (yuvData ? convertYUVDataToYUVFrame(yuvData) : undefined),
    [yuvData],
  );

  const currentFrame = frames[currentFrameIndex] || null;

  // Fetch AV1 advanced features when in an AV1-specific mode
  const { av1Features } = useAv1Features(
    currentFrameIndex,
    activeCodec,
    currentMode,
  );

  return (
    <div className="yuv-viewer">
      {/* Toolbar */}
      <div className="yuv-toolbar">
        <FrameNavigationControls
          currentFrameIndex={currentFrameIndex}
          totalFrames={totalFrames}
          onFirstFrame={goToFirstFrame}
          onPrevFrame={goToPrevFrame}
          onNextFrame={goToNextFrame}
          onLastFrame={goToLastFrame}
          onFrameChange={onFrameChange}
        />

        <div className="yuv-toolbar-spacer"></div>

        <PlaybackControls
          isPlaying={isPlaying}
          playbackSpeed={playbackSpeed}
          onTogglePlay={togglePlay}
          onSpeedChange={setPlaybackSpeed}
        />

        <div className="yuv-toolbar-spacer"></div>

        <CodecBadge codec={activeCodec} />

        <ModeSelector
          currentMode={currentMode}
          onModeChange={setMode}
          availableModes={availableModes}
        />

        <OverlayToggleBar
          availableOverlays={availableOverlays}
          activeOverlays={activeOverlays}
          onToggle={toggleOverlay}
        />

        <div className="yuv-toolbar-spacer"></div>

        <ZoomControls
          zoom={zoom}
          onZoomIn={zoomIn}
          onZoomOut={zoomOut}
          onResetZoom={resetZoom}
        />
      </div>

      {/* Canvas Area or Analysis View */}
      {currentMode === "coding-flow" ? (
        <div className="yuv-analysis-view-container">
          <CodingFlowView frame={currentFrame} />
        </div>
      ) : currentMode === "deblocking" || currentMode === "loop-filter" ? (
        <div className="yuv-analysis-view-container">
          <DeblockingView
            frame={currentFrame}
            width={frameImage?.width ?? 1920}
            height={frameImage?.height ?? 1080}
          />
        </div>
      ) : currentMode === "residuals" ? (
        <div className="yuv-analysis-view-container">
          <ResidualsView
            frame={currentFrame}
            width={frameImage?.width ?? 1920}
            height={frameImage?.height ?? 1080}
          />
        </div>
      ) : currentMode === "av1-features" ? (
        <div className="yuv-analysis-view-container">
          <AV1FeaturesView
            frame={currentFrame}
            width={frameImage?.width ?? 1920}
            height={frameImage?.height ?? 1080}
            subMode={currentMode}
          />
        </div>
      ) : (
        <VideoCanvas
          frameImage={frameImage}
          currentFrameIndex={currentFrameIndex}
          currentFrame={currentFrame}
          currentMode={currentMode}
          zoom={zoom}
          pan={pan}
          onWheel={canvasHandlers.onWheel}
          onMouseDown={canvasHandlers.onMouseDown}
          onMouseMove={canvasHandlers.onMouseMove}
          onMouseUp={canvasHandlers.onMouseUp}
          isDragging={isDragging}
          yuvData={convertedYuvFrame}
          activeOverlays={activeOverlays}
          av1Features={av1Features ?? undefined}
          colorspace={colorspace}
          channelMode={channelMode}
        />
      )}

      {/* Loading and Placeholder States */}
      {isLoading && (
        <div className="yuv-loading-overlay">
          <span className="codicon codicon-loading codicon-spin"></span>
          <span>Loading frame {currentFrameIndex}...</span>
        </div>
      )}

      {loadError && !isLoading && (
        <div className="yuv-error-overlay">
          <span className="codicon codicon-error"></span>
          <span className="yuv-error-message">{loadError}</span>
          <button
            className="yuv-error-retry"
            onClick={() => setRetryCount((c) => c + 1)}
          >
            <span className="codicon codicon-refresh"></span>
            Retry
          </button>
        </div>
      )}

      {!frameImage && !yuvData && !isLoading && !loadError && (
        <div className="yuv-placeholder-overlay">
          <span className="codicon codicon-device-camera"></span>
          <span>No frame loaded</span>
          <span style={{ fontSize: "11px", opacity: 0.7 }}>
            Use arrow keys or toolbar to navigate
          </span>
        </div>
      )}

      {/* Status Bar */}
      <StatusBar
        currentFrameIndex={currentFrameIndex}
        totalFrames={totalFrames}
        currentMode={currentMode}
        zoom={zoom}
        isPlaying={isPlaying}
        playbackSpeed={playbackSpeed}
      />
    </div>
  );
});
