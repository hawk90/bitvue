/**
 * Tests for CodingFlowView component
 */

import { describe, it, expect } from "vitest";
import { screen } from "@testing-library/react";
import { renderAct } from "@/test/test-utils";
import { CodingFlowView } from "../CodingFlowView";
import { muteBridgeUnavailableLogs } from "@/test/muteBridgeUnavailable";

muteBridgeUnavailableLogs();

describe("CodingFlowView", () => {
  const mockFrame = {
    frame_index: 100,
    frame_type: "P" as const,
    size: 25000,
  };

  it("renders without crashing", async () => {
    await renderAct(<CodingFlowView frame={mockFrame} codec="hevc" />);
    expect(screen.getByText(/Coding Flow/)).toBeInTheDocument();
  });

  it("displays frame information", async () => {
    await renderAct(<CodingFlowView frame={mockFrame} codec="hevc" />);

    expect(screen.getByText("Frame 100")).toBeInTheDocument();
    expect(screen.getByText("P")).toBeInTheDocument();
  });

  it("displays codec-specific features for HEVC", async () => {
    await renderAct(<CodingFlowView frame={mockFrame} codec="hevc" />);

    expect(screen.getByText("35 Intra Modes")).toBeInTheDocument();
    expect(screen.getByText("Advanced Motion Vector Pred")).toBeInTheDocument();
  });

  it("displays codec-specific features for AV1", async () => {
    await renderAct(<CodingFlowView frame={mockFrame} codec="av1" />);

    expect(screen.getByText("Directional Intra Pred")).toBeInTheDocument();
    expect(screen.getByText("Compound Prediction")).toBeInTheDocument();
  });

  it("displays codec-specific features for VVC", async () => {
    await renderAct(<CodingFlowView frame={mockFrame} codec="vvc" />);

    expect(screen.getByText("67 Intra Modes")).toBeInTheDocument();
    expect(screen.getByText("GPM/Combine Pred")).toBeInTheDocument();
  });

  it("renders all pipeline stages", async () => {
    await renderAct(<CodingFlowView frame={mockFrame} codec="hevc" />);

    expect(screen.getByText("Input")).toBeInTheDocument();
    expect(screen.getByText("Prediction")).toBeInTheDocument();
    expect(screen.getByText("Transform")).toBeInTheDocument();
    expect(screen.getByText("Quantization")).toBeInTheDocument();
    expect(screen.getByText("Entropy Coding")).toBeInTheDocument();
  });

  it("renders legend", async () => {
    await renderAct(<CodingFlowView frame={mockFrame} codec="hevc" />);

    expect(screen.getByText("Current Stage")).toBeInTheDocument();
    expect(screen.getByText("Completed Flow")).toBeInTheDocument();
    expect(screen.getByText("Encoder Only")).toBeInTheDocument();
    expect(screen.getByText("Decoder Path")).toBeInTheDocument();
  });

  it("handles null frame gracefully", async () => {
    const { container } = await renderAct(
      <CodingFlowView frame={null} codec="hevc" />,
    );

    // Should render without crashing
    expect(container).toBeInTheDocument();
  });
});
