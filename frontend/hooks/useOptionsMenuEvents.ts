/**
 * Options menu events — CPU AVX2 toggle, HEVC/VVC codec-setting toggles, digest mode, auto-save
 * layout. Extracted from App.tsx's `AppContent` (2026-08-20, axis-4 cleanup -- App.tsx had grown
 * ~190 lines of `window.addEventListener("menu-...")` wiring across 5 separate useEffect blocks,
 * one per native menu category; splitting each into its own hook mirrors how
 * `bitvue-desktop/electron/nativeMenu.ts`'s menu template is itself organized by these same
 * categories). Fully self-contained -- no props, no external state -- since every handler here
 * either writes straight to `localStorage` or re-dispatches a `codec-*` CustomEvent for whichever
 * panel cares to listen, rather than needing anything from `AppContent`.
 */

import { useWindowEventMap } from "./useWindowEvent";

// CPU avx2 / auto-save-layout toggles — persist preference to localStorage
const toggleStored = (key: string) => () => {
  const next = !(localStorage.getItem(key) === "true");
  localStorage.setItem(key, String(next));
};
// Codec settings — forward as viewer events so backend/panel can react
const fwd = (eventName: string) => () =>
  window.dispatchEvent(new CustomEvent(eventName));

const OPTIONS_HANDLERS: Record<string, () => void> = {
  "menu-cpu-avx2": toggleStored("bitvue:cpu-avx2"),
  "menu-codec-hevc-ext": fwd("codec-hevc-extensions"),
  "menu-codec-hevc-index": fwd("codec-hevc-index"),
  "menu-codec-hevc-mv": fwd("codec-hevc-mv"),
  "menu-codec-vvc-dynamic": fwd("codec-vvc-dynamic"),
  "menu-codec-vvc-details": fwd("codec-vvc-details"),
  "menu-digest-force": fwd("codec-digest-force"),
  "menu-digest-none": fwd("codec-digest-none"),
  "menu-digest-stream": fwd("codec-digest-stream"),
  "menu-auto-save-layout": toggleStored("bitvue:auto-save-layout"),
};

export function useOptionsMenuEvents(): void {
  useWindowEventMap(OPTIONS_HANDLERS);
}
