/**
 * Zoom Controls Component
 *
 * Zoom in/out buttons and zoom level display
 * Includes reset zoom button
 */

import { memo } from "react";

interface ZoomControlsProps {
  zoom: number;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onResetZoom: () => void;
}

export const ZoomControls = memo(function ZoomControls({
  zoom,
  onZoomIn,
  onZoomOut,
  onResetZoom,
}: ZoomControlsProps) {
  return (
    <div className="yuv-toolbar-group">
      <button onClick={onZoomOut} title="Zoom Out (-)" aria-label="Zoom out">
        <span className="codicon codicon-zoom-out"></span>
      </button>
      <span
        className="yuv-zoom-label"
        title={`Zoom: ${Math.round(zoom * 100)}%`}
      >
        {Math.round(zoom * 100)}%
      </span>
      <button onClick={onZoomIn} title="Zoom In (+)" aria-label="Zoom in">
        <span className="codicon codicon-zoom-in"></span>
      </button>
      <button
        onClick={onResetZoom}
        title="Reset Zoom (Ctrl+0)"
        aria-label="Reset zoom to 100%"
      >
        <span className="codicon codicon-screen-normal"></span>
      </button>
    </div>
  );
});
