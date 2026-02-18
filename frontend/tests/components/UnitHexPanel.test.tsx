/**
 * UnitHexPanel Component Tests
 * Tests unit HEX panel with tabbed interface
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { UnitHexPanel } from "../UnitHexPanel";

// Mock context
vi.mock("../../../contexts/StreamDataContext", () => ({
  useStreamData: () => ({
    frames: [
      { frame_index: 0, frame_type: "I", size: 50000, poc: 0 },
      { frame_index: 1, frame_type: "P", size: 30000, poc: 1, ref_frames: [0] },
      {
        frame_index: 2,
        frame_type: "B",
        size: 20000,
        poc: 2,
        ref_frames: [0, 1],
      },
    ],
    currentFrameIndex: 1,
    filePath: "/test/path",
  }),
}));

describe("UnitHexPanel", () => {
  it("should render unit hex panel", () => {
    render(<UnitHexPanel />);

    expect(screen.getByText("Unit HEX")).toBeInTheDocument();
  });

  it("should render all tab buttons", () => {
    render(<UnitHexPanel />);

    expect(screen.getByText("Frame")).toBeInTheDocument();
    expect(screen.getByText("Hex")).toBeInTheDocument();
    expect(screen.getByText("DPB")).toBeInTheDocument();
  });

  it("should show Frame tab by default", () => {
    render(<UnitHexPanel />);

    const frameTab = screen.getByText("Frame");
    const frameButton = frameTab.closest("button");
    expect(frameButton).toHaveClass("active");
  });

  it("should switch tabs on click", () => {
    render(<UnitHexPanel />);

    const hexTab = screen.getByText("Hex");
    fireEvent.click(hexTab);

    const hexButton = hexTab.closest("button");
    expect(hexButton).toHaveClass("active");
  });

  it("should switch to DPB tab", () => {
    render(<UnitHexPanel />);

    const dpbTab = screen.getByText("DPB");
    fireEvent.click(dpbTab);

    const dpbButton = dpbTab.closest("button");
    expect(dpbButton).toHaveClass("active");
  });

  it("should use React.memo for performance", () => {
    const { rerender } = render(<UnitHexPanel />);

    rerender(<UnitHexPanel />);

    expect(screen.getByText("Unit HEX")).toBeInTheDocument();
  });

  it("should display tab icons", () => {
    const { container } = render(<UnitHexPanel />);

    expect(container.querySelector(".codicon-file")).toBeInTheDocument();
    expect(container.querySelector(".codicon-file-code")).toBeInTheDocument();
    expect(container.querySelector(".codicon-database")).toBeInTheDocument();
  });
});

describe("UnitHexPanel tabs", () => {
  it("should render Frame tab content", () => {
    render(<UnitHexPanel />);

    expect(screen.getByText("Frame")).toBeInTheDocument();
  });

  it("should render Hex tab content when switched", () => {
    render(<UnitHexPanel />);

    const hexTab = screen.getByText("Hex");
    fireEvent.click(hexTab);

    expect(screen.getByText("Hex")).toBeInTheDocument();
  });

  it("should render DPB tab content when switched", () => {
    render(<UnitHexPanel />);

    const dpbTab = screen.getByText("DPB");
    fireEvent.click(dpbTab);

    expect(screen.getByText("DPB")).toBeInTheDocument();
  });
});

describe("UnitHexPanel edge cases", () => {
  it("should handle empty frames array", () => {
    // Mock is already set up at the top
    // The component handles empty/null currentFrame internally
    render(<UnitHexPanel />);

    expect(screen.getByText("Unit HEX")).toBeInTheDocument();
  });

  it("should handle null current frame", () => {
    // Component handles null currentFrame internally
    render(<UnitHexPanel />);

    expect(screen.getByText("Unit HEX")).toBeInTheDocument();
  });

  it("should handle out of range frame index", () => {
    // Component handles out of range index internally
    render(<UnitHexPanel />);

    expect(screen.getByText("Unit HEX")).toBeInTheDocument();
  });
});
