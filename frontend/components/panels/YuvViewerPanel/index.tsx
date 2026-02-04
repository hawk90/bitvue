/**
 * Main Video Viewer Panel
 *
 * Core video display with frame navigation
 * VQAnalyzer-style main viewer
 *
 * Split into subcomponents for better maintainability:
 * - VideoCanvas: Canvas rendering with zoom/pan
 * - FrameNavigationControls: Frame navigation buttons and input
 * - PlaybackControls: Play/pause and speed controls
 * - ModeSelector: Visualization mode dropdown
 * - ZoomControls: Zoom in/out/reset buttons
 * - StatusBar: Bottom status bar with info
 */

import { useState, useRef, useEffect, useCallback, memo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useMode } from '../../../contexts/ModeContext';
import { useFrameData } from '../../../contexts/FrameDataContext';
import { createLogger } from '../../../utils/logger';
import { useCanvasInteraction } from '../../../hooks/useCanvasInteraction';
import { ZOOM, TIMING } from '../../../constants/ui';
import { VideoCanvas } from './VideoCanvas';
import { YUVFrame } from '../../../utils/yuvRenderer';
import { FrameNavigationControls } from './FrameNavigationControls';
import { PlaybackControls } from './PlaybackControls';
import { ModeSelector } from './ModeSelector';
import { ZoomControls } from './ZoomControls';
import { StatusBar } from './StatusBar';
import type { DecodedFrameData, FrameAnalysisData, YUVFrameData } from '../../../types/video';

import './YuvViewerPanel.css';

