/**
 * Status Bar Component
 *
 * Displays status information at the bottom of the application
 */

import { memo } from "react";
import "./StatusBar.css";

interface StatusBarProps {
  /** Current file path or application name */
  fileInfo: { path?: string } | null;
  /** Number of frames loaded */
  frameCount: number;
  /** Callback to show keyboard shortcuts dialog */
  onShowShortcuts?: () => void;
}

export const StatusBar = memo(function StatusBar({
  fileInfo,
  frameCount,
  onShowShortcuts,
}: StatusBarProps) {
  return (
    <div className="status-bar" data-testid="status-bar">
      <div className="status-bar-left"></div>
      <div className="status-bar-center">
        {fileInfo?.path || "Bitvue - Video Bitstream Analyzer"}
      </div>
      <div className="status-bar-right">
        {frameCount > 0 ? `${frameCount} frames` : "Ready"}
        {onShowShortcuts && (
          <button
            className="status-shortcuts-btn"
            onClick={onShowShortcuts}
            title="Keyboard Shortcuts (Ctrl+?)"
          >
            <span className="codicon codicon-keyboard"></span>
          </button>
        )}
      </div>
    </div>
  );
});

export default StatusBar;
