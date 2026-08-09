/**
 * Context Menu Component
 *
 * Generic right-click menu -- renders a fixed list of items (already guard-evaluated by the
 * backend, see electronBridgeService's getContextMenuItems) at a screen position, with
 * disabled+tooltip state for guard-failed items. Closes on outside click, Escape, or item select.
 */

import { useEffect, useRef } from "react";
import type { ContextMenuItemWire } from "../services/electronBridgeService";
import "./ContextMenu.css";

interface ContextMenuProps {
  x: number;
  y: number;
  items: ContextMenuItemWire[];
  onSelect: (command: string) => void;
  onClose: () => void;
}

export function ContextMenu({
  x,
  y,
  items,
  onSelect,
  onClose,
}: ContextMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handlePointerDown = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        onClose();
      }
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    document.addEventListener("mousedown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("mousedown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [onClose]);

  if (items.length === 0) return null;

  return (
    <div
      ref={menuRef}
      className="context-menu"
      style={{ position: "fixed", top: y, left: x }}
      role="menu"
    >
      {items.map((item) => (
        <button
          key={item.id}
          type="button"
          role="menuitem"
          className="context-menu-item"
          disabled={!item.enabled}
          title={item.disabled_reason ?? undefined}
          onClick={() => {
            onSelect(item.command);
            onClose();
          }}
        >
          {item.label}
        </button>
      ))}
    </div>
  );
}
