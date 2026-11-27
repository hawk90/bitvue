/**
 * Layout menu events — save/load/reset layout. Extracted from App.tsx's `AppContent`
 * (2026-08-20, axis-4 cleanup) -- see `useOptionsMenuEvents`'s doc for why this got split out
 * from the rest of App.tsx's menu-event wiring.
 */

import { useWindowEventMap } from "./useWindowEvent";

export function useLayoutMenuEvents(
  saveLayout: () => void,
  loadLayout: () => void,
  resetLayout: () => void,
): void {
  useWindowEventMap({
    "menu-save-layout": () => saveLayout(),
    "menu-load-layout": () => loadLayout(),
    "menu-reset-layout": () => resetLayout(),
  });
}
