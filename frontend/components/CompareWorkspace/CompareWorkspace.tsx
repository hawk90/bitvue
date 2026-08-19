/**
 * Compare Workspace - Side-by-side A/B stream comparison
 *
 * Per parity and COMPARE_ALIGNMENT_POLICY.md:
 * - Side-by-side player view with Stream A and B
 * - Sync controls (Off/Playhead/Full)
 * - Manual offset UI for alignment adjustment
 * - Resolution mismatch detection
 *
 * Wired to the real backend (docs/DEVELOPMENT_PHASES.md Phase 7.5) as of this pass -- previously
 * this took the FULL raw `CompareWorkspace` engine struct (including `alignment.frame_pairs`, a
 * bulk per-frame array) as a prop and did its own client-side scan through it for
 * `getAlignedFrame`, duplicating what the real `get_aligned_frame` sidecar command already does
 * server-side. Now pulls the lean `CompareWorkspaceSummary` from `useCompare()`'s context
 * directly (same as `setSyncMode`/`setManualOffset` already did) instead of requiring it as a
 * prop, and calls the context's async `getAlignedFrame` instead of a local reimplementation.
 * Also fixes three named-vs-default import bugs (`CompareControls`/`StreamPlayer`/`DiffOverlay`
 * are all `export default`, but were imported as `{ X }`) that went unnoticed because this whole
 * directory was excluded from `tsc` while unmounted (`frontend/tsconfig.json`'s `exclude` -- now
 * removed as part of mounting this for real).
 */

import { useState, useCallback, useEffect, memo } from "react";
import {
  SyncMode,
  type FrameInfo,
  type AlignmentQuality,
} from "../../types/video";
import CompareControls from "./CompareControls";
import StreamPlayer from "./StreamPlayer";
import DiffOverlay from "./DiffOverlay";
import { useCompare } from "../../contexts/CompareContext";
import type { DiffMode } from "../../services/electronBridgeService";
import "./CompareWorkspace.css";

interface CompareWorkspaceProps {
  framesA: FrameInfo[];
  framesB: FrameInfo[];
  currentFrameA: number;
  currentFrameB: number;
  onFrameChangeA: (index: number) => void;
  onFrameChangeB: (index: number) => void;
}

