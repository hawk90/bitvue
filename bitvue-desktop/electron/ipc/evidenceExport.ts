/**
 * Evidence/export domain IPC handlers — diagnostic evidence bundle export and the context-menu
 * guard catalog. Mirrors `frontend/services/bridge/evidenceExport.ts` channel-for-channel
 * (2026-08-20 axis-1 split — see `main.ts`'s `registerIpcHandlers` doc for why this got split
 * out).
 */

import { BrowserWindow, ipcMain, dialog } from "electron";
import path from "node:path";
import type { SidecarClient } from "../../src/sidecarClient.js";

export function registerEvidenceExportIpcHandlers(
  requireSidecar: () => SidecarClient,
): void {
  ipcMain.handle(
    "bitvue:getContextMenuItems",
    async (
      _event,
      scope: string,
      hasSelection: boolean,
      hasByteRange: boolean,
    ) => {
      return requireSidecar().request("get_context_menu_items", {
        scope,
        has_selection: hasSelection,
        has_byte_range: hasByteRange,
      });
    },
  );

  ipcMain.handle(
    "bitvue:exportEvidenceBundle",
    async (
      _event,
      params: {
        outputDir: string;
        workspace?: string;
        mode?: string;
        orderType?: string;
        screenshotDataUrl?: string;
      },
    ) => {
      return requireSidecar().request("export_evidence_bundle", {
        output_dir: params.outputDir,
        workspace: params.workspace,
        mode: params.mode,
        order_type: params.orderType,
        screenshot_data_url: params.screenshotDataUrl,
      });
    },
  );

  ipcMain.handle("bitvue:showDirectoryDialog", async (event) => {
    // Test-only bypass: a real native OS dialog can't be scripted in an automated/offscreen run.
    // Reused here as an arbitrary writable directory (a temp dir would need its own env var; the
    // fixture path's parent directory is always writable in the test environment this runs in).
    if (process.env.BITVUE_ELECTRON_SELFTEST_FIXTURE_PATH) {
      return path.dirname(process.env.BITVUE_ELECTRON_SELFTEST_FIXTURE_PATH);
    }
    const win = BrowserWindow.fromWebContents(event.sender);
    const options: Electron.OpenDialogOptions = {
      properties: ["openDirectory", "createDirectory"],
    };
    const result = win
      ? await dialog.showOpenDialog(win, options)
      : await dialog.showOpenDialog(options);
    if (result.canceled || result.filePaths.length === 0) return null;
    return result.filePaths[0];
  });

  // Captures the current window as a PNG, base64-encoded as a data: URL -- the same
  // `capturePage()`/`toPNG()` pair `runScreenshotAndExit` already uses for the
  // BITVUE_ELECTRON_SCREENSHOT dev/CI mode, exposed here as an on-demand handler for real
  // in-app features (currently: Evidence Bundle export's `screenshots/` artifact). Returns null
  // if there's no window to capture from (shouldn't happen in practice -- the renderer calling
  // this is itself running inside a window).
  ipcMain.handle("bitvue:captureScreenshot", async (event) => {
    const win = BrowserWindow.fromWebContents(event.sender);
    if (!win) return null;
    const image = await win.webContents.capturePage();
    return `data:image/png;base64,${image.toPNG().toString("base64")}`;
  });
}
