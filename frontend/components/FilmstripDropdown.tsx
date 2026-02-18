/**
 * Filmstrip Dropdown Component
 *
 * View mode selector for filmstrip display
 */

import { useState, useEffect, useRef, memo, useCallback } from "react";
import "./FilmstripDropdown.css";

export type DisplayView =
  | "thumbnails"
  | "sizes"
  | "bpyramid"
  | "timeline"
  | "hrdbuffer"
  | "enhanced";

interface FilmstripDropdownProps {
  displayView: DisplayView;
  onViewChange: (view: DisplayView) => void;
}

export const FilmstripDropdown = memo(function FilmstripDropdown({
  displayView,
  onViewChange,
}: FilmstripDropdownProps) {
  const [dropdownOpen, setDropdownOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!dropdownOpen) return;

    const handleClickOutside = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      if (dropdownRef.current && !dropdownRef.current.contains(target)) {
        setDropdownOpen(false);
      }
    };

    document.addEventListener("click", handleClickOutside);
    return () => {
      document.removeEventListener("click", handleClickOutside);
    };
  }, [dropdownOpen]);

  const handleSelectView = useCallback(
    (view: DisplayView) => {
      onViewChange(view);
      setDropdownOpen(false);
    },
    [onViewChange],
  );

  const handleToggleDropdown = useCallback(() => {
    setDropdownOpen((prev) => !prev);
  }, []);

  const getViewLabel = (view: DisplayView): string => {
    switch (view) {
      case "thumbnails":
        return "Thumbnails";
      case "sizes":
        return "Frame Sizes";
      case "bpyramid":
        return "B-Pyramid";
      case "timeline":
        return "Timeline";
      case "hrdbuffer":
        return "HRD Buffer";
      case "enhanced":
        return "Enhanced";
    }
  };

  return (
    <div ref={dropdownRef} className="filmstrip-dropdown">
      <button
        className="filmstrip-dropdown-trigger"
        onMouseDown={handleToggleDropdown}
        aria-label="View mode"
        aria-haspopup="listbox"
        aria-expanded={dropdownOpen}
      >
        <span className="filmstrip-dropdown-label">
          {getViewLabel(displayView)}
        </span>
        <span
          className="codicon codicon-chevron-down"
          aria-hidden="true"
        ></span>
      </button>

      {dropdownOpen && (
        <ul className="filmstrip-dropdown-menu" role="listbox">
          <li>
            <button
              className={displayView === "thumbnails" ? "active" : ""}
              onClick={() => handleSelectView("thumbnails")}
              role="option"
              aria-selected={displayView === "thumbnails"}
            >
              <span>Thumbnails</span>
            </button>
          </li>
          <li>
            <button
              className={displayView === "sizes" ? "active" : ""}
              onClick={() => handleSelectView("sizes")}
              role="option"
              aria-selected={displayView === "sizes"}
            >
              <span>Frame Sizes</span>
            </button>
          </li>
          <li>
            <button
              className={displayView === "bpyramid" ? "active" : ""}
              onClick={() => handleSelectView("bpyramid")}
              role="option"
              aria-selected={displayView === "bpyramid"}
            >
              <span>B-Pyramid</span>
            </button>
          </li>
          <li>
            <button
              className={displayView === "timeline" ? "active" : ""}
              onClick={() => handleSelectView("timeline")}
              role="option"
              aria-selected={displayView === "timeline"}
            >
              <span>Timeline</span>
            </button>
          </li>
          <li>
            <button
              className={displayView === "hrdbuffer" ? "active" : ""}
              onClick={() => handleSelectView("hrdbuffer")}
              role="option"
              aria-selected={displayView === "hrdbuffer"}
            >
              <span>HRD Buffer</span>
            </button>
          </li>
          <li>
            <button
              className={displayView === "enhanced" ? "active" : ""}
              onClick={() => handleSelectView("enhanced")}
              role="option"
              aria-selected={displayView === "enhanced"}
            >
              <span>Enhanced</span>
            </button>
          </li>
        </ul>
      )}
    </div>
  );
});
