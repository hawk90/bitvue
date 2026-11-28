/**
 * YuvDiffContext: whole-stream PSNR/SSIM (#606) -- computed in one bridge call, dropped when the
 * reference, offset or crop changes, and a failed run is reported instead of swallowed.
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import type { ReactNode } from "react";

const bridge = vi.hoisted(() => ({
  loadDebugYuv: vi.fn(),
  unloadDebugYuv: vi.fn(),
  setDebugYuvOffset: vi.fn(),
  getYuvDiffMetrics: vi.fn(),
  getYuvDiffMetricsAll: vi.fn(),
  findFirstDiffFrame: vi.fn(),
}));

vi.mock("@/services/electronBridgeService", () => ({
  loadDebugYuv: bridge.loadDebugYuv,
  unloadDebugYuv: bridge.unloadDebugYuv,
  setDebugYuvOffset: bridge.setDebugYuvOffset,
  getYuvDiffMetrics: bridge.getYuvDiffMetrics,
  getYuvDiffMetricsAll: bridge.getYuvDiffMetricsAll,
  findFirstDiffFrame: bridge.findFirstDiffFrame,
}));

import { YuvDiffProvider, useYuvDiff } from "@/contexts/YuvDiffContext";

const wrapper = ({ children }: { children: ReactNode }) => (
  <YuvDiffProvider>{children}</YuvDiffProvider>
);

const frame = (i: number, psnr: number) => ({
  frame_index: i,
  psnr_y: psnr,
  psnr_u: psnr,
  psnr_v: psnr,
  psnr_avg: psnr,
  ssim_y: 0.9,
  max_diff_y: 3,
  has_mismatch: true,
});

const ALL = {
  frame_count: 2,
  average_psnr: 39,
  average_ssim: 0.9,
  frames: [frame(0, 40), frame(1, 38)],
};

async function loaded() {
  const hook = renderHook(() => useYuvDiff(), { wrapper });
  await act(async () => {
    await hook.result.current.loadFile({
      path: "/tmp/ref.yuv",
      width: 16,
      height: 16,
      format: "i420",
      bitdepth: 8,
    });
  });
  return hook;
}

describe("YuvDiffContext whole-stream metrics", () => {
  beforeEach(() => {
    Object.values(bridge).forEach((m) => m.mockReset());
    bridge.loadDebugYuv.mockResolvedValue({ success: true, frame_count: 5 });
    bridge.unloadDebugYuv.mockResolvedValue(undefined);
    bridge.setDebugYuvOffset.mockResolvedValue(undefined);
  });

  it("does nothing until a reference is loaded", async () => {
    const hook = renderHook(() => useYuvDiff(), { wrapper });
    await act(async () => {
      await hook.result.current.computeStreamMetrics();
    });
    expect(bridge.getYuvDiffMetricsAll).not.toHaveBeenCalled();
    expect(hook.result.current.streamMetrics).toBeNull();
  });

  it("stores the one-call result", async () => {
    bridge.getYuvDiffMetricsAll.mockResolvedValueOnce(ALL);
    const hook = await loaded();
    await act(async () => {
      await hook.result.current.computeStreamMetrics();
    });
    expect(bridge.getYuvDiffMetricsAll).toHaveBeenCalledTimes(1);
    expect(hook.result.current.streamMetrics).toEqual(ALL);
    expect(hook.result.current.loading).toBe(false);
  });

  it("drops the result when the offset or the crop changes, since both change what was compared", async () => {
    bridge.getYuvDiffMetricsAll.mockResolvedValue(ALL);
    const hook = await loaded();
    await act(async () => {
      await hook.result.current.computeStreamMetrics();
    });
    act(() => hook.result.current.setPictureOffset(1));
    expect(hook.result.current.streamMetrics).toBeNull();

    await act(async () => {
      await hook.result.current.computeStreamMetrics();
    });
    expect(hook.result.current.streamMetrics).not.toBeNull();
    act(() => hook.result.current.invalidateStreamMetrics());
    expect(hook.result.current.streamMetrics).toBeNull();
  });

  it("drops the result when another reference is loaded", async () => {
    bridge.getYuvDiffMetricsAll.mockResolvedValue(ALL);
    const hook = await loaded();
    await act(async () => {
      await hook.result.current.computeStreamMetrics();
    });
    await act(async () => {
      await hook.result.current.loadFile({
        path: "/tmp/other.yuv",
        width: 16,
        height: 16,
        format: "i420",
        bitdepth: 8,
      });
    });
    expect(hook.result.current.streamMetrics).toBeNull();
  });

  it("reports a failed run as an error and keeps no stale result", async () => {
    bridge.getYuvDiffMetricsAll.mockRejectedValueOnce(
      new Error(
        "decoded frame 0 is 8x8 but reference frame (after crop) is 16x16",
      ),
    );
    const hook = await loaded();
    await act(async () => {
      await hook.result.current.computeStreamMetrics();
    });
    expect(hook.result.current.streamMetrics).toBeNull();
    expect(hook.result.current.error).toContain("8x8");
    expect(hook.result.current.loading).toBe(false);
  });
});
