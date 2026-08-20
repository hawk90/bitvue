/**
 * Syntax/hex domain IPC handlers — raw byte ranges, lazy per-unit syntax trees, and the four
 * structural (multi-sync) selection commands. Mirrors `frontend/services/bridge/syntaxHex.ts`
 * channel-for-channel (2026-08-20 axis-1 split — see `main.ts`'s `registerIpcHandlers` doc for
 * why this got split out).
 */

import { ipcMain } from "electron";
import type { SidecarClient } from "../../src/sidecarClient.js";

export function registerSyntaxHexIpcHandlers(
  requireSidecar: () => SidecarClient,
): void {
  ipcMain.handle(
    "bitvue:getHexRange",
    async (_event, stream: string, offset: number, len: number) => {
      const { bytes, ...metadata } = await requireSidecar().getHexRange({
        stream,
        offset,
        len,
      });
      // Same raw-Buffer-over-structured-clone approach as getDecodedFrameYuv -- no base64.
      return { ...metadata, bytes };
    },
  );

  ipcMain.handle(
    "bitvue:getFrameSyntax",
    async (_event, stream: string, frameIndex: number) => {
      return requireSidecar().request("get_frame_syntax", {
        stream,
        frame_index: frameIndex,
      });
    },
  );

  ipcMain.handle(
    "bitvue:selectUnit",
    async (
      _event,
      stream: string,
      unitType: string,
      offset: number,
      size: number,
    ) => {
      return requireSidecar().request("select_unit", {
        stream,
        unit_type: unitType,
        offset,
        size,
      });
    },
  );

  ipcMain.handle(
    "bitvue:selectSyntax",
    async (
      _event,
      stream: string,
      nodeId: string,
      startBit: number,
      endBit: number,
    ) => {
      return requireSidecar().request("select_syntax", {
        stream,
        node_id: nodeId,
        start_bit: startBit,
        end_bit: endBit,
      });
    },
  );

  ipcMain.handle(
    "bitvue:selectBitRange",
    async (_event, stream: string, startBit: number, endBit: number) => {
      return requireSidecar().request("select_bit_range", {
        stream,
        start_bit: startBit,
        end_bit: endBit,
      });
    },
  );

  ipcMain.handle(
    "bitvue:selectSpatialBlock",
    async (
      _event,
      stream: string,
      x: number,
      y: number,
      w: number,
      h: number,
    ) => {
      return requireSidecar().request("select_spatial_block", {
        stream,
        x,
        y,
        w,
        h,
      });
    },
  );
}
