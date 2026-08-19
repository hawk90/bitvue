/**
 * TAURI_WEB-006: the single choke point every "quit the app" trigger funnels through --
 * File > Quit / TitleBar's Quit menu item / TitleBar's in-window close (X) button / the
 * OS-native window-chrome close button all route through `requestQuit()` instead of calling
 * `app.quit()` directly, so a real open bitstream always gets a confirmation dialog first
 * (previously `app.quit()` was called unconditionally, discarding in-progress work with zero
 * confirmation, and the OS-native close button didn't go through any of this at all). See
 * `main.ts`'s `createWindow()` ('close' listener), `before-quit` handler, and the
 * `bitvue:closeWindow` IPC handler for the three call sites that wire into this module.
 */

import { app, dialog, type BrowserWindow } from "electron";

let mainWindow: BrowserWindow | undefined;
// Set to true only once the user has actually confirmed (or didn't need to confirm) quitting --
// every quit trigger checks this before doing anything destructive, and `requestQuit` is the
// only place that ever sets it.
let quitConfirmed = false;
let quitPending: Promise<void> | null = null;

/** Called once, right after the main window is created (`main.ts`'s `createWindow()`). */
export function setMainWindow(win: BrowserWindow | undefined): void {
  mainWindow = win;
}

/**
 * Whether `requestQuit()` has already decided to actually quit -- used by `main.ts`'s
 * `close`/`before-quit` listeners to let a real `app.quit()` (fired by `requestQuit()` itself)
 * through without re-triggering the confirmation dialog a second time.
 */
export function isQuitConfirmed(): boolean {
  return quitConfirmed;
}

/**
 * Reads `window.__BITVUE_HAS_OPEN_FILE__` (set by `App.tsx`'s `useOpenFileStatus` hook, see
 * `electronBridgeService.ts`'s `setHasOpenFile` doc on that property) straight out of the
 * renderer -- the one *real* "is there work worth losing" signal available anywhere in the
 * frontend/sidecar today. There's no tracked "export in progress" or "unsaved compare workspace"
 * state to check instead: exportEvidenceBundle awaits to completion with no cancel/progress
 * tracking (useExportEvidenceBundle.ts), and CompareWorkspace (the thing that would have a real
 * "unsaved session") is unmounted dead UI (see docs/DEVELOPMENT_PHASES.md's Phase 7.6 section).
 * Errs toward "nothing to lose" (false) if the renderer can't be reached (e.g. mid-navigation) --
 * a missed confirmation is far less bad than an unclosable app.
 */
async function hasOpenFileInRenderer(win: BrowserWindow): Promise<boolean> {
  try {
    return await win.webContents.executeJavaScript(
      "window.__BITVUE_HAS_OPEN_FILE__ === true",
    );
  } catch {
    return false;
  }
}

/**
 * Only shows a confirmation dialog when `hasOpenFileInRenderer` reports real work to lose --
 * a fresh, empty launch (WelcomeScreen, no file open yet) quits immediately with no prompt,
 * matching this session's explicit design goal ("not every close showing a dialog for a
 * freshly-launched empty app").
 *
 * Deduplicates concurrent callers via `quitPending` (e.g. `before-quit` and a window `close`
 * both firing for the same app.quit() call) so the dialog can't show twice for one quit attempt.
 */
export async function requestQuit(): Promise<void> {
  if (quitConfirmed) return;
  if (quitPending) return quitPending;
  quitPending = (async () => {
    const win = mainWindow;
    const hasOpenFile = win !== undefined && (await hasOpenFileInRenderer(win));
    if (hasOpenFile && win !== undefined) {
      const { response } = await dialog.showMessageBox(win, {
        type: "question",
        buttons: ["Cancel", "Quit"],
        defaultId: 0,
        cancelId: 0,
        message: "Quit Bitvue?",
        detail:
          "A bitstream is currently open. Quitting now will discard the current analysis session.",
      });
      if (response !== 1) {
        // Cancelled -- leave quitConfirmed false so the next quit attempt re-checks and
        // re-prompts (state may have changed, e.g. the user closed the file first).
        return;
      }
    }
    quitConfirmed = true;
    app.quit();
  })().finally(() => {
    quitPending = null;
  });
  return quitPending;
}
