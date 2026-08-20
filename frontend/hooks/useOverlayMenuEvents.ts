/**
 * Overlay toggle/clear events (from View → Info Overlays submenu + Ctrl+F1–F6). Extracted from
 * App.tsx's `AppContent` (2026-08-20, axis-4 cleanup) -- see `useOptionsMenuEvents`'s doc for why
 * this got split out from the rest of App.tsx's menu-event wiring.
 */

import { useEffect } from "react";
import type { VisualizationMode } from "../utils/codecModeRegistry";

export function useOverlayMenuEvents(
  toggleOverlay: (mode: VisualizationMode) => void,
  clearOverlays: () => void,
): void {
  useEffect(() => {
    const handleToggleOverlay = (e: Event) => {
      const mode = (e as CustomEvent<string>).detail;
      if (mode) toggleOverlay(mode as VisualizationMode);
    };
    const handleClearOverlays = () => clearOverlays();
    // Ctrl+F1–F6: map to the first 6 available overlay modes
    const handleOverlayFKey = (e: Event) => {
      const fKey = (e as CustomEvent<number>).detail;
      const overlayKeys = [
        "mv-field",
        "qp-heatmap",
        "partition",
        "cbf-luma",
        "transform",
        "pred-mode",
      ] as const;
      const mode = overlayKeys[fKey - 1];
      if (mode) toggleOverlay(mode as VisualizationMode);
    };
    window.addEventListener("menu-toggle-overlay", handleToggleOverlay);
    window.addEventListener("menu-clear-overlays", handleClearOverlays);
    window.addEventListener("viewer-toggle-overlay-fkey", handleOverlayFKey);
    return () => {
      window.removeEventListener("menu-toggle-overlay", handleToggleOverlay);
      window.removeEventListener("menu-clear-overlays", handleClearOverlays);
      window.removeEventListener(
        "viewer-toggle-overlay-fkey",
        handleOverlayFKey,
      );
    };
  }, [toggleOverlay, clearOverlays]);
}
