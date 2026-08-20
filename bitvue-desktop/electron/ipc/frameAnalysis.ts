/**
 * Frame analysis domain IPC handlers — per-frame QP/MV/partition grids and codec-feature
 * breakdowns. Mirrors `frontend/services/bridge/frameAnalysis.ts` channel-for-channel
 * (2026-08-20 axis-1 split — see `main.ts`'s `registerIpcHandlers` doc for why this got split
 * out).
 */

import { ipcMain } from "electron";
import type { SidecarClient } from "../../src/sidecarClient.js";

export function registerFrameAnalysisIpcHandlers(
  requireSidecar: () => SidecarClient,
): void {
  // Two-frame command (see sidecarClient.ts's getFrameAnalysis doc) -- qp_bytes is raw
  // little-endian i16 per value, merged alongside the JSON metadata rather than reassembled into
  // a `qp` array here so the renderer's structured-clone IPC transfer stays a real Buffer, not a
  // JSON array of numbers (that reassembly happens at frontend/services/bridge/frameAnalysis.ts).
  ipcMain.handle(
    "bitvue:getFrameAnalysis",
    async (_event, frameIndex: number) => {
      const { metadata, qpBytes } = await requireSidecar().getFrameAnalysis({
        frameIndex,
      });
      return { ...metadata, qp_bytes: qpBytes };
    },
  );

  ipcMain.handle(
    "bitvue:getAv1Features",
    async (_event, frameIndex: number) => {
      return requireSidecar().request("get_av1_features", {
        frame_index: frameIndex,
      });
    },
  );

  ipcMain.handle(
    "bitvue:getCodingFlowAnalysis",
    async (_event, frameIndex: number) => {
      return requireSidecar().request("get_coding_flow_analysis", {
        frame_index: frameIndex,
      });
    },
  );

  ipcMain.handle(
    "bitvue:getDeblockingAnalysis",
    async (_event, frameIndex: number) => {
      return requireSidecar().request("get_deblocking_analysis", {
        frame_index: frameIndex,
      });
    },
  );

  ipcMain.handle(
    "bitvue:getCodecExtendedInfo",
    async (_event, frameIndex: number) => {
      return requireSidecar().request("get_codec_extended_info", {
        frame_index: frameIndex,
      });
    },
  );

  ipcMain.handle(
    "bitvue:getResidualAnalysis",
    async (_event, frameIndex: number) => {
      return requireSidecar().request("get_residual_analysis", {
        frame_index: frameIndex,
      });
    },
  );
}
