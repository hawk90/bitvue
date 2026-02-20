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
      <button onClick={onZoomOut} title="Zoom Out (-)">
        <span className="codicon codicon-zoom-out"></span>
      </button>
      <span className="yuv-zoom-label">{Math.round(zoom * 100)}%</span>
      <button onClick={onZoomIn} title="Zoom In (+)">
        <span className="codicon codicon-zoom-in"></span>
      </button>
      <button onClick={onResetZoom} title="Reset Zoom (Ctrl+0)">
        <span className="codicon codicon-screen-normal"></span>
      </button>
    </div>
  );
});
