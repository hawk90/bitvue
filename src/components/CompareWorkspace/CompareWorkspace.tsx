/**
 * Compare Workspace - Side-by-side A/B stream comparison
 *
 * Per VQAnalyzer parity and COMPARE_ALIGNMENT_POLICY.md:
 * - Side-by-side player view with Stream A and B
 * - Sync controls (Off/Playhead/Full)
 * - Manual offset UI for alignment adjustment
 * - Resolution mismatch detection
 */

import { useState, useCallback, memo } from 'react';
import {
  CompareWorkspace as CompareWorkspaceType,
  SyncMode,
  AlignmentQuality,
  type FrameInfo,
} from '../../types/video';
import { CompareControls } from './CompareControls';
import { StreamPlayer } from './StreamPlayer';
import { DiffOverlay } from './DiffOverlay';
import './CompareWorkspace.css';

interface CompareWorkspaceProps {
  workspace: CompareWorkspaceType;
  framesA: FrameInfo[];
  framesB: FrameInfo[];
  currentFrameA: number;
  currentFrameB: number;
  onFrameChangeA: (index: number) => void;
  onFrameChangeB: (index: number) => void;
}

function CompareWorkspace({
  workspace,
  framesA,
  framesB,
  currentFrameA,
  currentFrameB,
  onFrameChangeA,
  onFrameChangeB,
}: CompareWorkspaceProps) {
  const [showDiff, setShowDiff] = useState(workspace.diff_enabled);
  const [diffMode, setDiffMode] = useState<'difference' | 'psnr' | 'ssim'>('difference');

  // Get aligned frame for stream A
  const getAlignedFrame = useCallback((aIdx: number): { bIdx: number | null; quality: AlignmentQuality } => {
    // Apply manual offset
    const adjustedAIdx = Math.max(0, aIdx + workspace.manual_offset);

    // Find alignment pair
    const pair = workspace.alignment.frame_pairs.find(
      p => p.stream_a_idx === adjustedAIdx
    );

    if (pair && pair.stream_b_idx !== null) {
      const quality = pair.has_gap
        ? AlignmentQuality.Gap
        : pair.pts_delta === 0
          ? AlignmentQuality.Exact
          : AlignmentQuality.Nearest;
      return { bIdx: pair.stream_b_idx, quality };
    }

    return { bIdx: null, quality: AlignmentQuality.Gap };
  }, [workspace.alignment.frame_pairs, workspace.manual_offset]);

  // Handle sync mode change
  const handleSyncModeChange = useCallback((mode: SyncMode) => {
    // This would be handled by parent component
    console.log('Sync mode changed:', mode);
  }, []);

  // Handle manual offset change
  const handleOffsetChange = useCallback((delta: number) => {
    // This would be handled by parent component
    console.log('Offset changed:', delta);
  }, []);

  // Handle frame change with sync
  const handleFrameChangeA = useCallback((index: number) => {
    onFrameChangeA(index);

    // Sync to B if enabled
    if (workspace.sync_mode !== SyncMode.Off) {
      const { bIdx } = getAlignedFrame(index);
      if (bIdx !== null) {
        onFrameChangeB(bIdx);
      }
    }
  }, [onFrameChangeA, onFrameChangeB, workspace.sync_mode, getAlignedFrame]);

  const currentFrameAData = framesA[currentFrameA] || null;
  const currentFrameBData = framesB[currentFrameB] || null;

  return (
    <div className="compare-workspace">
      {/* Header with sync controls */}
      <div className="compare-header">
        <div className="compare-title">
          <h2>A/B Compare</h2>
          <span className="compare-subtitle">
            {workspace.alignment?.method ?? 'Unknown'} • {workspace.alignment?.confidence ?? 0} confidence
          </span>
        </div>

        <CompareControls
          syncMode={workspace.sync_mode}
          manualOffset={workspace.manual_offset}
          onSyncModeChange={handleSyncModeChange}
          onOffsetChange={handleOffsetChange}
          alignmentInfo={{
            method: workspace.alignment?.method ?? 'Unknown',
            confidence: workspace.alignment?.confidence ?? 0,
            gapPercentage: workspace.alignment?.frame_pairs
              ? (workspace.alignment.gap_count / Math.max(workspace.alignment.frame_pairs.length, 1)) * 100
              : 0,
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
              onChange={(e) => setDiffMode(e.target.value as any)}
              className="diff-mode-select"
            >
              <option value="difference">Difference</option>
              <option value="psnr">PSNR</option>
              <option value="ssim">SSIM</option>
            </select>
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
              {workspace.resolution_info.stream_a[0]}x{workspace.resolution_info.stream_a[1]} • {framesA.length} frames
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
              {workspace.resolution_info.stream_b[0]}x{workspace.resolution_info.stream_b[1]} • {framesB.length} frames
            </span>
          </div>
          <StreamPlayer
            frames={framesB}
            currentFrame={currentFrameB}
            onFrameChange={onFrameChangeB}
            streamLabel="B"
            alignedFrame={getAlignedFrame(currentFrameA).bIdx}
            alignmentQuality={getAlignedFrame(currentFrameA).quality}
          />
        </div>

        {/* Diff overlay */}
        {showDiff && workspace.diff_enabled && currentFrameAData && currentFrameBData && (
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
          <span>Offset: {workspace.manual_offset > 0 ? '+' : ''}{workspace.manual_offset}</span>
          <span className={`quality-indicator quality-${workspace.alignment.confidence.toLowerCase()}`}>
            {workspace.alignment.confidence}
          </span>
        </div>
      </div>
    </div>
  );
}

export default memo(CompareWorkspace);
