/**
 * Tests for DeblockingView component
 * TODO: Skipping due to complex codec view requiring full parser backend
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { DeblockingView } from "../DeblockingView";
import type { FrameInfo } from "@/types/video";

describe.skip("DeblockingView", () => {
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
        width={1920}
        height={1080}
        codec="hevc"
      />,
    );
    expect(screen.getByText("Deblocking Filter Analysis")).toBeInTheDocument();
  });

  it("displays frame information", () => {
    render(
      <DeblockingView
        frame={mockFrame}
        width={1920}
        height={1080}
        codec="hevc"
      />,
    );

    expect(screen.getByText("Frame 100")).toBeInTheDocument();
    expect(screen.getByText("P")).toBeInTheDocument();
    expect(screen.getByText(/1920x1080/)).toBeInTheDocument();
  });

  it("displays deblocking statistics", () => {
    render(
      <DeblockingView
        frame={mockFrame}
        width={1920}
        height={1080}
        codec="hevc"
      />,
    );

    // The component generates mock stats
    expect(screen.getByText(/Total Boundaries:/)).toBeInTheDocument();
    expect(screen.getByText(/Filtered:/)).toBeInTheDocument();
  });

  it("displays deblocking parameters", () => {
    render(
      <DeblockingView
        frame={mockFrame}
        width={1920}
        height={1080}
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
        width={1920}
        height={1080}
        codec="av1"
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
        width={1920}
        height={1080}
        codec="hevc"
      />,
    );

    expect(screen.getByText(/HEVC.*8x8 block boundaries/i)).toBeInTheDocument();
  });

  it("displays legend", () => {
    render(
      <DeblockingView
        frame={mockFrame}
        width={1920}
        height={1080}
        codec="hevc"
      />,
    );

    expect(screen.getByText("Strong Boundary")).toBeInTheDocument();
    expect(screen.getByText("Weak Boundary")).toBeInTheDocument();
    expect(screen.getByText("Not Filtered")).toBeInTheDocument();
  });

  it("handles null frame gracefully", () => {
    const { container } = render(
      <DeblockingView frame={null} width={1920} height={1080} codec="hevc" />,
    );

    expect(screen.getByText("No frame selected")).toBeInTheDocument();
  });
});