const logger = createLogger('YuvViewerPanel');

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
  const chromaSubsampling: '420' | '422' | '444' = (data as any).chroma_subsampling || '420';

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

  logger.debug('convertYUVDataToYUVFrame:', {
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
  const { currentMode, setMode } = useMode();
  const { frames, setFrames } = useFrameData();

  // Image and loading state
  const [frameImage, setFrameImage] = useState<HTMLImageElement | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  // YUV data state (more efficient than RGB conversion)
  const [yuvData, setYuvData] = useState<YUVFrameData | null>(null);

  // Analysis data state
  const [frameAnalysis, setFrameAnalysis] = useState<FrameAnalysisData | null>(null);

  // Canvas interaction (zoom, pan, drag)
  const { zoom, pan, isDragging, zoomIn, zoomOut, resetZoom, handlers: canvasHandlers } = useCanvasInteraction({
    minZoom: ZOOM.MIN,
    maxZoom: ZOOM.MAX,
    zoomStep: ZOOM.STEP,
    requireModifierKey: true,
  });

  // Playback state
  const [isPlaying, setIsPlaying] = useState(false);
  const [playbackSpeed, setPlaybackSpeed] = useState(1);
  const playbackTimerRef = useRef<number | null>(null);

  // Load frame and analysis data when currentFrameIndex changes
  useEffect(() => {
    loadFrame(currentFrameIndex);
    loadFrameAnalysis(currentFrameIndex);
  }, [currentFrameIndex]);

  const loadFrame = async (frameIndex: number) => {
    setIsLoading(true);
    try {
      // Try YUV first (more efficient)
      const yuvResult = await invoke<YUVFrameData>('get_decoded_frame_yuv', {
        frameIndex,
      });

      if (yuvResult && yuvResult.success && yuvResult.y_plane) {
        // Successfully got YUV data
        setYuvData(yuvResult);
        setFrameImage(null); // Clear RGB image
        setIsLoading(false);
        logger.debug('Loaded YUV frame:', frameIndex, 'size:', yuvResult.width, 'x', yuvResult.height);
      } else {
        // Fallback to RGB
        logger.debug('YUV not available, falling back to RGB');
        const result = await invoke<DecodedFrameData>('get_decoded_frame', {
          frameIndex,
        });

        if (result && result.success && result.frame_data) {
          const img = new Image();
          img.onload = () => {
            setFrameImage(img);
            setIsLoading(false);
          };
          img.onerror = () => {
            logger.error('Failed to decode frame image for frame:', frameIndex);
            setIsLoading(false);
            setFrameImage(null);
          };
          img.src = `data:image/png;base64,${result.frame_data}`;
        } else {
          logger.error('Failed to load frame:', result.error);
          setIsLoading(false);
        }
      }
    } catch (error) {
      logger.error('Failed to load frame:', error);
      setIsLoading(false);
    }
  };

  const loadFrameAnalysis = async (frameIndex: number) => {
    try {
      const result = await invoke<FrameAnalysisData>('get_frame_analysis', {
        frameIndex,
      });

      // Update frame with analysis data
      setFrameAnalysis(result);

      // Merge analysis data into frames context
      setFrames(prevFrames => {
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
      logger.error('Failed to load frame analysis:', error);
    }
  };

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
    setIsPlaying(prev => !prev);
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
      } else {
        setIsPlaying(false);
      }
    }, interval) as unknown as number;

    return () => {
      if (playbackTimerRef.current) {
        clearTimeout(playbackTimerRef.current);
      }
    };
  }, [isPlaying, currentFrameIndex, totalFrames, playbackSpeed, onFrameChange]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (playbackTimerRef.current) {
        clearTimeout(playbackTimerRef.current);
      }
    };
  }, []);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      switch (e.key) {
        case ' ':
          if (!e.ctrlKey && !e.metaKey && !e.shiftKey) {
            e.preventDefault();
            togglePlay();
          }
          break;
        case 'ArrowLeft':
          e.preventDefault();
          goToPrevFrame();
          break;
        case 'ArrowRight':
          e.preventDefault();
          goToNextFrame();
          break;
        case 'Home':
          e.preventDefault();
          goToFirstFrame();
          break;
        case 'End':
          e.preventDefault();
          goToLastFrame();
          break;
        case '+':
        case '=':
          e.preventDefault();
          zoomIn();
          break;
        case '-':
          e.preventDefault();
          zoomOut();
          break;
        case '0':
          if (e.ctrlKey || e.metaKey) {
            e.preventDefault();
            resetZoom();
          }
          break;
        case 'F1':
          e.preventDefault();
          setMode('overview');
          break;
        case 'F2':
          e.preventDefault();
          setMode('coding-flow');
          break;
        case 'F3':
          e.preventDefault();
          setMode('prediction');
          break;
        case 'F4':
          e.preventDefault();
          setMode('transform');
          break;
        case 'F5':
          e.preventDefault();
          setMode('qp-map');
          break;
        case 'F6':
          e.preventDefault();
          setMode('mv-field');
          break;
        case 'F7':
          e.preventDefault();
          setMode('reference');
          break;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [togglePlay, goToPrevFrame, goToNextFrame, goToFirstFrame, goToLastFrame, zoomIn, zoomOut, resetZoom, setMode]);

  const currentFrame = frames[currentFrameIndex] || null;

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

        <ModeSelector
          currentMode={currentMode}
          onModeChange={setMode}
        />

        <div className="yuv-toolbar-spacer"></div>

        <ZoomControls
          zoom={zoom}
          onZoomIn={zoomIn}
          onZoomOut={zoomOut}
          onResetZoom={resetZoom}
        />
      </div>

      {/* Canvas Area */}
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
        yuvData={yuvData ? convertYUVDataToYUVFrame(yuvData) : undefined}
      />

      {/* Loading and Placeholder States */}
      {isLoading && (
        <div className="yuv-loading-overlay">
          <span className="codicon codicon-loading codicon-spin"></span>
          <span>Loading frame {currentFrameIndex}...</span>
        </div>
      )}

      {!frameImage && !isLoading && (
        <div className="yuv-placeholder-overlay">
          <span className="codicon codicon-device-camera"></span>
          <span>No frame loaded</span>
          <span style={{ fontSize: '11px', opacity: 0.7 }}>
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
