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

import { useEffect } from "react";

export function useOptionsMenuEvents(): void {
  useEffect(() => {
    // CPU avx2 toggle — persist preference to localStorage
    const handleCpuAvx2 = () => {
      const key = "bitvue:cpu-avx2";
      const next = !(localStorage.getItem(key) === "true");
      localStorage.setItem(key, String(next));
    };
    // Codec settings — forward as viewer events so backend/panel can react
    const fwd = (eventName: string) => () =>
      window.dispatchEvent(new CustomEvent(eventName));
    const handleHevcExt = fwd("codec-hevc-extensions");
    const handleHevcIndex = fwd("codec-hevc-index");
    const handleHevcMv = fwd("codec-hevc-mv");
    const handleVvcDynamic = fwd("codec-vvc-dynamic");
    const handleVvcDetails = fwd("codec-vvc-details");
    const handleDigestForce = fwd("codec-digest-force");
    const handleDigestNone = fwd("codec-digest-none");
    const handleDigestStream = fwd("codec-digest-stream");
    // Auto-save layout toggle
    const handleAutoSaveLayout = () => {
      const key = "bitvue:auto-save-layout";
      const next = !(localStorage.getItem(key) === "true");
      localStorage.setItem(key, String(next));
    };

    window.addEventListener("menu-cpu-avx2", handleCpuAvx2);
    window.addEventListener("menu-codec-hevc-ext", handleHevcExt);
    window.addEventListener("menu-codec-hevc-index", handleHevcIndex);
    window.addEventListener("menu-codec-hevc-mv", handleHevcMv);
    window.addEventListener("menu-codec-vvc-dynamic", handleVvcDynamic);
    window.addEventListener("menu-codec-vvc-details", handleVvcDetails);
    window.addEventListener("menu-digest-force", handleDigestForce);
    window.addEventListener("menu-digest-none", handleDigestNone);
    window.addEventListener("menu-digest-stream", handleDigestStream);
    window.addEventListener("menu-auto-save-layout", handleAutoSaveLayout);
    return () => {
      window.removeEventListener("menu-cpu-avx2", handleCpuAvx2);
      window.removeEventListener("menu-codec-hevc-ext", handleHevcExt);
      window.removeEventListener("menu-codec-hevc-index", handleHevcIndex);
      window.removeEventListener("menu-codec-hevc-mv", handleHevcMv);
      window.removeEventListener("menu-codec-vvc-dynamic", handleVvcDynamic);
      window.removeEventListener("menu-codec-vvc-details", handleVvcDetails);
      window.removeEventListener("menu-digest-force", handleDigestForce);
      window.removeEventListener("menu-digest-none", handleDigestNone);
      window.removeEventListener("menu-digest-stream", handleDigestStream);
      window.removeEventListener("menu-auto-save-layout", handleAutoSaveLayout);
    };
  }, []);
}
