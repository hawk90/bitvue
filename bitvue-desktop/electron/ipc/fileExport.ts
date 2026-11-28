/**
 * `bitvue:saveTextFile`: save text the renderer produced (CSV/JSON/report) through a native save
 * dialog the main process shows itself. See `../saveTextFile.ts` for why the renderer sends no path.
 */

import { BrowserWindow, ipcMain, dialog } from "electron";
import { writeFile } from "node:fs/promises";
import { saveTextFile } from "../saveTextFile.js";

export function registerFileExportIpcHandlers(): void {
  ipcMain.handle("bitvue:saveTextFile", async (event, request: unknown) => {
    const win = BrowserWindow.fromWebContents(event.sender);
    return saveTextFile(request, {
      chooseSavePath: async ({ defaultName, filters }) => {
        // Test-only bypass: a native dialog cannot be scripted in an offscreen run. Set only by
        // the self-test / screenshot tooling, never by the renderer.
        if (process.env.BITVUE_ELECTRON_SELFTEST_SAVE_PATH) {
          return process.env.BITVUE_ELECTRON_SELFTEST_SAVE_PATH;
        }
        const options: Electron.SaveDialogOptions = {
          defaultPath: defaultName,
          filters,
        };
        const result = win
          ? await dialog.showSaveDialog(win, options)
          : await dialog.showSaveDialog(options);
        return result.canceled || !result.filePath ? null : result.filePath;
      },
      writeFile: (filePath, content) => writeFile(filePath, content, "utf8"),
    });
  });
}
