/**
 * Frame decode domain IPC handler — real decoded YUV pixel planes for one frame. Mirrors
 * `frontend/services/bridge/frameDecode.ts` (2026-08-20 axis-1 split — see `main.ts`'s
 * `registerIpcHandlers` doc for why this got split out).
 */

import { ipcMain } from "electron";
import type { SidecarClient } from "../../src/sidecarClient.js";

export function registerFrameDecodeIpcHandlers(
  requireSidecar: () => SidecarClient,
): void {
  ipcMain.handle(
    "bitvue:getDecodedFrameYuv",
    async (_event, stream: string, frameIndex: number) => {
      const { bytes, ...metadata } = await requireSidecar().getDecodedFrameYuv({
        stream,
        frameIndex,
      });
      // Buffer survives Electron's structured-clone IPC as-is (renderer sees a Uint8Array) --
      // no base64/JSON-array encoding. This is the whole point of the migration; see the
      // `YUVFrameData` anti-pattern note in DEVELOPMENT_PHASES.md.
      return { ...metadata, bytes };
    },
  );
}
