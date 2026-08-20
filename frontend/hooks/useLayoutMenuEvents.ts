/**
 * Layout menu events — save/load/reset layout. Extracted from App.tsx's `AppContent`
 * (2026-08-20, axis-4 cleanup) -- see `useOptionsMenuEvents`'s doc for why this got split out
 * from the rest of App.tsx's menu-event wiring.
 */

import { useEffect } from "react";

export function useLayoutMenuEvents(
  saveLayout: () => void,
  loadLayout: () => void,
  resetLayout: () => void,
): void {
  useEffect(() => {
    const handleSaveLayout = () => saveLayout();
    const handleLoadLayout = () => loadLayout();
    const handleResetLayout = () => resetLayout();
    window.addEventListener("menu-save-layout", handleSaveLayout);
    window.addEventListener("menu-load-layout", handleLoadLayout);
    window.addEventListener("menu-reset-layout", handleResetLayout);
    return () => {
      window.removeEventListener("menu-save-layout", handleSaveLayout);
      window.removeEventListener("menu-load-layout", handleLoadLayout);
      window.removeEventListener("menu-reset-layout", handleResetLayout);
    };
  }, [saveLayout, loadLayout, resetLayout]);
}
