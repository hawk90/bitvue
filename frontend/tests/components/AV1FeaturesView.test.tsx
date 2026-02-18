/**
 * Tests for AV1FeaturesView component
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { AV1FeaturesView } from "../AV1FeaturesView";
import type { FrameInfo } from "@/types/video";

describe("AV1FeaturesView", () => {
  const mockFrame: FrameInfo = {
    frame_index: 100,
    frame_type: "P",
    poc: 100,
    pts: 100,
    size: 25000,
    temporal_id: 1,
    spatial_id: 0,
    ref_frames: [99, 101],
  };

  it("renders without crashing", () => {
    render(<AV1FeaturesView frame={mockFrame} width={64} height={64} />);
    expect(screen.getByText("AV1 Advanced Features")).toBeInTheDocument();
  });

  it("displays frame information", () => {
    render(<AV1FeaturesView frame={mockFrame} width={64} height={64} />);

    expect(screen.getByText("Frame 100")).toBeInTheDocument();
    expect(screen.getByText("P")).toBeInTheDocument();
  });

  it("displays CDEF section when enabled", () => {
    render(
      <AV1FeaturesView
        frame={mockFrame}
        width={64}
        height={64}
        showCdef={true}
      />,
    );

    expect(
      screen.getByText("CDEF (Constrained Directional Enhancement Filter)"),
    ).toBeInTheDocument();
    expect(screen.getByText(/Blocks:/)).toBeInTheDocument();
  });

  it("displays Loop Restoration section when enabled", () => {
    render(
      <AV1FeaturesView
        frame={mockFrame}
        width={64}
        height={64}
        showLoopRestoration={true}
      />,
    );

    expect(screen.getByText("Loop Restoration")).toBeInTheDocument();
  });

  it("displays Film Grain section when enabled", () => {
    render(
      <AV1FeaturesView
        frame={mockFrame}
        width={64}
        height={64}
        showFilmGrain={true}
      />,
    );

    expect(screen.getByText("Film Grain Synthesis")).toBeInTheDocument();
  });

  it("displays Super Resolution section when enabled", () => {
    render(
      <AV1FeaturesView
        frame={mockFrame}
        width={64}
        height={64}
        showSuperRes={true}
      />,
    );

    expect(screen.getByText("Super Resolution")).toBeInTheDocument();
  });

  it("displays film grain parameters", () => {
    render(
      <AV1FeaturesView
        frame={mockFrame}
        width={64}
        height={64}
        showFilmGrain={true}
      />,
    );

    expect(screen.getByText(/Seed:/)).toBeInTheDocument();
    expect(screen.getByText(/AR Coeff Lag:/)).toBeInTheDocument();
  });

  it("displays legend", () => {
    render(<AV1FeaturesView frame={mockFrame} width={64} height={64} />);

    expect(screen.getByText(/CDEF Direction/)).toBeInTheDocument();
    expect(screen.getByText(/Wiener Filter/)).toBeInTheDocument();
    expect(screen.getByText(/SgrProj Filter/)).toBeInTheDocument();
  });

  it("handles null frame gracefully", () => {
    const { container } = render(
      <AV1FeaturesView frame={null} width={64} height={64} />,
    );

    expect(screen.getByText("No frame selected")).toBeInTheDocument();
  });
});