function CompareWorkspace({
  framesA,
  framesB,
  currentFrameA,
  currentFrameB,
  onFrameChangeA,
  onFrameChangeB,
}: CompareWorkspaceProps) {
  const {
    workspace,
    setSyncMode,
    setManualOffset,
    getAlignedFrame,
    findFirstDiffFrame,
    isScanningDiff,
  } = useCompare();

  const [showDiff, setShowDiff] = useState(workspace?.diff_enabled ?? false);
  const [diffMode, setDiffMode] = useState<DiffMode>("abs");
  const [alignedB, setAlignedB] = useState<{
    bIdx: number | null;
    quality: AlignmentQuality | null;
  }>({ bIdx: null, quality: null });
  const [diffScanStatus, setDiffScanStatus] = useState<string | null>(null);

  // Real aligned-B lookup for the current stream A frame (server-side, PTS-based) -- replaces
  // the old synchronous `workspace.alignment.frame_pairs.find(...)` scan.
  useEffect(() => {
    let cancelled = false;
    getAlignedFrame(currentFrameA).then((result) => {
      if (!cancelled) setAlignedB(result);
    });
    return () => {
      cancelled = true;
    };
  }, [currentFrameA, getAlignedFrame]);

  const handleSyncModeChange = useCallback(
    (mode: SyncMode) => {
      setSyncMode(mode).catch(() => {});
    },
    [setSyncMode],
  );

  const handleOffsetChange = useCallback(
    (delta: number) => {
      if (!workspace) return;
      setManualOffset(workspace.manual_offset + delta).catch(() => {});
    },
    [setManualOffset, workspace],
  );

  const handleFrameChangeA = useCallback(
    (index: number) => {
      onFrameChangeA(index);

      // Sync to B if enabled
      if (workspace && workspace.sync_mode !== SyncMode.Off) {
        getAlignedFrame(index)
          .then(({ bIdx }) => {
            if (bIdx !== null) onFrameChangeB(bIdx);
          })
          .catch(() => {});
      }
    },
    [onFrameChangeA, onFrameChangeB, workspace, getAlignedFrame],
  );

  // PARITY_CHECKLIST.md CMP-04 -- scans, then jumps both players to the first real difference
  // (same sync-aware jump handleFrameChangeA already does, so this respects the current sync
  // mode instead of moving A without B).
  const handleFindFirstDiff = useCallback(() => {
    setDiffScanStatus(null);
    findFirstDiffFrame().then(({ frameIndex, totalChecked }) => {
      if (frameIndex === null) {
        setDiffScanStatus(
          `No differences found (${totalChecked} frames checked)`,
        );
        return;
      }
      setDiffScanStatus(null);
      handleFrameChangeA(frameIndex);
    });
  }, [findFirstDiffFrame, handleFrameChangeA]);

  if (!workspace) {
    return null;
  }

  const currentFrameAData = framesA[currentFrameA] || null;
  const currentFrameBData = framesB[currentFrameB] || null;

  return (
    <div className="compare-workspace">
      {/* Header with sync controls */}
      <div className="compare-header">
        <div className="compare-title">
          <h2>A/B Compare</h2>
          <span className="compare-subtitle">
            {workspace.alignment.method} • {workspace.alignment.confidence}{" "}
            confidence
          </span>
        </div>

        <CompareControls
          syncMode={workspace.sync_mode}
          manualOffset={workspace.manual_offset}
          onSyncModeChange={handleSyncModeChange}
          onOffsetChange={handleOffsetChange}
          alignmentInfo={{
            method: workspace.alignment.method,
            confidence: workspace.alignment.confidence,
            gapPercentage: workspace.alignment.gap_percentage,
          }}
        />

        <div className="compare-actions">
          {workspace.diff_enabled && (
            <label className="diff-toggle">
              <input
                type="checkbox"
                checked={showDiff}
                onChange={(e) => setShowDiff(e.target.checked)}
              />
              Show Diff
            </label>
          )}
          {showDiff && (
            <select
              value={diffMode}
              onChange={(e) => setDiffMode(e.target.value as DiffMode)}
              className="diff-mode-select"
            >
              {/* VQ-Analyzer-parity naming (PARITY_CHECKLIST.md CMP-03) -- both real, implemented
                  DiffMode variants; no psnr/ssim/metric options (unimplemented in the engine,
                  would be fake controls -- see this file's own doc). */}
              <option value="signed">Subtraction</option>
              <option value="abs">Temperature</option>
            </select>
          )}
          {workspace.diff_enabled && (
            <button
              type="button"
              className="find-first-diff-button"
              onClick={handleFindFirstDiff}
              disabled={isScanningDiff}
              title="Scan stream A for the first frame that genuinely differs from its aligned stream B frame"
            >
              {isScanningDiff ? "Scanning…" : "Find First Diff"}
            </button>
          )}
          {diffScanStatus && (
            <span className="find-first-diff-status">{diffScanStatus}</span>
          )}
        </div>
      </div>

      {/* Resolution warning if incompatible */}
      {!workspace.resolution_info.is_compatible && (
        <div className="compare-warning">
          <span className="warning-icon">⚠</span>
          {workspace.disable_reason}
        </div>
      )}

      {/* Side-by-side players */}
      <div className="compare-content">
        <div className="compare-stream">
          <div className="stream-header stream-a">
            <h3>Stream A</h3>
            <span className="stream-info">
              {workspace.resolution_info.stream_a[0]}x
              {workspace.resolution_info.stream_a[1]} • {framesA.length} frames
            </span>
          </div>
          <StreamPlayer
            frames={framesA}
            currentFrame={currentFrameA}
            onFrameChange={handleFrameChangeA}
            streamLabel="A"
          />
        </div>

        <div className="compare-divider" />

        <div className="compare-stream">
          <div className="stream-header stream-b">
            <h3>Stream B</h3>
            <span className="stream-info">
              {workspace.resolution_info.stream_b[0]}x
              {workspace.resolution_info.stream_b[1]} • {framesB.length} frames
            </span>
          </div>
          <StreamPlayer
            frames={framesB}
            currentFrame={currentFrameB}
            onFrameChange={onFrameChangeB}
            streamLabel="B"
            alignedFrame={alignedB.bIdx}
            alignmentQuality={alignedB.quality ?? undefined}
          />
        </div>

        {/* Diff overlay */}
        {showDiff &&
          workspace.diff_enabled &&
          currentFrameAData &&
          currentFrameBData && (
            <DiffOverlay
              frameA={currentFrameAData}
              frameB={currentFrameBData}
              mode={diffMode}
            />
          )}
      </div>

      {/* Alignment info footer */}
      <div className="compare-footer">
        <div className="alignment-summary">
          <span>Gap: {workspace.alignment.gap_count} frames</span>
          <span>
            Offset: {workspace.manual_offset > 0 ? "+" : ""}
            {workspace.manual_offset}
          </span>
          <span
            className={`quality-indicator quality-${workspace.alignment.confidence.toLowerCase()}`}
          >
            {workspace.alignment.confidence}
          </span>
        </div>
      </div>
    </div>
  );
}

export default memo(CompareWorkspace);
