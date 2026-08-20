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

import { useState, useRef, useEffect, useCallback, memo } from "react";
import {
  getDecodedFrameYuvCancellable,
  getDebugYuvFrame,
  getFrameAnalysis,
  bridgeYuvToFrame,
  getContextMenuItems,
  type ContextMenuItemWire,
} from "../../../services/electronBridgeService";
import { useExportEvidenceBundle } from "../../../hooks/useExportEvidenceBundle";
import { ContextMenu } from "../../ContextMenu";
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
import { useYuvDiff } from "../../../contexts/YuvDiffContext";
import { FrameNavigationControls } from "./FrameNavigationControls";
import { PlaybackControls } from "./PlaybackControls";
import { ModeSelector } from "./ModeSelector";
import { ZoomControls } from "./ZoomControls";
import { StatusBar } from "./StatusBar";
import type { FrameAnalysisData } from "../../../types/video";

import "./YuvViewerPanel.css";

const logger = createLogger("YuvViewerPanel");

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

  // Decoded-pixel state, via the Electron bridge (no base64) -- getDecodedFrameYuv for the real
  // stream, or getDebugYuvFrame when a debug YUV reference is loaded (see the debugYuvLoaded
  // branch below); both converge on the same bridgeYuvToFrame conversion.
  const [decodedFrame, setDecodedFrame] = useState<YUVFrame | null>(null);

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

  // Debug YUV diff state
  const {
    isLoaded: debugYuvLoaded,
    displayMode: debugDisplayMode,
    amplifyFactor: debugAmplifyFactor,
  } = useYuvDiff();

  // Right-click context menu (Phase 7.6, "Player" scope) -- see ContextMenu component doc.
  const exportEvidence = useExportEvidenceBundle();
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    items: ContextMenuItemWire[];
  } | null>(null);

  const handleCanvasContextMenu = useCallback((event: React.MouseEvent) => {
    event.preventDefault();
    // TODO(Phase 7.6 follow-up): thread real selection state in (SelectionContext isn't
    // currently a dependency of this component, and this is the only place that would need it --
    // deferred rather than adding a hard context dependency here for one guard on one item
    // (Player's "toggle_detail") that isn't wired to a real action yet anyway).
    const hasSelection = false;
    const hasByteRange = false;
    const x = event.clientX;
    const y = event.clientY;
    getContextMenuItems("Player", hasSelection, hasByteRange)
      .then((items) => setContextMenu({ x, y, items }))
      .catch(() => setContextMenu(null));
  }, []);

  const handleContextMenuSelect = useCallback(
    (command: string) => {
      if (command === "Export.EvidenceBundle") {
        void exportEvidence();
      }
      // Other Player-scope commands (Toggle.DetailMode, Copy.Selection) aren't wired to a real
      // action yet -- deliberately out of scope for this pass (see Phase 7.6 doc).
    },
    [exportEvidence],
  );

  // Load frame and analysis data when currentFrameIndex changes
  useEffect(() => {
    let cancelled = false;
    // Set only while a real (non-debug-YUV) decode is in flight -- calling this on cleanup tells
    // bitvue-sidecar to stop decoding a frame the user already scrubbed past, instead of just
    // discarding the result client-side once it eventually arrives (see decode_session.rs's
    // cancel_flag wiring and SidecarClient.getDecodedFrameYuvCancellable's doc).
    let cancelDecode: (() => void) | null = null;

    const loadFrame = async (frameIndex: number) => {
      setIsLoading(true);
      setLoadError(null);
      try {
        // When debug YUV is loaded, fetch via the bridge's getDebugYuvFrame instead -- same
        // raw-bytes wire shape as getDecodedFrameYuv, so both paths converge on bridgeYuvToFrame.
        if (debugYuvLoaded) {
          const debugFrame = await getDebugYuvFrame(
            frameIndex,
            debugDisplayMode,
            debugDisplayMode === "amplified" ? debugAmplifyFactor : undefined,
          );
          if (cancelled) return;
          setDecodedFrame(bridgeYuvToFrame(debugFrame));
          setFrameImage(null);
          setIsLoading(false);
          return;
        }

        // Real decode path, via the Electron bridge -- AV1/IVF only, matches this app's stream
        // "A" convention (see FileStateContext/electronBridgeService callers).
        const { promise, cancel } = getDecodedFrameYuvCancellable(
          "A",
          frameIndex,
        );
        cancelDecode = cancel;
        const decoded = await promise;

        if (cancelled) return;

        setDecodedFrame(bridgeYuvToFrame(decoded));
        setFrameImage(null);
        logger.debug(
          "Loaded YUV frame:",
          frameIndex,
          "size:",
          decoded.width,
          "x",
          decoded.height,
        );
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
        const result = await getFrameAnalysis(frameIndex);

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
              energy_grid: result.energy_grid,
              mv_grid: result.mv_grid,
              partition_grid: result.partition_grid,
              prediction_mode_grid: result.prediction_mode_grid,
              transform_grid: result.transform_grid,
              mb_type_grid: result.mb_type_grid,
              ref_idx_grid: result.ref_idx_grid,
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
      cancelDecode?.();
    };
  }, [
    currentFrameIndex,
    retryCount,
    setFrames,
    debugYuvLoaded,
    debugDisplayMode,
    debugAmplifyFactor,
  ]);

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

        <div className="yuv-toolbar-divider"></div>

        <PlaybackControls
          isPlaying={isPlaying}
          playbackSpeed={playbackSpeed}
          onTogglePlay={togglePlay}
          onSpeedChange={setPlaybackSpeed}
        />

        <div className="yuv-toolbar-divider"></div>

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

        {/* Single growing spacer -- previously 3 independent `flex:1` spacers sat between every
            group, so whenever a middle group rendered empty (e.g. OverlayToggleBar has nothing
            for modes with no overlays, or CodecBadge before a file was ever opened -- see its own
            fix note), the remaining spacers still split 100% of the leftover width evenly between
            them, producing huge dead gaps between the few groups that *did* render. Pushing only
            ZoomControls to the right with one spacer keeps everything else in a single left-
            aligned cluster regardless of which optional groups are present. */}
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
          <CodingFlowView
            frame={currentFrame}
            codec={activeCodec ?? undefined}
          />
        </div>
      ) : currentMode === "deblocking" || currentMode === "loop-filter" ? (
        <div className="yuv-analysis-view-container">
          <DeblockingView
            frame={currentFrame}
            width={frameImage?.width ?? 1920}
            height={frameImage?.height ?? 1080}
            codec={activeCodec ?? undefined}
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
      ) : currentMode === "av1-features" ||
        currentMode === "cdef-filter" ||
        currentMode === "loop-restoration" ||
        currentMode === "film-grain" ||
        currentMode === "super-res" ? (
        <div className="yuv-analysis-view-container">
          {/* AV1FeaturesView already has real, complete CDEF/LoopRestoration/FilmGrain/SuperRes
              sections (its show* props gate each independently) -- the individual per-feature
              F-keys (cdef-filter/loop-restoration/film-grain/super-res) used to fall through to
              the plain VideoCanvas below with no analysis rendered at all, despite the data
              already being fetched (see useAv1Features's mode gate above). "av1-features" (the
              catch-all, reachable via the info-overlay menu, not an F-key) still shows all four. */}
          <AV1FeaturesView
            frame={currentFrame}
            width={frameImage?.width ?? 1920}
            height={frameImage?.height ?? 1080}
            showCdef={
              currentMode === "av1-features" || currentMode === "cdef-filter"
            }
            showLoopRestoration={
              currentMode === "av1-features" ||
              currentMode === "loop-restoration"
            }
            showFilmGrain={
              currentMode === "av1-features" || currentMode === "film-grain"
            }
            showSuperRes={
              currentMode === "av1-features" || currentMode === "super-res"
            }
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
          onContextMenu={handleCanvasContextMenu}
          isDragging={isDragging}
          yuvData={decodedFrame ?? undefined}
          activeOverlays={activeOverlays}
          av1Features={av1Features ?? undefined}
          colorspace={colorspace}
          channelMode={channelMode}
        />
      )}

      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          items={contextMenu.items}
          onSelect={handleContextMenuSelect}
          onClose={() => setContextMenu(null)}
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

      {!frameImage && !decodedFrame && !isLoading && !loadError && (
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
        availableModes={availableModes}
      />
    </div>
  );
});
