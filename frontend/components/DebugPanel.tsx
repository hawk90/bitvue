/**
 * Debug Panel Component
 *
 * Shows frame data state for debugging
 */

import { useSelection } from "../contexts/SelectionContext";
import type { FrameInfo } from "../types/video";
import { useCallback, memo } from "react";
import "./DebugPanel.css";

interface DebugPanelProps {
  frames: FrameInfo[];
  visible: boolean;
  onClose: () => void;
}

export const DebugPanel = memo(function DebugPanel({
  frames,
  visible,
  onClose,
}: DebugPanelProps) {
  const { selection } = useSelection();
  const currentFrameIndex = selection?.frame?.frameIndex ?? 0;

  const currentFrame = frames[currentFrameIndex];

  const handleFrameClick = useCallback((frameIndex: number) => {
    const frameEl = document.querySelector(
      `[data-frame-index="${frameIndex}"]`,
    );
    if (frameEl instanceof HTMLElement) {
      frameEl.click();
    }
  }, []);

  // Statistics
  const stats = {
    total: frames.length,
    keyFrames: frames.filter(
      (f) => f.frame_type === "KEY" || f.frame_type === "I",
    ).length,
    pFrames: frames.filter(
      (f) => f.frame_type === "P" || f.frame_type === "INTER",
    ).length,
    bFrames: frames.filter((f) => f.frame_type === "B").length,
    withRefs: frames.filter((f) => f.ref_frames && f.ref_frames.length > 0)
      .length,
  };

  if (!visible) return null;

  return (
    <>
      <button
        className="debug-toggle open"
        onClick={onClose}
        title="Close Debug Panel"
      >
        <span className="codicon codicon-chevron-right" />
      </button>
      <div className="debug-panel">
        <div className="debug-panel-header">
          <span className="debug-panel-title">Debug Panel</span>
          <button className="debug-panel-close" onClick={onClose}>
            <span className="codicon codicon-close" />
          </button>
        </div>
        <div className="debug-panel-content">
          {/* Statistics */}
          <div className="debug-section">
            <div className="debug-section-title">Statistics</div>
            <div className="debug-info-row">
              <span className="debug-info-label">Total Frames</span>
              <span className="debug-info-value number">{stats.total}</span>
            </div>
            <div className="debug-info-row">
              <span className="debug-info-label">Key Frames (I)</span>
              <span className="debug-info-value number">{stats.keyFrames}</span>
            </div>
            <div className="debug-info-row">
              <span className="debug-info-label">P Frames</span>
              <span className="debug-info-value number">{stats.pFrames}</span>
            </div>
            <div className="debug-info-row">
              <span className="debug-info-label">B Frames</span>
              <span className="debug-info-value number">{stats.bFrames}</span>
            </div>
            <div className="debug-info-row">
              <span className="debug-info-label">With References</span>
              <span className="debug-info-value number">{stats.withRefs}</span>
            </div>
          </div>

          {/* Current Frame */}
          {currentFrame && (
            <div className="debug-section">
              <div className="debug-section-title">
                Current Frame #{currentFrameIndex}
              </div>
              <div className="debug-info-row">
                <span className="debug-info-label">Frame Type</span>
                <span className="debug-info-value string">
                  {currentFrame.frame_type}
                </span>
              </div>
              <div className="debug-info-row">
                <span className="debug-info-label">Size</span>
                <span className="debug-info-value number">
                  {(currentFrame.size / 1024).toFixed(2)} KB
                </span>
              </div>
              <div className="debug-info-row">
                <span className="debug-info-label">Key Frame</span>
                <span className="debug-info-value boolean">
                  {String(currentFrame.key_frame ?? false)}
                </span>
              </div>
              <div className="debug-info-row">
                <span className="debug-info-label">Display Order</span>
                <span className="debug-info-value number">
                  {currentFrame.display_order ?? "null"}
                </span>
              </div>
              <div className="debug-info-row">
                <span className="debug-info-label">Coding Order</span>
                <span className="debug-info-value number">
                  {currentFrame.coding_order ?? "null"}
                </span>
              </div>
              <div className="debug-info-row">
                <span className="debug-info-label">POC</span>
                <span className="debug-info-value number">
                  {currentFrame.poc ?? "null"}
                </span>
              </div>
              <div className="debug-info-row">
                <span className="debug-info-label">Ref Frames</span>
                <span className="debug-info-value">
                  {currentFrame.ref_frames ? (
                    <span className="debug-info-value string">
                      [{currentFrame.ref_frames.join(", ")}]
                    </span>
                  ) : (
                    <span className="debug-info-value null">null</span>
                  )}
                </span>
              </div>
              <div className="debug-info-row">
                <span className="debug-info-label">Ref Slots</span>
                <span className="debug-info-value">
                  {currentFrame.ref_slots ? (
                    <span className="debug-info-value string">
                      [{currentFrame.ref_slots.join(", ")}]
                    </span>
                  ) : (
                    <span className="debug-info-value null">null</span>
                  )}
                </span>
              </div>
            </div>
          )}

          {/* All Frames List */}
          <div className="debug-section">
            <div className="debug-section-title">All Frames (First 50)</div>
            <div className="debug-frame-list">
              {frames.slice(0, 50).map((frame) => (
                <div
                  key={frame.frame_index}
                  className={`debug-frame-item ${frame.frame_index === currentFrameIndex ? "selected" : ""}`}
                  onClick={() => handleFrameClick(frame.frame_index)}
                >
                  <div className="debug-frame-header">
                    <span className="debug-frame-index">
                      #{frame.frame_index}
                    </span>
                    <span className={`debug-frame-type ${frame.frame_type}`}>
                      {frame.frame_type}
                    </span>
                  </div>
                  {frame.ref_frames && frame.ref_frames.length > 0 && (
                    <div className="debug-frame-details">
                      <span className="debug-frame-detail-label">Refs:</span>
                      <span className="debug-frame-detail-value">
                        {frame.ref_frames.join(", ")}
                      </span>
                    </div>
                  )}
                  <div className="debug-frame-details">
                    <span className="debug-frame-detail-label">Size:</span>
                    <span className="debug-frame-detail-value">
                      {(frame.size / 1024).toFixed(1)}K
                    </span>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </>
  );
});

export const DebugPanelToggle = memo(function DebugPanelToggle({
  isOpen,
  onToggle,
}: {
  isOpen: boolean;
  onToggle: () => void;
}) {
  return (
    <button
      className={`debug-toggle ${isOpen ? "open" : ""}`}
      onClick={onToggle}
      title={isOpen ? "Close Debug Panel" : "Open Debug Panel"}
    >
      <span
        className={`codicon codicon-chevron-${isOpen ? "right" : "left"}`}
      />
    </button>
  );
});
