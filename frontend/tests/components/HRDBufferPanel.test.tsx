/**
 * HRDBufferPanel tests.
 *
 * No real call site passes a signaled `targetBitrate` (AV1 decoder_model_info /
 * operating_parameters_info aren't parsed into anything the UI receives), so the panel must say
 * "no HRD information" rather than simulate from an average-bitrate guess (#461). With a real
 * `targetBitrate` it still plots the buffer: the simulation must not be flat (an earlier bug
 * drained exactly each frame's own size, so the curve never moved off 50%).
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

// Bitrate whose per-frame drain equals the sizes' average, so occupancy moves with size variance.
const avgBitrate = (sizes: number[], fps = 30) =>
  (sizes.reduce((a, b) => a + b, 0) / sizes.length) * fps * 8;

describe("HRDBufferPanel", () => {
  it("shows 'No HRD information' and draws nothing when no targetBitrate is signaled", () => {
    const { container } = render(
      <HRDBufferPanel
        frames={makeFrames([1000, 2000, 1500])}
        currentFrameIndex={0}
        frameRate={30}
      />,
    );

    expect(screen.getByText(/No HRD information/)).toBeInTheDocument();
    expect(container.querySelector("canvas")).toBeNull();
    expect(screen.queryByText(/Buffer:/)).not.toBeInTheDocument();
    expect(screen.queryByText(/Mbps/)).not.toBeInTheDocument();
  });

  it("shows the same message for an empty frame list", () => {
    render(<HRDBufferPanel frames={[]} currentFrameIndex={0} frameRate={30} />);
    expect(screen.getByText(/No HRD information/)).toBeInTheDocument();
  });

  it("labels the bitrate stat as Target when targetBitrate is provided", () => {
    render(
      <HRDBufferPanel
        frames={makeFrames([1000, 2000, 1500])}
        currentFrameIndex={0}
        frameRate={30}
        targetBitrate={5_000_000}
      />,
    );

    expect(screen.getByText("Target:")).toBeInTheDocument();
    expect(screen.getByText("5.00 Mbps")).toBeInTheDocument();
    expect(screen.queryByText(/No HRD information/)).not.toBeInTheDocument();
  });

  it("produces a non-flat buffer occupancy when frame sizes vary", () => {
    const sizes = [100, 100, 100, 50_000, 100, 100, 100];
    const { container } = render(
      <HRDBufferPanel
        frames={makeFrames(sizes)}
        currentFrameIndex={3}
        frameRate={30}
        targetBitrate={avgBitrate(sizes)}
      />,
    );

    const bufferStat = container.querySelector(".hrd-stat .stat-value");
    expect(bufferStat).toBeInTheDocument();
    expect(bufferStat!.textContent).not.toBe("50.0%");
  });

  it("tracks the currently viewed frame's occupancy, not always the stream's last frame", () => {
    const sizes = [100, 100, 100, 50_000, 100, 100, 100];
    const frames = makeFrames(sizes);
    const targetBitrate = avgBitrate(sizes);

    const { container: atSpike } = render(
      <HRDBufferPanel
        frames={frames}
        currentFrameIndex={3}
        frameRate={30}
        targetBitrate={targetBitrate}
      />,
    );
    const { container: atEnd } = render(
      <HRDBufferPanel
        frames={frames}
        currentFrameIndex={6}
        frameRate={30}
        targetBitrate={targetBitrate}
      />,
    );

    const spikeStat = atSpike.querySelector(
      ".hrd-stat .stat-value",
    )!.textContent;
    const endStat = atEnd.querySelector(".hrd-stat .stat-value")!.textContent;
    expect(spikeStat).not.toBe(endStat);
  });

  it("stays at the 50% starting point only when every frame equals the drain", () => {
    const sizes = [1000, 1000, 1000, 1000];
    const { container } = render(
      <HRDBufferPanel
        frames={makeFrames(sizes)}
        currentFrameIndex={0}
        frameRate={30}
        targetBitrate={avgBitrate(sizes)}
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
          targetBitrate={1_000_000}
        />,
      ),
    ).not.toThrow();
  });
});
