/**
 * Crop Dialog
 *
 * Lets the user set L/R/T/B pixel crop values applied to the decoded frame
 * before comparison with the reference YUV.
 */

import { memo, useState, useCallback } from "react";
import "./YuvDiffPanel.css";

export interface CropValues {
  left: number;
  right: number;
  top: number;
  bottom: number;
}

interface CropDialogProps {
  current: CropValues;
  onConfirm: (crop: CropValues) => void;
  onCancel: () => void;
}

export const CropDialog = memo(function CropDialog({
  current,
  onConfirm,
  onCancel,
}: CropDialogProps) {
  const [left, setLeft] = useState(current.left);
  const [right, setRight] = useState(current.right);
  const [top, setTop] = useState(current.top);
  const [bottom, setBottom] = useState(current.bottom);

  const parseUint = (v: string) => Math.max(0, parseInt(v, 10) || 0);

  const handleConfirm = useCallback(() => {
    onConfirm({ left, right, top, bottom });
  }, [onConfirm, left, right, top, bottom]);

  const handleReset = useCallback(() => {
    setLeft(0);
    setRight(0);
    setTop(0);
    setBottom(0);
  }, []);

  return (
    <div
      className="yuv-diff-dialog-overlay"
      onClick={(e) => e.target === e.currentTarget && onCancel()}
    >
      <div className="yuv-diff-dialog" role="dialog" aria-modal>
        <h3>Crop Settings</h3>

        <div style={{ fontSize: 11, color: "#888", marginBottom: 4 }}>
          Pixels to remove from each side of the decoded frame before
          comparison.
        </div>

        <div className="yuv-diff-crop-grid">
          <div className="yuv-diff-crop-field">
            <label htmlFor="crop-top">Top</label>
            <input
              id="crop-top"
              type="number"
              min={0}
              value={top}
              onChange={(e) => setTop(parseUint(e.target.value))}
            />
          </div>
          <div className="yuv-diff-crop-field">
            <label htmlFor="crop-bottom">Bottom</label>
            <input
              id="crop-bottom"
              type="number"
              min={0}
              value={bottom}
              onChange={(e) => setBottom(parseUint(e.target.value))}
            />
          </div>
          <div className="yuv-diff-crop-field">
            <label htmlFor="crop-left">Left</label>
            <input
              id="crop-left"
              type="number"
              min={0}
              value={left}
              onChange={(e) => setLeft(parseUint(e.target.value))}
            />
          </div>
          <div className="yuv-diff-crop-field">
            <label htmlFor="crop-right">Right</label>
            <input
              id="crop-right"
              type="number"
              min={0}
              value={right}
              onChange={(e) => setRight(parseUint(e.target.value))}
            />
          </div>
        </div>

        <div className="yuv-diff-dialog-buttons">
          <button
            className="yuv-diff-dialog-btn-cancel"
            onClick={handleReset}
            style={{ marginRight: "auto" }}
          >
            Reset
          </button>
          <button className="yuv-diff-dialog-btn-cancel" onClick={onCancel}>
            Cancel
          </button>
          <button className="yuv-diff-dialog-btn-load" onClick={handleConfirm}>
            Apply
          </button>
        </div>
      </div>
    </div>
  );
});

export default CropDialog;
