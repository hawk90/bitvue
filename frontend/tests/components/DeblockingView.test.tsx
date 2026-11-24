/**
 * Tests for DeblockingView component
 */

import { describe, it, expect } from "vitest";
import { renderAct, screen } from "@/test/test-utils";
import { DeblockingView } from "../DeblockingView";
import type { FrameInfo } from "@/types/video";
import { muteBridgeUnavailableLogs } from "@/test/muteBridgeUnavailable";

// Use small dimensions (16x16) to keep SVG boundary count manageable (~6 elements)
// instead of 1920x1080 which generates ~65,000 elements per test.
const TEST_WIDTH = 16;
const TEST_HEIGHT = 16;

muteBridgeUnavailableLogs();

describe("DeblockingView", () => {
  const mockFrame: FrameInfo = {
    frame_index: 100,
    frame_type: "P",
    poc: 100,
    pts: 100,
    size: 25000,
    temporal_id: 0,
    spatial_id: 0,
    ref_frames: [99],
  };

  it("renders without crashing", async () => {
    await renderAct(
      <DeblockingView
        frame={mockFrame}
        width={TEST_WIDTH}
        height={TEST_HEIGHT}
        codec="hevc"
      />,
    );
    expect(screen.getByText("Loop Filter")).toBeInTheDocument();
  });

  it("displays frame information", async () => {
    await renderAct(
      <DeblockingView
        frame={mockFrame}
        width={TEST_WIDTH}
        height={TEST_HEIGHT}
        codec="hevc"
      />,
    );

    expect(screen.getByText("Frame 100")).toBeInTheDocument();
    expect(screen.getByText("P")).toBeInTheDocument();
    expect(screen.getByText("hevc")).toBeInTheDocument();
  });

  it("displays deblocking statistics", async () => {
    await renderAct(
      <DeblockingView
        frame={mockFrame}
        width={TEST_WIDTH}
        height={TEST_HEIGHT}
        codec="hevc"
      />,
    );

    // counts are in 4-sample edge segments, split by the largest filter an edge allows
    expect(screen.getByText(/Edge segments:/)).toBeInTheDocument();
    expect(screen.getByText(/^Filtered:$/)).toBeInTheDocument();
    expect(screen.getByText(/Up to 16 samples:/)).toBeInTheDocument();
    expect(screen.getByText(/Up to 8 samples:/)).toBeInTheDocument();
    expect(screen.getByText(/Up to 4 samples:/)).toBeInTheDocument();
    expect(screen.getByText(/Not filtered:/)).toBeInTheDocument();
  });

  it("displays deblocking parameters", async () => {
    await renderAct(
      <DeblockingView
        frame={mockFrame}
        width={TEST_WIDTH}
        height={TEST_HEIGHT}
        codec="hevc"
      />,
    );

    expect(screen.getByText("Loop Filter Parameters")).toBeInTheDocument();
    expect(screen.getByText(/Level \(Y vert\/horz\):/)).toBeInTheDocument();
    expect(screen.getByText(/Level \(U \/ V\):/)).toBeInTheDocument();
    expect(screen.getByText(/Sharpness:/)).toBeInTheDocument();
  });

  it("displays codec-specific notes for AV1", async () => {
    await renderAct(
      <DeblockingView
        frame={mockFrame}
        width={TEST_WIDTH}
        height={TEST_HEIGHT}
        codec="AV1"
      />,
    );

    expect(
      screen.getByText(/AV1.*CDEF.*Loop Restoration/i),
    ).toBeInTheDocument();
  });

  it("displays codec-specific notes for HEVC", async () => {
    await renderAct(
      <DeblockingView
        frame={mockFrame}
        width={TEST_WIDTH}
        height={TEST_HEIGHT}
        codec="HEVC"
      />,
    );

    expect(screen.getByText(/HEVC.*8x8 block boundaries/i)).toBeInTheDocument();
  });

  it("displays codec-specific notes for lowercase codec strings too (real activeCodec value from the sidecar is lowercase, e.g. 'hevc' not 'HEVC')", async () => {
    await renderAct(
      <DeblockingView
        frame={mockFrame}
        width={TEST_WIDTH}
        height={TEST_HEIGHT}
        codec="hevc"
      />,
    );

    expect(screen.getByText(/HEVC.*8x8 block boundaries/i)).toBeInTheDocument();
  });

  it("displays legend", async () => {
    await renderAct(
      <DeblockingView
        frame={mockFrame}
        width={TEST_WIDTH}
        height={TEST_HEIGHT}
        codec="HEVC"
      />,
    );

    // AV1 has no boundary strength (that is HEVC): edges are colored by filter length
    expect(screen.getByText("Up to 16-sample filter")).toBeInTheDocument();
    expect(screen.getByText("Up to 8-sample filter")).toBeInTheDocument();
    expect(screen.getByText("Up to 4-sample filter")).toBeInTheDocument();
    expect(screen.getByText("Considered, not filtered")).toBeInTheDocument();
    expect(screen.queryByText(/BS \d/)).toBeNull();
  });

  it("handles null frame gracefully", async () => {
    await renderAct(
      <DeblockingView
        frame={null}
        width={TEST_WIDTH}
        height={TEST_HEIGHT}
        codec="hevc"
      />,
    );

    expect(screen.getByText("No frame selected")).toBeInTheDocument();
  });
});
