/**
 * Options > Dark/Light Theme menu events. Both the macOS native menu (`nativeMenu.ts`) and the
 * Windows/Linux custom `TitleBar` dispatch `MENU_EVENTS.themeChange` with `"dark" | "light"` as the
 * detail; this routes it to `ThemeContext.setTheme`. Extracted from App.tsx's inline effect so the
 * menu → theme path is testable without rendering the whole App.
 */

import { MENU_EVENTS } from "../../bitvue-desktop/electron/menuEvents";
import type { Theme } from "../contexts/ThemeContext";
import { useWindowEventMap } from "./useWindowEvent";

export function useThemeMenuEvents(setTheme: (theme: Theme) => void): void {
  useWindowEventMap({
    [MENU_EVENTS.themeChange]: (e) => {
      const theme = (e as CustomEvent<unknown>).detail;
      if (theme === "dark" || theme === "light") setTheme(theme);
    },
  });
}
