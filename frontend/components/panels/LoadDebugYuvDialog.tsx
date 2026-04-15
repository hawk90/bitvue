/**
 * Load Debug YUV Dialog
 *
 * Shown when the user selects Debug → Open debug YUV…
 * Lets the user confirm or override the auto-detected width/height and choose
 * the YUV format / bit depth.
 *
 * Auto-detection heuristic:
 *   - Tries to parse WxH from filename (e.g. "1920x1080" or "1920_1080")
 *   - Tries to infer format / bitdepth from filename tokens
 *     ("420", "422", "444", "nv12", "10bit", "8bit", etc.)
 *   - Falls back to 1920×1080 / I420 / 8-bit
 */

import { memo, useState, useCallback } from "react";
import type { YuvFormat } from "../../contexts/YuvDiffContext";
import "./YuvDiffPanel.css";

interface LoadDebugYuvDialogProps {
  filePath: string;
  onConfirm: (params: {
    width: number;
    height: number;
    format: YuvFormat;
    bitdepth: number;
  }) => void;
  onCancel: () => void;
}

const FORMAT_OPTIONS: { value: YuvFormat; label: string }[] = [
  { value: "i420", label: "I420  (4:2:0 planar)" },
  { value: "nv12", label: "NV12  (4:2:0 semi-planar)" },
  { value: "nv21", label: "NV21  (4:2:0 semi-planar, UV swapped)" },
  { value: "i422", label: "I422  (4:2:2 planar)" },
  { value: "i444", label: "I444  (4:4:4 planar)" },
];

const BITDEPTH_OPTIONS = [8, 10, 12, 16];

/** Try to extract WxH, format and bitdepth from a filename. */
function autoDetect(path: string): {
  width: number;
  height: number;
  format: YuvFormat;
  bitdepth: number;
} {
  const name = path.split(/[\\/]/).pop() ?? "";

  // Resolution: 1920x1080 or 1920_1080
  const resMatch = name.match(/(\d{3,4})[x_](\d{3,4})/i);
  const width = resMatch ? parseInt(resMatch[1], 10) : 1920;
  const height = resMatch ? parseInt(resMatch[2], 10) : 1080;

  // Format tokens
  const lower = name.toLowerCase();
  let format: YuvFormat = "i420";
  if (lower.includes("nv21")) format = "nv21";
  else if (lower.includes("nv12")) format = "nv12";
  else if (lower.includes("444")) format = "i444";
  else if (lower.includes("422")) format = "i422";
  else if (lower.includes("420")) format = "i420";

  // Bitdepth
  let bitdepth = 8;
  if (lower.includes("16bit") || lower.includes("16b") || lower.includes("_16"))
    bitdepth = 16;
  else if (
    lower.includes("12bit") ||
    lower.includes("12b") ||
    lower.includes("_12")
  )
    bitdepth = 12;
  else if (
    lower.includes("10bit") ||
    lower.includes("10b") ||
    lower.includes("_10")
  )
    bitdepth = 10;

  return { width, height, format, bitdepth };
}

export const LoadDebugYuvDialog = memo(function LoadDebugYuvDialog({
  filePath,
  onConfirm,
  onCancel,
}: LoadDebugYuvDialogProps) {
  const detected = autoDetect(filePath);
  const [width, setWidth] = useState(detected.width);
  const [height, setHeight] = useState(detected.height);
  const [format, setFormat] = useState<YuvFormat>(detected.format);
  const [bitdepth, setBitdepth] = useState(detected.bitdepth);

  const filename = filePath.split(/[\\/]/).pop() ?? filePath;

  // Estimate frame count from file size (display only)
  const frameSize = (() => {
    const bps = bitdepth > 8 ? 2 : 1;
    const luma = width * height * bps;
    const chroma =
      format === "i444"
        ? luma * 2
        : format === "i422"
          ? luma
          : Math.round(luma * 0.5);
    return luma + chroma;
  })();

  const handleConfirm = useCallback(() => {
    onConfirm({ width, height, format, bitdepth });
  }, [onConfirm, width, height, format, bitdepth]);

  return (
    <div
      className="yuv-diff-dialog-overlay"
      onClick={(e) => e.target === e.currentTarget && onCancel()}
    >
      <div className="yuv-diff-dialog" role="dialog" aria-modal>
        <h3>Load Reference YUV</h3>

        <div className="yuv-diff-dialog-path" title={filePath}>
          {filename}
        </div>

        {/* Width / Height */}
        <div className="yuv-diff-dialog-row">
          <span className="yuv-diff-dialog-label">Resolution</span>
          <input
            type="number"
            min={1}
            max={16384}
            value={width}
            onChange={(e) =>
              setWidth(Math.max(1, parseInt(e.target.value, 10) || 1))
            }
            aria-label="Width"
          />
          <span style={{ color: "#666" }}>×</span>
          <input
            type="number"
            min={1}
            max={16384}
            value={height}
            onChange={(e) =>
              setHeight(Math.max(1, parseInt(e.target.value, 10) || 1))
            }
            aria-label="Height"
          />
        </div>

        {/* Format */}
        <div className="yuv-diff-dialog-row">
          <span className="yuv-diff-dialog-label">Format</span>
          <select
            value={format}
            onChange={(e) => setFormat(e.target.value as YuvFormat)}
          >
            {FORMAT_OPTIONS.map((o) => (
              <option key={o.value} value={o.value}>
                {o.label}
              </option>
            ))}
          </select>
        </div>

        {/* Bit depth */}
        <div className="yuv-diff-dialog-row">
          <span className="yuv-diff-dialog-label">Bit depth</span>
          <select
            value={bitdepth}
            onChange={(e) => setBitdepth(parseInt(e.target.value, 10))}
          >
            {BITDEPTH_OPTIONS.map((bd) => (
              <option key={bd} value={bd}>
                {bd}-bit
              </option>
            ))}
          </select>
        </div>

        {/* Frame size hint */}
        <div className="yuv-diff-dialog-hint">
          {frameSize > 0 ? `~${(frameSize / 1024).toFixed(0)} KB / frame` : ""}
        </div>

        <div className="yuv-diff-dialog-buttons">
          <button className="yuv-diff-dialog-btn-cancel" onClick={onCancel}>
            Cancel
          </button>
          <button className="yuv-diff-dialog-btn-load" onClick={handleConfirm}>
            Load
          </button>
        </div>
      </div>
    </div>
  );
});

export default LoadDebugYuvDialog;
