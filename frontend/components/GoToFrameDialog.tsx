/**
 * Go To Frame Dialog
 *
 * Lets the user type a frame number and jump directly to it.
 * Triggered by Ctrl+G / Ctrl+F keyboard shortcuts or the View menu.
 */

import { memo, useState, useEffect, useRef, useCallback } from "react";
import "./GoToFrameDialog.css";

interface GoToFrameDialogProps {
  isOpen: boolean;
  onClose: () => void;
  currentIndex: number;
  totalFrames: number;
  onGoTo: (index: number) => void;
}

export const GoToFrameDialog = memo(function GoToFrameDialog({
  isOpen,
  onClose,
  currentIndex,
  totalFrames,
  onGoTo,
}: GoToFrameDialogProps) {
  const [value, setValue] = useState("");
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Reset and focus when opened
  useEffect(() => {
    if (isOpen) {
      setValue(String(currentIndex + 1)); // 1-based display
      setError(null);
      setTimeout(() => {
        inputRef.current?.focus();
        inputRef.current?.select();
      }, 0);
    }
  }, [isOpen, currentIndex]);

  const handleSubmit = useCallback(() => {
    const num = parseInt(value, 10);
    if (isNaN(num) || num < 1 || num > totalFrames) {
      setError(`Enter a number between 1 and ${totalFrames}`);
      return;
    }
    onGoTo(num - 1); // convert to 0-based
    onClose();
  }, [value, totalFrames, onGoTo, onClose]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        handleSubmit();
      } else if (e.key === "Escape") {
        onClose();
      }
    },
    [handleSubmit, onClose],
  );

  if (!isOpen) return null;

  return (
    <div
      className="goto-overlay"
      onClick={onClose}
      role="dialog"
      aria-modal="true"
      aria-label="Go to frame"
    >
      <div className="goto-dialog" onClick={(e) => e.stopPropagation()}>
        <div className="goto-header">
          <span className="goto-title">Go to Frame</span>
          <button className="goto-close" onClick={onClose} aria-label="Close">
            ×
          </button>
        </div>

        <div className="goto-body">
          <label className="goto-label" htmlFor="goto-input">
            Frame number (1–{totalFrames})
          </label>
          <input
            id="goto-input"
            ref={inputRef}
            className={`goto-input${error ? " goto-input--error" : ""}`}
            type="number"
            min={1}
            max={totalFrames}
            value={value}
            onChange={(e) => {
              setValue(e.target.value);
              setError(null);
            }}
            onKeyDown={handleKeyDown}
            aria-describedby={error ? "goto-error" : undefined}
          />
          {error && (
            <span id="goto-error" className="goto-error" role="alert">
              {error}
            </span>
          )}
        </div>

        <div className="goto-footer">
          <button className="goto-btn goto-btn--cancel" onClick={onClose}>
            Cancel
          </button>
          <button className="goto-btn goto-btn--primary" onClick={handleSubmit}>
            Go
          </button>
        </div>
      </div>
    </div>
  );
});
