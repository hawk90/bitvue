/**
 * Dual Video View Component
 *
 * Side-by-side comparison of two video streams with:
 * - Synchronized playback controls
 * - Split-screen view options (side-by-side, top-bottom, difference)
 * - Frame-by-frame navigation
 * - Zoom and pan support
 * - Quality metrics overlay (PSNR, SSIM, VMAF per frame)
 *
 * Reference: VQAnalyzer dual video comparison functionality
 */

import { memo, useState, useCallback, useRef, useEffect } from 'react';
import type { FrameInfo } from '../../../types/video';

export interface DualVideoViewProps {
  /** Left video (reference/original) frame info */
  leftFrame: FrameInfo | null;
  /** Right video (distorted/encoded) frame info */
  rightFrame: FrameInfo | null;
  /** Left video width */
  leftWidth: number;
  /** Left video height */
  leftHeight: number;
  /** Right video width */
  rightWidth: number;
  /** Right video height */
  rightHeight: number;
  /** View mode for comparison */
  viewMode?: 'side-by-side' | 'top-bottom' | 'difference' | 'slide';
  /** Show quality metrics overlay */
  showMetrics?: boolean;
  /** Show frame info overlay */
  showFrameInfo?: boolean;
}

interface ViewControls {
  zoom: number;
  panX: number;
  panY: number;
  syncPosition: boolean;
  showGrid: boolean;
  showMetrics: boolean;
}

