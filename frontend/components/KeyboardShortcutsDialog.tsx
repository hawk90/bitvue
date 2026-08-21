/**
 * Keyboard Shortcuts Dialog
 *
 * Shows all available keyboard shortcuts
 */

import { useEffect, memo } from "react";
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
        className="shortcuts-dialog visible"
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
              <table className="shortcuts-table">
                <tbody>
                  {category.shortcuts.map((shortcut) => (
                    <tr
                      key={`${shortcut.key}-${shortcut.ctrl}-${shortcut.meta}-${shortcut.shift}-${shortcut.alt}-${shortcut.description}`}
                    >
                      <td className="shortcuts-description">
                        {shortcut.description}
                      </td>
                      <td className="shortcuts-key-cell">
                        <kbd className="shortcuts-key">
                          {getShortcutDisplay(shortcut)}
                        </kbd>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
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
