/**
 * Electron sidecar bridge — the replacement for the old `@tauri-apps/api` `invoke()`-based
 * command layer (`tauriCommandService.ts`, deleted 2026-08-20, and the various files that called
 * `invoke()` directly). Thin wrappers around `window.bitvue.*`, exposed by
 * `bitvue-desktop/electron/preload.cjs` via `contextBridge`.
 *
 * IMPORTANT — the old Tauri command surface (~40 commands, `src-tauri/src/commands/*.rs`,
 * deleted 2026-08-08 -- though a handful of *frontend* files still call `@tauri-apps/api` directly
 * and haven't been migrated to this bridge yet, e.g. `utils/exportUtils.ts`'s CSV/JSON/report
 * export -- that one's a real backend gap, not a simple rewiring job, see its own warning comment
 * -- and a few intentionally-dead panels flagged in place; grep the frontend tree before assuming
 * a panel is covered) and the new `bitvue-sidecar` surface (see `docs/DEVELOPMENT_PHASES.md` §
 * "제품 아키텍처 확정") are NOT the same API — different names, different param/result shapes.
 *
 * This module used to be one 1006-line file mixing every wire type, the IPC contract, and every
 * wrapper function together. Split 2026-08-20 (axis-1 cleanup) into `services/bridge/*` by
 * feature domain -- `core` (shared IPC contract + `requireBridge`), `stream`, `frameDecode`,
 * `frameAnalysis`, `syntaxHex`, `compareDiff`, `evidenceExport`, `windowLifecycle`. This file is
 * now just a re-export barrel so every existing `from "./services/electronBridgeService"` /
 * `from "../services/electronBridgeService"` import across the frontend keeps working unchanged
 * -- go to `services/bridge/<domain>.ts` directly for the real implementation and its doc
 * comments.
 */

export * from "./bridge/core";
export * from "./bridge/stream";
export * from "./bridge/frameDecode";
export * from "./bridge/frameAnalysis";
export * from "./bridge/syntaxHex";
export * from "./bridge/compareDiff";
export * from "./bridge/evidenceExport";
export * from "./bridge/windowLifecycle";
