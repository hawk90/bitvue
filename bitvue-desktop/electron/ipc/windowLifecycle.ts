/**
 * Window lifecycle domain IPC handlers — native "open file" dialog and app-window chrome (quit,
 * minimize, maximize). Mirrors `frontend/services/bridge/windowLifecycle.ts` channel-for-channel
 * (2026-08-20 axis-1 split — see `main.ts`'s `registerIpcHandlers` doc for why this got split
 * out). No sidecar involvement at all here, unlike every other domain module.
 */

import { BrowserWindow, ipcMain, dialog } from "electron";
import { requestQuit } from "../quitGuard.js";

export function registerWindowLifecycleIpcHandlers(): void {
  ipcMain.handle(
    "bitvue:showOpenDialog",
    async (event, filters?: Array<{ name: string; extensions: string[] }>) => {
      // Test-only bypass: a real native OS dialog can't be scripted in an automated/offscreen
      // run. Only takes effect when this env var is explicitly set (never in normal usage) --
      // lets BITVUE_ELECTRON_SCREENSHOT drive the actual production handleOpenFile() code path
      // (real dialog call site, just answered without a human) instead of calling bridge
      // functions directly the way BITVUE_ELECTRON_SELFTEST does.
      if (process.env.BITVUE_ELECTRON_SELFTEST_FIXTURE_PATH) {
        return process.env.BITVUE_ELECTRON_SELFTEST_FIXTURE_PATH;
      }
      const win = BrowserWindow.fromWebContents(event.sender);
      const options: Electron.OpenDialogOptions = {
        properties: ["openFile"],
        filters: filters ?? [{ name: "All Files", extensions: ["*"] }],
      };
      const result = win
        ? await dialog.showOpenDialog(win, options)
        : await dialog.showOpenDialog(options);
      if (result.canceled || result.filePaths.length === 0) return null;
      return result.filePaths[0];
    },
  );

  // Quit menu item / TitleBar's Quit button / TitleBar's in-window close (X) button. Routed
  // through requestQuit() (TAURI_WEB-006) instead of calling app.quit() directly -- see its doc
  // for why (confirms first when a real bitstream is open) and for how this stays "quit the app"
  // cross-platform, not just "close the current window", same as before this fix.
  ipcMain.handle("bitvue:closeWindow", async () => {
    await requestQuit();
  });

  // TitleBar's minimize/maximize buttons (Windows/Linux only -- see frontend/utils/platform.ts's
  // shouldShowTitleBar -- macOS uses the native menu + native traffic-light window controls
  // instead). These called @tauri-apps/api/window's getCurrentWindow() directly pre-migration,
  // which doesn't exist under Electron and threw on every click -- a real, reachable bug (found
  // 2026-08-20 during the Tauri-leftover audit), not dead code: shouldShowTitleBar() is
  // unconditional on Windows/Linux and this component is mounted whenever it returns true.
  ipcMain.handle("bitvue:minimizeWindow", (event) => {
    BrowserWindow.fromWebContents(event.sender)?.minimize();
  });
  ipcMain.handle("bitvue:toggleMaximizeWindow", (event) => {
    const win = BrowserWindow.fromWebContents(event.sender);
    if (!win) return;
    if (win.isMaximized()) {
      win.unmaximize();
    } else {
      win.maximize();
    }
  });
}
