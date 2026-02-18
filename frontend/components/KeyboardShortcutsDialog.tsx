/**
 * Keyboard Shortcuts Dialog
 *
 * Shows all available keyboard shortcuts
 */

import { useState, useEffect, memo } from "react";
import {
  KEYBOARD_SHORTCUTS,
  getShortcutDisplay,
  isMac,
} from "../utils/keyboardShortcuts";
import "./KeyboardShortcutsDialog.css";

interface KeyboardShortcutsDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

export const KeyboardShortcutsDialog = memo(function KeyboardShortcutsDialog({
  isOpen,
  onClose,
}: KeyboardShortcutsDialogProps) {
  const [isVisible, setIsVisible] = useState(false);

  useEffect(() => {
    if (isOpen) {
      setIsVisible(true);
    } else {
      setIsVisible(false);
    }
  }, [isOpen]);

  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape" && isOpen) {
        onClose();
      }
    };

    window.addEventListener("keydown", handleEscape);
    return () => window.removeEventListener("keydown", handleEscape);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  return (
    <div className="shortcuts-overlay" onClick={onClose}>
      <div
        className={`shortcuts-dialog ${isVisible ? "visible" : ""}`}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="shortcuts-header">
          <h2 className="shortcuts-title">Keyboard Shortcuts</h2>
          <button
            className="shortcuts-close"
            onClick={onClose}
            aria-label="Close"
          >
            <span className="codicon codicon-close"></span>
          </button>
        </div>

        <div className="shortcuts-content">
          {KEYBOARD_SHORTCUTS.map((category) => (
            <div key={category.name} className="shortcuts-category">
              <h3 className="shortcuts-category-title">{category.name}</h3>
              <div className="shortcuts-list">
                {category.shortcuts.map((shortcut, idx) => (
                  <div key={idx} className="shortcuts-item">
                    <span className="shortcuts-description">
                      {shortcut.description}
                    </span>
                    <kbd className="shortcuts-key">
                      {getShortcutDisplay(shortcut)}
                    </kbd>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>

        <div className="shortcuts-footer">
          <span className="shortcuts-platform">
            {isMac() ? "macOS" : "Windows/Linux"}
          </span>
          <button className="shortcuts-btn" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
});
