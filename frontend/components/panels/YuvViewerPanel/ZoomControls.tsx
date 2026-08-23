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
  /** INT-01: Fit-to-window preset (key "1") -- optional so callers that haven't wired it (if
   *  any exist) don't need a stub. */
  onZoomToFit?: () => void;
}

export const ZoomControls = memo(function ZoomControls({
  zoom,
  onZoomIn,
  onZoomOut,
  onResetZoom,
  onZoomToFit,
}: ZoomControlsProps) {
  return (
    <div className="yuv-toolbar-group">
      {onZoomToFit && (
        <button
          onClick={onZoomToFit}
          title="Fit to Window (1)"
          aria-label="Fit to window"
        >
          <span className="codicon codicon-screen-full"></span>
        </button>
      )}
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
