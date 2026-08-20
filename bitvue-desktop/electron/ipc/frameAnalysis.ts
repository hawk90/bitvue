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
  ipcMain.handle(
    "bitvue:getFrameAnalysis",
    async (_event, frameIndex: number) => {
      return requireSidecar().request("get_frame_analysis", {
        frame_index: frameIndex,
      });
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
