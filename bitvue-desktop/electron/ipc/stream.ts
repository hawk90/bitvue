/**
 * Stream domain IPC handlers — connection handshake, open/close/select a stream, metadata
 * indexing, the display-order timeline, and filmstrip thumbnails. Mirrors
 * `frontend/services/bridge/stream.ts` channel-for-channel (2026-08-20 axis-1 split — see
 * `main.ts`'s `registerIpcHandlers` doc for why this got split out).
 */

import { ipcMain } from "electron";
import type { SidecarClient } from "../../src/sidecarClient.js";

export function registerStreamIpcHandlers(
  requireSidecar: () => SidecarClient,
): void {
  ipcMain.handle("bitvue:hello", async (_event, clientVersion?: string) => {
    return requireSidecar().hello(clientVersion);
  });

  ipcMain.handle(
    "bitvue:openStream",
    async (_event, stream: string, filePath: string) => {
      return requireSidecar().request("open_stream", {
        stream,
        path: filePath,
      });
    },
  );

  ipcMain.handle("bitvue:closeStream", async (_event, stream: string) => {
    return requireSidecar().request("close_stream", { stream });
  });

  ipcMain.handle(
    "bitvue:selectFrame",
    async (_event, stream: string, frameIndex: number) => {
      return requireSidecar().request("select_frame", {
        stream,
        frame_index: frameIndex,
      });
    },
  );

  ipcMain.handle("bitvue:indexStream", async (_event, stream: string) => {
    return requireSidecar().request("index_stream", { stream });
  });

  ipcMain.handle("bitvue:getStreamInfo", async (_event, stream: string) => {
    return requireSidecar().request("get_stream_info", { stream });
  });

  ipcMain.handle(
    "bitvue:getFramesChunk",
    async (_event, stream: string, offset: number, limit: number) => {
      return requireSidecar().request("get_frames_chunk", {
        stream,
        offset,
        limit,
      });
    },
  );

  ipcMain.handle("bitvue:getTimeline", async (_event, stream: string) => {
    return requireSidecar().request("get_timeline", { stream });
  });

  ipcMain.handle(
    "bitvue:getThumbnails",
    async (
      _event,
      stream: string,
      frameIndices: number[],
      targetWidth?: number,
    ) => {
      return requireSidecar().request("get_thumbnails", {
        stream,
        frame_indices: frameIndices,
        target_width: targetWidth,
      });
    },
  );
}
