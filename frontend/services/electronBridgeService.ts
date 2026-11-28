/**
 * Electron sidecar bridge: thin wrappers around `window.bitvue.*`, exposed by
 * `bitvue-desktop/electron/preload.cjs` via `contextBridge`. It replaced the command layer the app
 * had before the Electron move (2026-08); the `bitvue-sidecar` commands differ from those in name
 * and in param/result shape (see `docs/DEVELOPMENT_PHASES.md` § "제품 아키텍처 확정"), so look a
 * panel's data source up here instead of assuming an old command still exists.
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
export * from "./bridge/frameList";
export * from "./bridge/frameDecode";
export * from "./bridge/frameAnalysis";
export * from "./bridge/syntaxHex";
export * from "./bridge/compareDiff";
export * from "./bridge/evidenceExport";
export * from "./bridge/fileExport";
export * from "./bridge/windowLifecycle";
