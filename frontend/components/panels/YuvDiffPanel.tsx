/**
 * YUV Diff Panel
 *
 * Control panel for the YUVDiff (debug YUV) comparison mode.
 * Mirrors VQ Analyzer's "Debug YUV" / "Load Reference YUV" workflow:
 *
 *   [Decoded] [Reference] [Diff] [Amplified ×N]
 *   PSNR Y: 42.3 dB  U: 43.1 dB  V: 42.8 dB  Avg: 42.5 dB
 *   SSIM Y: 0.994     Max-Diff Y: 3
 *   [Find First Diff]  Offset: [±n]  [Crop…]  [⟳ Auto]
 */

import { memo, useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useYuvDiff } from "../../contexts/YuvDiffContext";
import type { YuvDiffDisplayMode } from "../../contexts/YuvDiffContext";
import { CropDialog } from "./CropDialog";
import type { CropValues } from "./CropDialog";
import "./YuvDiffPanel.css";

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

/** Amplify slider: ×1 to ×64, non-linear (powers of 2 in the low end). */
const AMPLIFY_MIN = 1;
const AMPLIFY_MAX = 64;

function fmt(v: number | undefined, dp = 2): string {
  if (v === undefined || v === null) return "—";
  if (v >= 99.99) return "∞";
  return v.toFixed(dp);
}

/** Auto-reload interval in seconds. */
const AUTO_RELOAD_INTERVAL_MS = 3000;

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

  const [showCropDialog, setShowCropDialog] = useState(false);
  const [crop, setCrop] = useState<CropValues>({
    left: 0,
    right: 0,
    top: 0,
    bottom: 0,
  });
  const [autoReload, setAutoReload] = useState(false);
  const autoReloadRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // ── Auto-reload ─────────────────────────────────────────────────────────────
  useEffect(() => {
    if (autoReload && isLoaded) {
      autoReloadRef.current = setInterval(() => {
        void fetchMetrics(currentFrameIndex);
      }, AUTO_RELOAD_INTERVAL_MS);
    } else {
      if (autoReloadRef.current) {
        clearInterval(autoReloadRef.current);
        autoReloadRef.current = null;
      }
    }
    return () => {
      if (autoReloadRef.current) {
        clearInterval(autoReloadRef.current);
        autoReloadRef.current = null;
      }
    };
  }, [autoReload, isLoaded, currentFrameIndex, fetchMetrics]);

  // ── Handlers ────────────────────────────────────────────────────────────────
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

  const handleAmplifyChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      setAmplifyFactor(parseInt(e.target.value, 10));
    },
    [setAmplifyFactor],
  );

  const handleCropApply = useCallback((newCrop: CropValues) => {
    setCrop(newCrop);
    setShowCropDialog(false);
    invoke("set_debug_yuv_crop", { crop: newCrop }).catch((e: unknown) =>
      console.warn("set_debug_yuv_crop:", e),
    );
  }, []);

  if (!isLoaded) {
    return (
      <div className="yuv-diff-panel yuv-diff-panel--empty">
        <span className="yuv-diff-hint">
          No debug YUV loaded. Use <strong>Debug → Open debug YUV…</strong>
        </span>
      </div>
    );
  }

  const filename = path ? path.split(/[\\/]/).pop() : "unknown";
  const hasCrop =
    crop.left > 0 || crop.right > 0 || crop.top > 0 || crop.bottom > 0;

  return (
    <div className="yuv-diff-panel">
      {/* ── File info row ────────────────────────────────────────────────── */}
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

      {/* ── Display mode toggle ──────────────────────────────────────────── */}
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

      {/* ── Amplification slider ─────────────────────────────────────────── */}
      {displayMode === "amplified" && (
        <div className="yuv-diff-amplify-row">
          <span className="yuv-diff-label">Amplify:</span>
          <input
            type="range"
            className="yuv-diff-amplify-slider"
            min={AMPLIFY_MIN}
            max={AMPLIFY_MAX}
            step={1}
            value={amplifyFactor}
            onChange={handleAmplifyChange}
            aria-label="Amplification factor"
          />
          <span className="yuv-diff-amplify-value">×{amplifyFactor}</span>
        </div>
      )}

      {/* ── Metrics row ──────────────────────────────────────────────────── */}
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

      {/* ── Controls row ─────────────────────────────────────────────────── */}
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

        <button
          className="yuv-diff-crop-btn"
          onClick={() => setShowCropDialog(true)}
          title={
            hasCrop
              ? `Crop: L${crop.left} R${crop.right} T${crop.top} B${crop.bottom}`
              : "Set crop values"
          }
          style={hasCrop ? { color: "#ffa", borderColor: "#aa8" } : undefined}
        >
          {hasCrop ? "Crop✓" : "Crop…"}
        </button>

        <button
          className={`yuv-diff-reload-btn${autoReload ? " yuv-diff-reload-btn--active" : ""}`}
          onClick={() => setAutoReload((v) => !v)}
          title={
            autoReload
              ? "Auto-reload active (click to disable)"
              : "Enable auto-reload every 3 s"
          }
        >
          {autoReload ? "⟳ Live" : "⟳ Auto"}
        </button>
      </div>

      {/* ── Error ────────────────────────────────────────────────────────── */}
      {error && <div className="yuv-diff-error">{error}</div>}

      {/* ── Crop dialog ──────────────────────────────────────────────────── */}
      {showCropDialog && (
        <CropDialog
          current={crop}
          onConfirm={handleCropApply}
          onCancel={() => setShowCropDialog(false)}
        />
      )}
    </div>
  );
});

export default YuvDiffPanel;
