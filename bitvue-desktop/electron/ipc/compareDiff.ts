/**
 * Compare/diff domain IPC handlers — the single-stream debug-YUV reference-file workflow and the
 * dual-stream A/B compare workspace (docs/DEVELOPMENT_PHASES.md Phase 7.5). Mirrors
 * `frontend/services/bridge/compareDiff.ts` channel-for-channel (2026-08-20 axis-1 split — see
 * `main.ts`'s `registerIpcHandlers` doc for why this got split out).
 */

import { ipcMain } from "electron";
import type { SidecarClient } from "../../src/sidecarClient.js";

export function registerCompareDiffIpcHandlers(
  requireSidecar: () => SidecarClient,
): void {
  ipcMain.handle(
    "bitvue:loadDebugYuv",
    async (
      _event,
      params: {
        path: string;
        width: number;
        height: number;
        format: string;
        bitdepth: number;
        picture_offset?: number;
        crop?: { left: number; right: number; top: number; bottom: number };
      },
    ) => {
      return requireSidecar().request("load_debug_yuv", params);
    },
  );

  ipcMain.handle("bitvue:unloadDebugYuv", async () => {
    return requireSidecar().request("unload_debug_yuv");
  });

  ipcMain.handle("bitvue:setDebugYuvOffset", async (_event, offset: number) => {
    return requireSidecar().request("set_debug_yuv_offset", { offset });
  });

  ipcMain.handle(
    "bitvue:setDebugYuvCrop",
    async (
      _event,
      crop: { left: number; right: number; top: number; bottom: number },
    ) => {
      return requireSidecar().request("set_debug_yuv_crop", { crop });
    },
  );

  ipcMain.handle(
    "bitvue:getYuvDiffMetrics",
    async (_event, frameIndex: number) => {
      return requireSidecar().request("get_yuv_diff_metrics", {
        frame_index: frameIndex,
      });
    },
  );

  ipcMain.handle("bitvue:findFirstDiffFrame", async () => {
    return requireSidecar().request("find_first_diff_frame");
  });

  ipcMain.handle(
    "bitvue:getDebugYuvFrame",
    async (_event, frameIndex: number, mode: string, amplify?: number) => {
      const { bytes, ...metadata } = await requireSidecar().getDebugYuvFrame({
        frameIndex,
        mode: mode as "decoded" | "reference" | "diff" | "amplified",
        amplify,
      });
      // Same raw-Buffer-over-structured-clone approach as getDecodedFrameYuv.
      return { ...metadata, bytes };
    },
  );

  // Dual-stream compare workspace (docs/DEVELOPMENT_PHASES.md Phase 7.5) -- PTS-based A/B
  // alignment + real diff/heatmap overlay. All plain JSON control-frame commands (no raw-Buffer
  // payload), unlike getDecodedFrameYuv/getHexRange/getDebugYuvFrame above.
  ipcMain.handle("bitvue:createCompareWorkspace", async () => {
    return requireSidecar().request("create_compare_workspace");
  });

  ipcMain.handle(
    "bitvue:getAlignedFrame",
    async (_event, streamAFrameIdx: number) => {
      return requireSidecar().request("get_aligned_frame", {
        stream_a_frame_idx: streamAFrameIdx,
      });
    },
  );

  ipcMain.handle(
    "bitvue:setSyncMode",
    async (_event, mode: "Off" | "Playhead" | "Full") => {
      return requireSidecar().request("set_sync_mode", { mode });
    },
  );

  ipcMain.handle(
    "bitvue:setManualOffset",
    async (_event, offset: number) => {
      return requireSidecar().request("set_manual_offset", { offset });
    },
  );

  ipcMain.handle("bitvue:resetOffset", async () => {
    return requireSidecar().request("reset_offset");
  });

  ipcMain.handle(
    "bitvue:getDiffFrame",
    async (_event, streamAFrameIdx: number, mode: "abs" | "signed") => {
      return requireSidecar().request("get_diff_frame", {
        stream_a_frame_idx: streamAFrameIdx,
        mode,
      });
    },
  );

  // PARITY_CHECKLIST.md CMP-04 -- distinct wire method name from bitvue:findFirstDiffFrame
  // above (that one is the debug-YUV single-stream-vs-reference-file command).
  ipcMain.handle("bitvue:findFirstDiffFrameAb", async () => {
    return requireSidecar().request("find_first_diff_frame_ab");
  });
}
