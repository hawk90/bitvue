/**
 * Kill switch for the WebGPU frame renderer.
 *
 * WebGPU support in Chromium/Electron is confirmed flaky on Linux (open Electron
 * issues on adapter/swiftshader failures) -- this must be disableable without a
 * rebuild so a broken adapter on a given machine degrades to the known-working
 * Canvas2D path instead of a blank canvas.
 *
 * Default is enabled; set `localStorage["bitvue:disableWebGpu"] = "1"` to force
 * the Canvas2D fallback.
 */
const STORAGE_KEY = "bitvue:disableWebGpu";

export function isWebGpuDisabledByFlag(): boolean {
  try {
    return (
      typeof localStorage !== "undefined" &&
      localStorage.getItem(STORAGE_KEY) === "1"
    );
  } catch {
    // localStorage can throw in restricted contexts (e.g. sandboxed iframes) -- treat as not disabled.
    return false;
  }
}
