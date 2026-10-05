/**
 * Options > Dark/Light Theme menu events. Both the macOS native menu (`nativeMenu.ts`) and the
 * Windows/Linux custom `TitleBar` dispatch `MENU_EVENTS.themeChange` with `"dark" | "light"` as the
 * detail; this routes it to `ThemeContext.setTheme`. Extracted from App.tsx's inline effect so the
 * menu → theme path is testable without rendering the whole App.
 */

import { useEffect } from "react";
import { MENU_EVENTS } from "../../bitvue-desktop/electron/menuEvents";
import type { Theme } from "../contexts/ThemeContext";

export function useThemeMenuEvents(setTheme: (theme: Theme) => void): void {
  useEffect(() => {
    const handleThemeChange = (e: Event) => {
      const theme = (e as CustomEvent<unknown>).detail;
      if (theme === "dark" || theme === "light") setTheme(theme);
    };
    window.addEventListener(MENU_EVENTS.themeChange, handleThemeChange);
    return () => {
      window.removeEventListener(MENU_EVENTS.themeChange, handleThemeChange);
    };
  }, [setTheme]);
}
