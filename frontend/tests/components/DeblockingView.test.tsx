/**
 * Tests for DeblockingView component
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@/test/test-utils";
import { DeblockingView } from "../DeblockingView";
import type { FrameInfo } from "@/types/video";

// Use small dimensions (16x16) to keep SVG boundary count manageable (~6 elements)
// instead of 1920x1080 which generates ~65,000 elements per test.
const TEST_WIDTH = 16;
const TEST_HEIGHT = 16;

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

  it("renders without crashing", () => {
    render(
      <DeblockingView
        frame={mockFrame}
        width={TEST_WIDTH}
        height={TEST_HEIGHT}
        codec="hevc"
      />,
    );
    expect(screen.getByText("Deblocking Filter Analysis")).toBeInTheDocument();
  });

  it("displays frame information", () => {
    render(
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

  it("displays deblocking statistics", () => {
    render(
      <DeblockingView
        frame={mockFrame}
        width={TEST_WIDTH}
        height={TEST_HEIGHT}
        codec="hevc"
      />,
    );

    // The component generates mock stats
    expect(screen.getByText(/Total Boundaries:/)).toBeInTheDocument();
    expect(screen.getByText(/^Filtered:$/)).toBeInTheDocument();
  });

  it("displays deblocking parameters", () => {
    render(
      <DeblockingView
        frame={mockFrame}
        width={TEST_WIDTH}
        height={TEST_HEIGHT}
        codec="hevc"
      />,
    );

    expect(screen.getByText("Deblocking Parameters")).toBeInTheDocument();
    expect(screen.getByText(/β Offset:/)).toBeInTheDocument();
    expect(screen.getByText(/tc Offset:/)).toBeInTheDocument();
    expect(screen.getByText(/Filter Strength:/)).toBeInTheDocument();
  });

  it("displays codec-specific notes for AV1", () => {
    render(
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

  it("displays codec-specific notes for HEVC", () => {
    render(
      <DeblockingView
        frame={mockFrame}
        width={TEST_WIDTH}
        height={TEST_HEIGHT}
        codec="HEVC"
      />,
    );

    expect(screen.getByText(/HEVC.*8x8 block boundaries/i)).toBeInTheDocument();
  });

  it("displays legend", () => {
    render(
      <DeblockingView
        frame={mockFrame}
        width={TEST_WIDTH}
        height={TEST_HEIGHT}
        codec="HEVC"
      />,
    );

    expect(screen.getByText("Strong Boundary (BS 3-4)")).toBeInTheDocument();
    expect(screen.getByText("Weak Boundary (BS 1-2)")).toBeInTheDocument();
    expect(screen.getByText("Not Filtered")).toBeInTheDocument();
  });

  it("handles null frame gracefully", () => {
    render(
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
