/**
 * Mode menu events (macOS native Mode menu, `bitvue-desktop/electron/nativeMenu.ts`).
 *
 * The native menu dispatches `MENU_EVENTS.modeChange` with a `VisualizationMode` key as its
 * detail; this routes it to the same `ModeContext.setMode` the Windows/Linux TitleBar Mode menu
 * calls via `onModeChange`, so both platforms reach every mode (including the `residuals` /
 * `av1-features` legacy modes) identically. Before this hook nothing listened for the event, so
 * the whole mac Mode menu was a no-op.
 */

import { useEffect } from "react";
import { MENU_EVENTS } from "../../bitvue-desktop/electron/menuEvents";
import type { VisualizationMode } from "../utils/codecModeRegistry";

export function useModeMenuEvents(
  setMode: (mode: VisualizationMode) => void,
): void {
  useEffect(() => {
    const handleModeChange = (e: Event) => {
      const mode = (e as CustomEvent<unknown>).detail;
      // setMode itself ignores keys that aren't valid for the current codec / legacy list.
      if (typeof mode === "string" && mode) setMode(mode as VisualizationMode);
    };
    window.addEventListener(MENU_EVENTS.modeChange, handleModeChange);
    return () => {
      window.removeEventListener(MENU_EVENTS.modeChange, handleModeChange);
    };
  }, [setMode]);
}
