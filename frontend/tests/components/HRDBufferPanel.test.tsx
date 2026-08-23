/**
 * HRDBufferPanel tests.
 *
 * Covers a real bug found via a UI/UX parity screenshot comparison against VQ Analyzer's
 * equivalent Buffer(HRD) view: with no `targetBitrate` prop (true for every real call site --
 * `Filmstrip.tsx` never passes one, since real AV1 HRD/decoder_model_info parameters aren't
 * parsed anywhere in this codebase yet), the old fallback drained exactly one frame's own bytes
 * per frame interval -- `occupancy - frameSize + frameSize` is always the starting value, so the
 * buffer curve was *always* perfectly flat (confirmed via a real Electron screenshot: exactly
 * 50.0%/488KB across an entire 250-frame stream) regardless of actual frame-size variance.
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@/test/test-utils";
import { HRDBufferPanel } from "../../components/panels/HRDBufferPanel";
import type { FrameInfo } from "../../types/video";

function makeFrames(sizes: number[]): FrameInfo[] {
  return sizes.map(
    (size, i) =>
      ({
        frame_index: i,
        frame_type: i === 0 ? "I" : "P",
        size,
      }) as FrameInfo,
  );
}

describe("HRDBufferPanel", () => {
  it("labels the bitrate stat as an honest estimate when no real targetBitrate is provided", () => {
    render(
      <HRDBufferPanel
        frames={makeFrames([1000, 2000, 1500])}
        currentFrameIndex={0}
        frameRate={30}
      />,
    );

    expect(screen.getByText("Avg bitrate (est.):")).toBeInTheDocument();
    expect(screen.queryByText("Target:")).not.toBeInTheDocument();
  });

  it("labels the bitrate stat as a real Target when targetBitrate is explicitly provided", () => {
    render(
      <HRDBufferPanel
        frames={makeFrames([1000, 2000, 1500])}
        currentFrameIndex={0}
        frameRate={30}
        targetBitrate={5_000_000}
      />,
    );

    expect(screen.getByText("Target:")).toBeInTheDocument();
    expect(screen.queryByText("Avg bitrate (est.):")).not.toBeInTheDocument();
    expect(screen.getByText("5.00 Mbps")).toBeInTheDocument();
  });

  it("does not render a bitrate stat at all when there are no frames to estimate from", () => {
    render(<HRDBufferPanel frames={[]} currentFrameIndex={0} frameRate={30} />);

    expect(screen.queryByText("Avg bitrate (est.):")).not.toBeInTheDocument();
    expect(screen.queryByText("Target:")).not.toBeInTheDocument();
  });

  it("produces a non-flat buffer occupancy when frame sizes vary (the real bug)", () => {
    // Highly bursty sizes around a fixed average-derived drain rate -- under the old buggy
    // fallback (drain == that same frame's own size every time), every single frame's step was a
    // mathematical no-op (occupancy - frameSize + frameSize == occupancy unchanged), so the
    // buffer never moved off its 50% starting point for ANY frame, ever. Reading frame 3 (right
    // after the 50KB spike, before the average-based drain has had time to bring it back down)
    // must show real buildup.
    const { container } = render(
      <HRDBufferPanel
        frames={makeFrames([100, 100, 100, 50_000, 100, 100, 100])}
        currentFrameIndex={3}
        frameRate={30}
      />,
    );

    const bufferStat = container.querySelector(".hrd-stat .stat-value");
    expect(bufferStat).toBeInTheDocument();
    expect(bufferStat!.textContent).not.toBe("50.0%");
  });

  it("tracks the currently viewed frame's occupancy, not always the stream's last frame", () => {
    const frames = makeFrames([100, 100, 100, 50_000, 100, 100, 100]);

    const { container: atSpike } = render(
      <HRDBufferPanel frames={frames} currentFrameIndex={3} frameRate={30} />,
    );
    const { container: atEnd } = render(
      <HRDBufferPanel frames={frames} currentFrameIndex={6} frameRate={30} />,
    );

    const spikeStat = atSpike.querySelector(
      ".hrd-stat .stat-value",
    )!.textContent;
    const endStat = atEnd.querySelector(".hrd-stat .stat-value")!.textContent;
    expect(spikeStat).not.toBe(endStat);
  });

  it("stays at the 50% starting point only in the degenerate all-frames-equal-size case", () => {
    // Sanity check on the test methodology above: uniform sizes really should net to no change,
    // confirming the *previous* test's divergence is caused by real size variance, not noise.
    const { container } = render(
      <HRDBufferPanel
        frames={makeFrames([1000, 1000, 1000, 1000])}
        currentFrameIndex={0}
        frameRate={30}
      />,
    );

    const bufferStat = container.querySelector(".hrd-stat .stat-value");
    expect(bufferStat!.textContent).toBe("50.0%");
  });

  it("renders without crashing when hovering isn't triggered (canvas absent in jsdom)", () => {
    expect(() =>
      render(
        <HRDBufferPanel
          frames={makeFrames([1000, 2000])}
          currentFrameIndex={0}
          frameRate={30}
        />,
      ),
    ).not.toThrow();
  });
});
