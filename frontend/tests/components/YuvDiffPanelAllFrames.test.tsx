/**
 * YuvDiffPanel's whole-stream row (#606): the "All Frames" button, the averages it shows, and the
 * jump to the lowest-PSNR frame.
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@/test/test-utils";
import { act } from "@testing-library/react";

const bridge = vi.hoisted(() => ({
  loadDebugYuv: vi.fn(),
  unloadDebugYuv: vi.fn(),
  setDebugYuvOffset: vi.fn(),
  setDebugYuvCrop: vi.fn(),
  getYuvDiffMetrics: vi.fn(),
  getYuvDiffMetricsAll: vi.fn(),
  findFirstDiffFrame: vi.fn(),
}));

vi.mock("@/services/electronBridgeService", () => ({
  ...bridge,
}));

import { YuvDiffProvider, useYuvDiff } from "@/contexts/YuvDiffContext";
import { YuvDiffPanel } from "@/components/panels/YuvDiffPanel";

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

let loadRef: () => Promise<void>;
function Loader() {
  const { loadFile } = useYuvDiff();
  loadRef = () =>
    loadFile({
      path: "/tmp/ref.yuv",
      width: 16,
      height: 16,
      format: "i420",
      bitdepth: 8,
    });
  return null;
}

async function renderLoaded(onJumpToFrame = vi.fn()) {
  render(
    <YuvDiffProvider>
      <Loader />
      <YuvDiffPanel currentFrameIndex={0} onJumpToFrame={onJumpToFrame} />
    </YuvDiffProvider>,
  );
  await act(async () => {
    await loadRef();
  });
  return onJumpToFrame;
}

describe("YuvDiffPanel whole-stream metrics", () => {
  beforeEach(() => {
    Object.values(bridge).forEach((m) => m.mockReset());
    bridge.loadDebugYuv.mockResolvedValue({ success: true, frame_count: 5 });
    bridge.unloadDebugYuv.mockResolvedValue(undefined);
    bridge.setDebugYuvOffset.mockResolvedValue(undefined);
  });

  it("shows the average PSNR/SSIM and jumps to the lowest-PSNR frame", async () => {
    bridge.getYuvDiffMetricsAll.mockResolvedValueOnce({
      frame_count: 3,
      average_psnr: 39.5,
      average_ssim: 0.9,
      frames: [frame(0, 42), frame(1, 33.25), frame(2, 43)],
    });
    const onJump = await renderLoaded();

    expect(screen.getByText(/press All Frames to compute/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "All Frames" }));

    await waitFor(() =>
      expect(screen.getByText("39.50 dB")).toBeInTheDocument(),
    );
    expect(screen.getByText("0.9000")).toBeInTheDocument();
    const worst = screen.getByRole("button", {
      name: /Worst: #1 \(33\.25 dB\)/,
    });
    fireEvent.click(worst);
    expect(onJump).toHaveBeenCalledWith(1);
  });

  it("says so when no frame overlapped the reference, instead of showing averages", async () => {
    bridge.getYuvDiffMetricsAll.mockResolvedValueOnce({
      frame_count: 0,
      average_psnr: null,
      average_ssim: null,
      frames: [],
    });
    await renderLoaded();
    fireEvent.click(screen.getByRole("button", { name: "All Frames" }));
    await waitFor(() =>
      expect(
        screen.getByText(/no frame overlapped the reference/),
      ).toBeInTheDocument(),
    );
    expect(screen.queryByText(/Worst:/)).not.toBeInTheDocument();
  });
});
