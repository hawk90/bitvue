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
