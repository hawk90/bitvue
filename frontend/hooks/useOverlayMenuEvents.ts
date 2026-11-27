/**
 * Overlay toggle/clear events (from View → Info Overlays submenu + Ctrl+F1–F6). Extracted from
 * App.tsx's `AppContent` (2026-08-20, axis-4 cleanup) -- see `useOptionsMenuEvents`'s doc for why
 * this got split out from the rest of App.tsx's menu-event wiring.
 */

import type {
  CodecModeEntry,
  VisualizationMode,
} from "../utils/codecModeRegistry";
import { useWindowEventMap } from "./useWindowEvent";

export function useOverlayMenuEvents(
  toggleOverlay: (mode: VisualizationMode) => void,
  clearOverlays: () => void,
  availableOverlays: CodecModeEntry[],
): void {
  useWindowEventMap({
    "menu-toggle-overlay": (e) => {
      const mode = (e as CustomEvent<string>).detail;
      if (mode) toggleOverlay(mode as VisualizationMode);
    },
    "menu-clear-overlays": () => clearOverlays(),
    // Ctrl+F1-F6: map to the first 6 available overlay modes *for the current codec*. Used to be
    // a single hardcoded list of mode strings shared across all codecs (mv-field/qp-heatmap/
    // partition/cbf-luma/transform/pred-mode) -- none of which actually match any real codec's
    // registry entries (toggleOverlay's own availability guard silently no-ops any mode not in
    // availableOverlays, so this was a dead shortcut for every codec: AV1's real 4 overlays are
    // heat-map/block-type/efficiency-map/psnr-overlay, HEVC's don't include "qp-heatmap" either,
    // just "qp-map" -- confirmed via a real screenshot before this fix). Now reads the actual
    // per-codec list, matching the doc comment's original documented intent.
    "viewer-toggle-overlay-fkey": (e) => {
      const fKey = (e as CustomEvent<number>).detail;
      const mode = availableOverlays[fKey - 1]?.mode;
      if (mode) toggleOverlay(mode);
    },
  });
}
