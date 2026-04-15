/**
 * YUV Diff Panel
 *
 * Control panel for the YUVDiff (debug YUV) comparison mode.
 * Mirrors VQ Analyzer's "Debug YUV" / "Load Reference YUV" workflow:
 *
 *   [Decoded] [Reference] [Diff] [Amplified ×N]
 *   PSNR Y: 42.3 dB  U: 43.1 dB  V: 42.8 dB  Avg: 42.5 dB
 *   SSIM Y: 0.994     Max-Diff Y: 3
 *   [Find First Diff]  Offset: [±n]
 */

import { memo, useCallback } from "react";
import { useYuvDiff } from "../../contexts/YuvDiffContext";
import type { YuvDiffDisplayMode } from "../../contexts/YuvDiffContext";

interface YuvDiffPanelProps {
  /** Currently displayed frame index — used to fetch per-frame metrics. */
  currentFrameIndex: number;
  /** Called when the user clicks "Find First Diff" and a mismatch is found. */
  onJumpToFrame?: (frameIndex: number) => void;
}

const DISPLAY_MODES: { mode: YuvDiffDisplayMode; label: string }[] = [
  { mode: "decoded", label: "Decoded" },
  { mode: "reference", label: "Reference" },
  { mode: "diff", label: "Diff" },
  { mode: "amplified", label: "Amplified" },
];

const AMPLIFY_FACTORS = [2, 4, 8, 16];

function fmt(v: number | undefined, dp = 2): string {
  if (v === undefined || v === null) return "—";
  if (v >= 99.99) return "∞";
  return v.toFixed(dp);
}

export const YuvDiffPanel = memo(function YuvDiffPanel({
  currentFrameIndex,
  onJumpToFrame,
}: YuvDiffPanelProps) {
  const {
    isLoaded,
    path,
    frameCount,
    displayMode,
    amplifyFactor,
    pictureOffset,
    metrics,
    loading,
    error,
    setDisplayMode,
    setAmplifyFactor,
    setPictureOffset,
    fetchMetrics,
    findFirstDiff,
    unloadFile,
  } = useYuvDiff();

  const handleRefreshMetrics = useCallback(() => {
    void fetchMetrics(currentFrameIndex);
  }, [fetchMetrics, currentFrameIndex]);

  const handleFindFirstDiff = useCallback(async () => {
    const idx = await findFirstDiff();
    if (idx !== null && onJumpToFrame) {
      onJumpToFrame(idx);
    }
  }, [findFirstDiff, onJumpToFrame]);

  const handleOffsetChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const v = parseInt(e.target.value, 10);
      if (!isNaN(v)) setPictureOffset(v);
    },
    [setPictureOffset],
  );

  if (!isLoaded) {
    return (
      <div className="yuv-diff-panel yuv-diff-panel--empty">
        <span className="yuv-diff-hint">
          No debug YUV loaded. Use <strong>Debug → Load Reference YUV…</strong>
        </span>
      </div>
    );
  }

  const filename = path ? path.split(/[\\/]/).pop() : "unknown";

  return (
    <div className="yuv-diff-panel">
      {/* File info row */}
      <div className="yuv-diff-file-row">
        <span className="yuv-diff-file-name" title={path ?? ""}>
          {filename}
        </span>
        <span className="yuv-diff-frame-count">{frameCount} frames</span>
        <button
          className="yuv-diff-unload-btn"
          title="Unload reference YUV"
          onClick={unloadFile}
        >
          ✕
        </button>
      </div>

      {/* Display mode toggle */}
      <div className="yuv-diff-mode-row" role="group" aria-label="Display mode">
        {DISPLAY_MODES.map(({ mode, label }) => (
          <button
            key={mode}
            className={`yuv-diff-mode-btn${displayMode === mode ? " yuv-diff-mode-btn--active" : ""}`}
            onClick={() => setDisplayMode(mode)}
            aria-pressed={displayMode === mode}
          >
            {label}
          </button>
        ))}
      </div>

      {/* Amplification factor (only when amplified mode is active) */}
      {displayMode === "amplified" && (
        <div className="yuv-diff-amplify-row">
          <span className="yuv-diff-label">Amplify:</span>
          {AMPLIFY_FACTORS.map((f) => (
            <button
              key={f}
              className={`yuv-diff-amp-btn${amplifyFactor === f ? " yuv-diff-amp-btn--active" : ""}`}
              onClick={() => setAmplifyFactor(f)}
              aria-pressed={amplifyFactor === f}
            >
              ×{f}
            </button>
          ))}
        </div>
      )}

      {/* Metrics row */}
      <div className="yuv-diff-metrics-row">
        <button
          className="yuv-diff-metrics-btn"
          onClick={handleRefreshMetrics}
          disabled={loading}
          title="Compute PSNR/SSIM for current frame"
        >
          ⟳ Metrics
        </button>
        {metrics && metrics.frame_index === currentFrameIndex ? (
          <div className="yuv-diff-metrics">
            <span title="PSNR luma">
              PSNR-Y: <strong>{fmt(metrics.psnr_y)} dB</strong>
            </span>
            <span title="Average PSNR (6:1:1 weighted)">
              Avg: <strong>{fmt(metrics.psnr_avg)} dB</strong>
            </span>
            <span title="Simplified SSIM (luma)">
              SSIM-Y: <strong>{fmt(metrics.ssim_y, 4)}</strong>
            </span>
            <span title="Max per-pixel luma difference">
              MaxΔY: <strong>{metrics.max_diff_y}</strong>
            </span>
            {metrics.has_mismatch && (
              <span className="yuv-diff-mismatch-badge">MISMATCH</span>
            )}
          </div>
        ) : (
          <span className="yuv-diff-metrics-placeholder">
            — press ⟳ to compute —
          </span>
        )}
      </div>

      {/* Controls row: Find-first-diff + picture offset */}
      <div className="yuv-diff-controls-row">
        <button
          className="yuv-diff-find-btn"
          onClick={() => void handleFindFirstDiff()}
          disabled={loading}
          title="Scan all frames and jump to the first mismatch"
        >
          {loading ? "Scanning…" : "Find First Diff"}
        </button>

        <label className="yuv-diff-offset-label">
          Offset:
          <input
            type="number"
            className="yuv-diff-offset-input"
            value={pictureOffset}
            onChange={handleOffsetChange}
            min={-9999}
            max={9999}
            title="Frame index offset between decoded and reference"
          />
        </label>
      </div>

      {/* Error display */}
      {error && <div className="yuv-diff-error">{error}</div>}
    </div>
  );
});

export default YuvDiffPanel;
