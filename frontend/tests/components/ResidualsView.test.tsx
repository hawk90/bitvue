/**
 * Tests for ResidualsView component
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@/test/test-utils";
import { ResidualsView } from "../ResidualsView";
import type { FrameInfo } from "@/types/video";
import type { ResidualAnalysisWireResult } from "../../services/electronBridgeService";

// ResidualsView fetches real data via getResidualAnalysis -- unmocked, that call throws in jsdom
// (no window.bitvue), and the .catch() leaves coefficientStats permanently null, so the
// Non-Zero/Zero Coeffs stats (guarded by `coefficientStats &&`) never render. Mock the bridge
// module directly with a realistic resolved response, same pattern as AV1FeaturesView.test.tsx.
const mockGetResidualAnalysis =
  vi.fn<(frameIndex: number) => Promise<ResidualAnalysisWireResult>>();

vi.mock("../../services/electronBridgeService", () => ({
  getResidualAnalysis: (frameIndex: number) =>
    mockGetResidualAnalysis(frameIndex),
}));

const mockWireResult: ResidualAnalysisWireResult = {
  frame_index: 100,
  width: 1920,
  height: 1080,
  coefficient_stats: {
    min: 0,
    max: 64,
    mean: 8.5,
    variance: 12.2,
    energy: 4096,
    zero_count: 120,
    non_zero_count: 80,
  },
  block_residuals: [
    {
      x: 0,
      y: 0,
      width: 8,
      height: 8,
      energy: 32,
      max_coeff: 12,
      non_zeros: 5,
    },
  ],
};

describe("ResidualsView", () => {
  beforeEach(() => {
    mockGetResidualAnalysis.mockReset();
    mockGetResidualAnalysis.mockResolvedValue(mockWireResult);
  });

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
    render(<ResidualsView frame={mockFrame} width={1920} height={1080} />);
    expect(screen.getByText("Residuals Analysis")).toBeInTheDocument();
  });

  it("displays frame information", () => {
    render(<ResidualsView frame={mockFrame} width={1920} height={1080} />);

    expect(screen.getByText("Frame 100")).toBeInTheDocument();
    expect(screen.getByText("P")).toBeInTheDocument();
  });

  it("displays coefficient statistics", async () => {
    render(<ResidualsView frame={mockFrame} width={1920} height={1080} />);

    await waitFor(() =>
      expect(screen.getByText(/Non-Zero Coeffs:/)).toBeInTheDocument(),
    );
    // Use exact text to avoid matching "Non-Zero Coeffs:" with /Zero Coeffs:/
    expect(screen.getByText("Zero Coeffs:")).toBeInTheDocument();
  });

  it("renders heatmap view when showHeatmap is true", () => {
    render(
      <ResidualsView
        frame={mockFrame}
        width={1920}
        height={1080}
        showHeatmap={true}
        showHistogram={false}
      />,
    );

    expect(screen.getByText("Residual Energy Heatmap")).toBeInTheDocument();
  });

  it("renders histogram view when showHistogram is true", () => {
    render(
      <ResidualsView
        frame={mockFrame}
        width={1920}
        height={1080}
        showHeatmap={false}
        showHistogram={true}
      />,
    );

    expect(screen.getByText("Coefficient Distribution")).toBeInTheDocument();
  });

  it("renders both views when both are enabled", () => {
    render(
      <ResidualsView
        frame={mockFrame}
        width={1920}
        height={1080}
        showHeatmap={true}
        showHistogram={true}
      />,
    );

    expect(screen.getByText("Residual Energy Heatmap")).toBeInTheDocument();
    expect(screen.getByText("Coefficient Distribution")).toBeInTheDocument();
  });

  it("handles null frame gracefully", () => {
    render(<ResidualsView frame={null} width={1920} height={1080} />);

    expect(screen.getByText("No frame selected")).toBeInTheDocument();
  });

  it("displays heatmap color scale legend", () => {
    render(
      <ResidualsView
        frame={mockFrame}
        width={1920}
        height={1080}
        showHeatmap={true}
      />,
    );

    expect(screen.getByText("Low")).toBeInTheDocument();
    expect(screen.getByText("High")).toBeInTheDocument();
  });
});
