/**
 * Evidence/export domain IPC handlers — diagnostic evidence bundle export and the context-menu
 * guard catalog. Mirrors `frontend/services/bridge/evidenceExport.ts` channel-for-channel
 * (2026-08-20 axis-1 split — see `main.ts`'s `registerIpcHandlers` doc for why this got split
 * out).
 */

import { BrowserWindow, ipcMain, dialog } from "electron";
import type { SidecarClient } from "../../src/sidecarClient.js";
import { exportEvidenceBundleViaDialog } from "../evidenceBundle.js";

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

  // The renderer sends manifest fields and a screenshot, never an output directory: the folder
  // comes from a dialog the main process shows itself (see `../evidenceBundle.ts`, #640).
  ipcMain.handle(
    "bitvue:exportEvidenceBundle",
    async (event, request: unknown) => {
      const win = BrowserWindow.fromWebContents(event.sender);
      return exportEvidenceBundleViaDialog(request, {
        chooseDirectory: async () => {
          // Test-only bypass: a native dialog cannot be scripted in an offscreen run. Set only by
          // the self-test tooling in the main process, never by the renderer.
          if (process.env.BITVUE_ELECTRON_SELFTEST_EVIDENCE_DIR) {
            return process.env.BITVUE_ELECTRON_SELFTEST_EVIDENCE_DIR;
          }
          const options: Electron.OpenDialogOptions = {
            properties: ["openDirectory", "createDirectory"],
          };
          const result = win
            ? await dialog.showOpenDialog(win, options)
            : await dialog.showOpenDialog(options);
          if (result.canceled || result.filePaths.length === 0) return null;
          return result.filePaths[0];
        },
        exportBundle: (params) =>
          requireSidecar().request("export_evidence_bundle", params),
      });
    },
  );

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
