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

  // Cancellable pair for the filmstrip-scrub hot path (see `SidecarClient
  // .getDecodedFrameYuvCancellable`'s doc and `decode_session.rs`'s cancel_flag wiring). `invoke`
  // is strictly request/response, so a synchronous `{promise, cancel}` handle can't cross the IPC
  // boundary directly -- the renderer instead mints a `requestId` up front and this map lets the
  // second channel find the matching in-flight request's `cancel()` closure. Entries are always
  // removed in the `finally` below (settle OR cancel), so this never grows unbounded even if a
  // caller never explicitly cancels.
  const pending = new Map<string, () => void>();

  ipcMain.handle(
    "bitvue:getDecodedFrameYuvCancellable",
    async (
      _event,
      requestId: string,
      stream: string,
      frameIndex: number,
    ) => {
      const { promise, cancel } = requireSidecar().getDecodedFrameYuvCancellable({
        stream,
        frameIndex,
      });
      pending.set(requestId, cancel);
      try {
        const { bytes, ...metadata } = await promise;
        return { ...metadata, bytes };
      } finally {
        pending.delete(requestId);
      }
    },
  );

  ipcMain.handle(
    "bitvue:cancelDecodedFrameYuv",
    (_event, requestId: string) => {
      // No-op if the request already settled (or was never valid) -- the caller can't know
      // which case applies without awaiting the original promise, so this is deliberately
      // silent rather than surfacing a "not found" error for what's a normal race.
      pending.get(requestId)?.();
    },
  );
}