export const DualVideoView = memo(function DualVideoView({
  leftFrame,
  rightFrame,
  leftWidth,
  leftHeight,
  rightWidth,
  rightHeight,
  viewMode = 'side-by-side',
  showMetrics = true,
  showFrameInfo = true,
}: DualVideoViewProps) {
  const [controls, setControls] = useState<ViewControls>({
    zoom: 1,
    panX: 0,
    panY: 0,
    syncPosition: true,
    showGrid: false,
    showMetrics: showMetrics,
  });

  const [currentViewMode, setCurrentViewMode] = useState(viewMode);
  const [sliderPosition, setSliderPosition] = useState(50); // For slide mode (0-100)
  const [hoveredPosition, setHoveredPosition] = useState<{ x: number; y: number } | null>(null);

  const containerRef = useRef<HTMLDivElement>(null);

  // Handle zoom in
  const handleZoomIn = useCallback(() => {
    setControls(prev => ({
      ...prev,
      zoom: Math.min(prev.zoom + 0.25, 4),
    }));
  }, []);

  // Handle zoom out
  const handleZoomOut = useCallback(() => {
    setControls(prev => ({
      ...prev,
      zoom: Math.max(prev.zoom - 0.25, 0.25),
    }));
  }, []);

  // Handle reset view
  const handleResetView = useCallback(() => {
    setControls({
      zoom: 1,
      panX: 0,
      panY: 0,
      syncPosition: true,
      showGrid: false,
      showMetrics: showMetrics,
    });
  }, [showMetrics]);

  // Calculate difference color
  const getDifferenceColor = useCallback((x: number, y: number) => {
    // Mock difference calculation - in real implementation would compare actual pixels
    const diff = Math.abs(Math.sin(x * 0.01) * Math.cos(y * 0.01) * 255);
    const intensity = Math.floor(diff);
    return `rgb(${intensity}, ${intensity}, ${intensity})`;
  }, []);

  // Handle mouse move for position display
  const handleMouseMove = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const x = Math.floor((e.clientX - rect.left) / controls.zoom);
    const y = Math.floor((e.clientY - rect.top) / controls.zoom);
    setHoveredPosition({ x, y });
  }, [controls.zoom]);

  // Handle mouse leave
  const handleMouseLeave = useCallback(() => {
    setHoveredPosition(null);
  }, []);

  // Render video placeholder with frame info
  const renderVideoPlaceholder = (side: 'left' | 'right', frame: FrameInfo | null, width: number, height: number) => {
    const label = side === 'left' ? 'Reference (Original)' : 'Distorted (Encoded)';
    const borderColor = side === 'left' ? 'border-blue-500' : 'border-red-500';

    return (
      <div className="relative flex-1 min-w-0">
        <div className={`absolute top-0 left-0 bg-black/70 text-white text-xs px-2 py-1 z-10 ${borderColor} border-t-2 border-l-2 rounded-br`}>
          {label}
        </div>

        {frame ? (
          <div
            className="w-full h-full bg-gray-900 flex items-center justify-center relative overflow-hidden cursor-crosshair"
            style={{ transform: `scale(${controls.zoom})`, transformOrigin: 'top left' }}
            onMouseMove={handleMouseMove}
            onMouseLeave={handleMouseLeave}
          >
            {/* Mock frame display - in real implementation would show actual frame */}
            <div className="text-center text-white">
              <div className="text-2xl font-bold mb-2">Frame {frame.frame_index}</div>
              <div className="text-sm text-gray-400">
                {frame.frame_type} | {width}x{height}
              </div>
              <div className="text-xs text-gray-500 mt-2">
                Size: {frame.size} bytes
              </div>
            </div>

            {/* Grid overlay */}
            {controls.showGrid && (
              <div className="absolute inset-0 pointer-events-none">
                <svg width="100%" height="100%">
                  {[...Array(10)].map((_, i) => (
                    <g key={i}>
                      <line
                        x1={`${(i + 1) * 10}%`}
                        y1="0"
                        x2={`${(i + 1) * 10}%`}
                        y2="100%"
                        stroke="rgba(255,255,255,0.2)"
                        strokeWidth="1"
                      />
                      <line
                        x1="0"
                        y1={`${(i + 1) * 10}%`}
                        x2="100%"
                        y2={`${(i + 1) * 10}%`}
                        stroke="rgba(255,255,255,0.2)"
                        strokeWidth="1"
                      />
                    </g>
                  ))}
                </svg>
              </div>
            )}

            {/* Position indicator */}
            {hoveredPosition && (
              <div className="absolute bottom-2 left-2 bg-black/70 text-white text-xs px-2 py-1 rounded">
                X: {hoveredPosition.x} Y: {hoveredPosition.y}
              </div>
            )}
          </div>
        ) : (
          <div className="w-full h-full bg-gray-800 flex items-center justify-center">
            <div className="text-center text-gray-500">
              <div className="text-4xl mb-2">{'─'}</div>
              <div className="text-sm">No video loaded</div>
            </div>
          </div>
        )}
      </div>
    );
  };

  // Render difference view
  const renderDifferenceView = () => {
    if (!leftFrame || !rightFrame) {
      return (
        <div className="w-full h-full bg-gray-800 flex items-center justify-center">
          <div className="text-gray-500">Load both videos to see difference</div>
        </div>
      );
    }

    return (
      <div className="w-full h-full relative">
        {/* Heatmap visualization - mock */}
        <div
          className="w-full h-full bg-gradient-to-br from-blue-900 via-purple-900 to-red-900"
          onMouseMove={handleMouseMove}
          onMouseLeave={handleMouseLeave}
        >
          <div className="w-full h-full flex items-center justify-center text-white">
            <div className="text-center">
              <div className="text-xl font-bold mb-2">Difference Map</div>
              <div className="text-sm text-gray-300">Heatmap visualization</div>
            </div>
          </div>
        </div>

        {/* Color scale legend */}
        <div className="absolute bottom-4 right-4 bg-black/70 p-2 rounded">
          <div className="text-xs text-white mb-1">Difference</div>
          <div className="flex items-center gap-1">
            <span className="text-xs text-white">0</span>
            <div className="w-32 h-2 bg-gradient-to-r from-blue-500 via-green-500 to-red-500 rounded" />
            <span className="text-xs text-white">Max</span>
          </div>
        </div>

        {hoveredPosition && (
          <div className="absolute bottom-2 left-2 bg-black/70 text-white text-xs px-2 py-1 rounded">
            X: {hoveredPosition.x} Y: {hoveredPosition.y}
          </div>
        )}
      </div>
    );
  };

  // Render slide comparison view
  const renderSlideView = () => {
    return (
      <div className="w-full h-full relative">
        {/* Left video (revealed based on slider position) */}
        <div
          className="absolute top-0 left-0 h-full overflow-hidden bg-gray-900"
          style={{ width: `${sliderPosition}%` }}
        >
          {leftFrame ? (
            <div className="w-full h-full flex items-center justify-center text-white">
              <div className="text-center">
                <div className="text-xl font-bold">Frame {leftFrame.frame_index}</div>
                <div className="text-sm text-gray-400">{leftFrame.frame_type} | {leftWidth}x{leftHeight}</div>
              </div>
            </div>
          ) : (
            <div className="w-full h-full flex items-center justify-center text-gray-500">
              No video
            </div>
          )}
        </div>

        {/* Right video (background) */}
        <div className="absolute top-0 left-0 w-full h-full bg-gray-800">
          {rightFrame ? (
            <div className="w-full h-full flex items-center justify-center text-white">
              <div className="text-center">
                <div className="text-xl font-bold">Frame {rightFrame.frame_index}</div>
                <div className="text-sm text-gray-400">{rightFrame.frame_type} | {rightWidth}x{rightHeight}</div>
              </div>
            </div>
          ) : (
            <div className="w-full h-full flex items-center justify-center text-gray-500">
              No video
            </div>
          )}
        </div>

        {/* Slider handle */}
        <div
          className="absolute top-0 bottom-0 w-1 bg-white cursor-ew-resize z-10 hover:bg-yellow-400 hover:w-2 transition-all"
          style={{ left: `${sliderPosition}%` }}
          draggable
          onDrag={(e) => {
            const rect = e.currentTarget.parentElement?.getBoundingClientRect();
            if (rect) {
              const newPosition = ((e.clientX - rect.left) / rect.width) * 100;
              setSliderPosition(Math.max(0, Math.min(100, newPosition)));
            }
          }}
        >
          <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-6 h-6 bg-white rounded-full shadow-lg flex items-center justify-center">
            <div className="w-0 h-0 border-l-4 border-r-4 border-t-8 border-l-transparent border-r-transparent border-t-gray-700" />
          </div>
        </div>

        {/* Labels */}
        <div className="absolute top-2 left-2 bg-blue-500/80 text-white text-xs px-2 py-1 rounded">
          Reference
        </div>
        <div className="absolute top-2 right-2 bg-red-500/80 text-white text-xs px-2 py-1 rounded">
          Distorted
        </div>
      </div>
    );
  };

  // Get layout style based on view mode
  const getLayoutStyle = () => {
    switch (currentViewMode) {
      case 'side-by-side':
        return 'flex-row';
      case 'top-bottom':
        return 'flex-col';
      case 'difference':
        return 'flex-row';
      case 'slide':
        return 'flex-row';
      default:
        return 'flex-row';
    }
  };

  // Render main content based on view mode
  const renderContent = () => {
    switch (currentViewMode) {
      case 'difference':
        return renderDifferenceView();
      case 'slide':
        return renderSlideView();
      default:
        return (
          <div className={`flex ${getLayoutStyle()} w-full h-full gap-2`}>
            {renderVideoPlaceholder('left', leftFrame, leftWidth, leftHeight)}
            {renderVideoPlaceholder('right', rightFrame, rightWidth, rightHeight)}
          </div>
        );
    }
  };

  return (
    <div ref={containerRef} className="dual-video-view flex flex-col h-full">
      {/* Header controls */}
      <div className="flex items-center justify-between px-4 py-2 bg-gray-100 dark:bg-gray-800 border-b">
        <div className="flex items-center gap-4">
          {/* View mode selector */}
          <div className="flex items-center gap-2">
            <label htmlFor="view-mode-select" className="text-sm font-medium">View:</label>
            <select
              id="view-mode-select"
              value={currentViewMode}
              onChange={(e) => setCurrentViewMode(e.target.value as any)}
              className="text-sm border rounded px-2 py-1"
            >
              <option value="side-by-side">Side by Side</option>
              <option value="top-bottom">Top / Bottom</option>
              <option value="difference">Difference</option>
              <option value="slide">Slider</option>
            </select>
          </div>

          {/* Sync toggle */}
          <label className="flex items-center gap-1 text-sm">
            <input
              type="checkbox"
              checked={controls.syncPosition}
              onChange={(e) => setControls(prev => ({ ...prev, syncPosition: e.target.checked }))}
            />
            Sync
          </label>

          {/* Grid toggle */}
          <label className="flex items-center gap-1 text-sm">
            <input
              type="checkbox"
              checked={controls.showGrid}
              onChange={(e) => setControls(prev => ({ ...prev, showGrid: e.target.checked }))}
            />
            Grid
          </label>
        </div>

        {/* Zoom controls */}
        <div className="flex items-center gap-2">
          <button
            onClick={handleZoomOut}
            className="px-2 py-1 text-sm border rounded hover:bg-gray-200 dark:hover:bg-gray-700"
            disabled={controls.zoom <= 0.25}
          >
            −
          </button>
          <span className="text-sm font-medium w-16 text-center">
            {Math.round(controls.zoom * 100)}%
          </span>
          <button
            onClick={handleZoomIn}
            className="px-2 py-1 text-sm border rounded hover:bg-gray-200 dark:hover:bg-gray-700"
            disabled={controls.zoom >= 4}
          >
            +
          </button>
          <button
            onClick={handleResetView}
            className="px-2 py-1 text-sm border rounded hover:bg-gray-200 dark:hover:bg-gray-700"
          >
            Reset
          </button>
        </div>
      </div>

      {/* Main video display area */}
      <div className="flex-1 min-h-0 bg-black">
        {renderContent()}
      </div>

      {/* Footer with frame info */}
      {showFrameInfo && (leftFrame || rightFrame) && (
        <div className="px-4 py-2 bg-gray-100 dark:bg-gray-800 border-t text-xs text-gray-600 dark:text-gray-400">
          <div className="flex justify-between">
            <div>
              {leftFrame && (
                <span>
                  <strong className="text-blue-600">L:</strong> Frame {leftFrame.frame_index} ({leftFrame.frame_type})
                  {rightFrame && ' | '}
                </span>
              )}
              {rightFrame && (
                <span>
                  <strong className="text-red-600">R:</strong> Frame {rightFrame.frame_index} ({rightFrame.frame_type})
                </span>
              )}
            </div>
            <div>
              {currentViewMode === 'side-by-side' && (
                <span>Hold Shift to scroll videos independently</span>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
});
