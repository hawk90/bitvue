/**
 * Filmstrip Dropdown Component
 *
 * View mode selector for filmstrip display
 */

import { useState, useEffect, useRef, memo, useCallback } from "react";
import { createPortal } from "react-dom";
import "./FilmstripDropdown.css";

export type DisplayView =
  | "thumbnails"
  | "sizes"
  | "bpyramid"
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
  const [menuPosition, setMenuPosition] = useState({ top: 0, right: 0 });
  const dropdownRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLUListElement>(null);

  // The filmstrip panel's ancestor chain includes a couple of `overflow: hidden` boxes
  // (TimelineFilmstrip.css's `.timeline-filmstrip`, DockableLayout.css's `.dockable-layout`) that
  // exist to contain the filmstrip's own scrolling content -- with the menu physically nested
  // inside that DOM subtree, any of its `position: absolute` bounds falling below the filmstrip's
  // own ~140-180px box got hard-clipped, hiding the later options entirely (and re-opening after
  // picking a later one made it worse, since the box just clipped in the same place again).
  // Porting it to `document.body` at a `position: fixed` coordinate computed from the trigger's
  // real screen position sidesteps every ancestor's overflow instead of trying to carve out an
  // exception in each one.
  useEffect(() => {
    if (!dropdownOpen) return;
    const rect = triggerRef.current?.getBoundingClientRect();
    if (rect) {
      setMenuPosition({
        top: rect.bottom + 4,
        right: window.innerWidth - rect.right,
      });
    }

    const handleClickOutside = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      if (
        dropdownRef.current &&
        !dropdownRef.current.contains(target) &&
        menuRef.current &&
        !menuRef.current.contains(target)
      ) {
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
      case "hrdbuffer":
        return "HRD Buffer";
      case "enhanced":
        return "Enhanced";
    }
  };

  return (
    <div ref={dropdownRef} className="filmstrip-dropdown">
      <button
        ref={triggerRef}
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

      {dropdownOpen &&
        createPortal(
          <ul
            ref={menuRef}
            className="filmstrip-dropdown-menu"
            role="listbox"
            style={{
              position: "fixed",
              top: menuPosition.top,
              right: menuPosition.right,
            }}
          >
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
          </ul>,
          document.body,
        )}
    </div>
  );
});
