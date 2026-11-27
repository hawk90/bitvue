/**
 * BitrateGraphPanel with `frames` passed in (how App mounts it, from FrameDataContext) and its x
 * axis. The fetch path is covered by BitrateGraphPanel.test.tsx.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@/test/test-utils";
import { BitrateGraphPanel } from "../BitrateGraphPanel";
import {
  frameInfoToGraphFrame,
  type GraphFrame,
} from "@/services/electronBridgeService";
import type { FrameInfo } from "@/types/video";

const { getFramesChunk } = vi.hoisted(() => ({ getFramesChunk: vi.fn() }));

vi.mock("@/services/bridge/stream", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/services/bridge/stream")>()),
  getFramesChunk,
}));

const gf = (
  frame_index: number,
  frame_type: string,
  size: number,
  pts: number | null,
): GraphFrame => ({
  frame_index,
  frame_type,
  size,
  pts,
  ref_frames: null,
  temporal_id: null,
});

describe("BitrateGraphPanel with provided frames", () => {
  beforeEach(() => {
    getFramesChunk.mockReset();
  });

  it("draws the given frames without asking the sidecar for them", () => {
    const { container } = render(
      <BitrateGraphPanel
        frames={[gf(0, "I", 9000, 0), gf(1, "P", 3000, 1), gf(2, "P", 2000, 2)]}
      />,
    );

    expect(container.querySelectorAll(".bitrate-bar")).toHaveLength(3);
    expect(screen.getByText("Bitrate Graph")).toBeInTheDocument();
    expect(getFramesChunk).not.toHaveBeenCalled();
  });

  it("has no Reload button (the context owns the data)", () => {
    render(<BitrateGraphPanel frames={[gf(0, "I", 9000, 0)]} />);
    expect(screen.queryByText("Reload")).not.toBeInTheDocument();
  });

  it("shows the empty state for an empty list instead of fetching", () => {
    render(<BitrateGraphPanel frames={[]} />);
    expect(screen.getByText("No stream loaded")).toBeInTheDocument();
    expect(getFramesChunk).not.toHaveBeenCalled();
  });

  it("lays bars out in display order and says so", () => {
    // decode order I P B: the B frame (pts 1) is displayed before the P frame (pts 2)
    const { container } = render(
      <BitrateGraphPanel
        frames={[gf(0, "I", 100, 0), gf(1, "P", 100, 2), gf(2, "B", 100, 1)]}
      />,
    );

    const bars = Array.from(container.querySelectorAll(".bitrate-bar")).map(
      (el) => (el as HTMLElement).style.backgroundColor,
    );
    // I blue, B amber, P green -> I, B, P
    expect(bars).toEqual([
      "rgb(59, 130, 246)",
      "rgb(245, 158, 11)",
      "rgb(16, 185, 129)",
    ]);
    expect(screen.getByText("Display order (by PTS)")).toBeInTheDocument();
  });

  it("falls back to decode order when a frame has no PTS", () => {
    render(
      <BitrateGraphPanel
        frames={[gf(0, "I", 100, null), gf(1, "P", 100, null)]}
      />,
    );
    expect(
      screen.getByText("Decode order (PTS missing or repeated)"),
    ).toBeInTheDocument();
  });

  it("labels the x axis with frame numbers, or seconds when a frame rate is known", () => {
    const frames = Array.from({ length: 61 }, (_, i) => gf(i, "P", 100, i));
    const { container, rerender } = render(
      <BitrateGraphPanel frames={frames} />,
    );
    const ticks = () =>
      Array.from(container.querySelectorAll(".bitrate-axis-tick")).map(
        (el) => el.textContent,
      );
    expect(ticks()).toEqual(["#0", "#15", "#30", "#45", "#60"]);

    rerender(<BitrateGraphPanel frames={frames} frameRate={30} />);
    expect(ticks()).toEqual(["0.00s", "0.50s", "1.00s", "1.50s", "2.00s"]);
  });
});

describe("frameInfoToGraphFrame", () => {
  it("maps missing optional fields to null", () => {
    const info: FrameInfo = { frame_index: 4, frame_type: "B", size: 77 };
    expect(frameInfoToGraphFrame(info)).toEqual({
      frame_index: 4,
      frame_type: "B",
      size: 77,
      pts: null,
      ref_frames: null,
      temporal_id: null,
    });
  });

  it("keeps present fields, including a PTS of 0", () => {
    const info: FrameInfo = {
      frame_index: 0,
      frame_type: "I",
      size: 10,
      pts: 0,
      ref_frames: [1, 2],
      temporal_id: 1,
    };
    expect(frameInfoToGraphFrame(info)).toEqual({
      frame_index: 0,
      frame_type: "I",
      size: 10,
      pts: 0,
      ref_frames: [1, 2],
      temporal_id: 1,
    });
  });
});
