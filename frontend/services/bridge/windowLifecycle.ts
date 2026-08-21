/**
 * Window lifecycle domain — native file dialogs and app-window chrome. No sidecar involvement at
 * all here, unlike every other domain (these are pure Electron main-process calls, see
 * `bitvue-desktop/electron/main.ts`'s `registerIpcHandlers`). See `services/bridge/core.ts`'s
 * module doc for how this file fits into the overall bridge split.
 */

import { requireBridge } from "./core";

export interface OpenFileDialogFilter {
  name: string;
  extensions: string[];
}

/** Native "open file" dialog via the main process. Returns the selected path, or null if cancelled. */
export async function showOpenDialog(
  filters?: OpenFileDialogFilter[],
): Promise<string | null> {
  return requireBridge().showOpenDialog(filters);
}

/** Whether `path` exists on disk right now. Used to prune stale entries out of the welcome
 *  screen's Recent Files list. */
export async function pathExists(path: string): Promise<boolean> {
  return requireBridge().pathExists(path);
}

/** Resolves a bundled sample's filename (e.g. "foreman_av1.ivf") to its real absolute path.
 *  Used by the welcome screen's "Samples" quick-open list. */
export async function getSamplePath(filename: string): Promise<string> {
  return requireBridge().getSamplePath(filename);
}

/** Quits the whole app (not just the current window) -- the "Quit" menu item / TitleBar button's
 *  intent, matches cross-platform app.quit() semantics in the main process. */
export async function closeWindow(): Promise<void> {
  return requireBridge().closeWindow();
}

/** TitleBar's minimize button (Windows/Linux only, see shouldShowTitleBar). */
export async function minimizeWindow(): Promise<void> {
  return requireBridge().minimizeWindow();
}

/** TitleBar's maximize/restore button (Windows/Linux only, see shouldShowTitleBar). */
export async function toggleMaximizeWindow(): Promise<void> {
  return requireBridge().toggleMaximizeWindow();
}

/** Subscribes to the sidecar-process-crashed-and-was-respawned notification (see
 *  `preload.cjs`'s own doc on `onSidecarRestarted`) -- `bitvue_engine::Core`'s in-memory state
 *  (open streams, selection) does NOT survive a restart, so callers should treat this as "prompt
 *  the user to re-open whatever they had open." Returns an unsubscribe function. */
export function onSidecarRestarted(callback: () => void): () => void {
  return requireBridge().onSidecarRestarted(callback);
}
