/**
 * Frame View Tab Component Tests
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@/test/test-utils";
import { FrameViewTab } from "../FrameViewTab";

const mockFrame = {
  frame_index: 42,
  frame_type: "I",
  size: 102400,
  pts: 1000,
  temporal_id: 0,
  display_order: 42,
  coding_order: 40,
  ref_frames: [10, 20, 30],
};

describe("FrameViewTab", () => {
  it("should render empty state when no frame selected", () => {
    render(<FrameViewTab frame={null} />);

    expect(screen.getByText("No frame selected")).toBeInTheDocument();
  });

  it("should render empty state icon", () => {
    const { container } = render(<FrameViewTab frame={null} />);

    expect(container.querySelector(".codicon-file")).toBeInTheDocument();
  });

  it("should render frame type badge", () => {
    const { container } = render(<FrameViewTab frame={mockFrame} />);

    // Use querySelector to find the specific badge element
    const badge = container.querySelector(".frame-info-type-badge");
    expect(badge).toBeInTheDocument();
    expect(badge).toHaveTextContent("I");
  });

  it("should render frame title", () => {
    render(<FrameViewTab frame={mockFrame} />);

    expect(screen.getByText("Frame 42")).toBeInTheDocument();
  });

  it("should render frame index", () => {
    const { container } = render(<FrameViewTab frame={mockFrame} />);

    // Find the frame index value by looking for the label-value pair
    const labelElements = container.querySelectorAll(".frame-info-label");
    const frameIndexLabel = Array.from(labelElements).find(
      (el) => el.textContent === "Frame Index:",
    );
    expect(frameIndexLabel).toBeInTheDocument();

    // Check the value in the next sibling
    const valueElement = frameIndexLabel?.nextElementSibling as HTMLElement;
    expect(valueElement?.textContent).toBe("42");
  });

  it("should render frame type", () => {
    const { container } = render(<FrameViewTab frame={mockFrame} />);

    const typeElements = container.querySelectorAll(".frame-type-i");
    expect(typeElements.length).toBeGreaterThan(0);
  });

  it("should render frame size in KB and bytes", () => {
    render(<FrameViewTab frame={mockFrame} />);

    expect(screen.getByText(/100\.00 KB/)).toBeInTheDocument();
    expect(screen.getByText(/102400 bytes/)).toBeInTheDocument();
  });

  it("should render PTS when available", () => {
    const { container } = render(<FrameViewTab frame={mockFrame} />);

    // Find the PTS value by looking for the label-value pair
    const labelElements = container.querySelectorAll(".frame-info-label");
    const ptsLabel = Array.from(labelElements).find(
      (el) => el.textContent === "PTS:",
    );
    expect(ptsLabel).toBeInTheDocument();

    // Check the value in the next sibling
    const valueElement = ptsLabel?.nextElementSibling as HTMLElement;
    expect(valueElement?.textContent).toBe("1000");
  });

  it("should not render PTS label when undefined", () => {
    const frameWithoutPts = { ...mockFrame, pts: undefined };
    const { container } = render(<FrameViewTab frame={frameWithoutPts} />);

    const ptsLabels = container.querySelectorAll(".frame-info-label");
    const hasPtsLabel = Array.from(ptsLabels).some(
      (el) => el.textContent === "PTS:",
    );
    expect(hasPtsLabel).toBe(false);
  });

  it("should render temporal ID when available", () => {
    render(<FrameViewTab frame={mockFrame} />);

    const temporalIdValues = screen.getAllByText("0");
    expect(temporalIdValues.length).toBeGreaterThan(0);
  });

  it("should not render temporal ID when undefined", () => {
    const frameWithoutTemporal = { ...mockFrame, temporal_id: undefined };
    const { container } = render(<FrameViewTab frame={frameWithoutTemporal} />);

    const temporalLabels = container.querySelectorAll(".frame-info-label");
    const hasTemporalLabel = Array.from(temporalLabels).some(
      (el) => el.textContent === "Temporal ID:",
    );
    expect(hasTemporalLabel).toBe(false);
  });

  it("should render display order when available", () => {
    render(<FrameViewTab frame={mockFrame} />);

    const displayOrderValues = screen.getAllByText("42");
    expect(displayOrderValues.length).toBeGreaterThan(0);
  });

  it("should not render display order when undefined", () => {
    const frameWithoutDisplay = { ...mockFrame, display_order: undefined };
    const { container } = render(<FrameViewTab frame={frameWithoutDisplay} />);

    const displayLabels = container.querySelectorAll(".frame-info-label");
    const hasDisplayLabel = Array.from(displayLabels).some(
      (el) => el.textContent === "Display Order:",
    );
    expect(hasDisplayLabel).toBe(false);
  });

  it("should render coding order when available", () => {
    render(<FrameViewTab frame={mockFrame} />);

    const codingOrderValues = screen.getAllByText("40");
    expect(codingOrderValues.length).toBeGreaterThan(0);
  });

  it("should not render coding order when undefined", () => {
    const frameWithoutCoding = { ...mockFrame, coding_order: undefined };
    const { container } = render(<FrameViewTab frame={frameWithoutCoding} />);

    const codingLabels = container.querySelectorAll(".frame-info-label");
    const hasCodingLabel = Array.from(codingLabels).some(
      (el) => el.textContent === "Coding Order:",
    );
    expect(hasCodingLabel).toBe(false);
  });

  it("should render reference frames count", () => {
    render(<FrameViewTab frame={mockFrame} />);

    expect(screen.getByText("3 frames")).toBeInTheDocument();
  });

  it('should render "1 frame" for single reference', () => {
    const frameWithSingleRef = { ...mockFrame, ref_frames: [10] };
    render(<FrameViewTab frame={frameWithSingleRef} />);

    expect(screen.getByText("1 frame")).toBeInTheDocument();
  });

  it('should render "0 frames" for no references', () => {
    const frameWithNoRefs = { ...mockFrame, ref_frames: [] };
    render(<FrameViewTab frame={frameWithNoRefs} />);

    expect(screen.getByText("0 frames")).toBeInTheDocument();
  });

  it("should render reference frames list when available", () => {
    render(<FrameViewTab frame={mockFrame} />);

    expect(screen.getByText("10, 20, 30")).toBeInTheDocument();
  });

  it("should not render reference frames list when empty", () => {
    const frameWithNoRefs = { ...mockFrame, ref_frames: [] };
    const { container } = render(<FrameViewTab frame={frameWithNoRefs} />);

    const refLabels = container.querySelectorAll(".frame-info-label");
    const hasRefLabel = Array.from(refLabels).some(
      (el) => el.textContent === "Ref Frames:",
    );
    expect(hasRefLabel).toBe(false);
  });

  it("should render note about parser integration", () => {
    render(<FrameViewTab frame={mockFrame} />);

    expect(
      screen.getByText(
        /Full bitstream data available after parser integration/,
      ),
    ).toBeInTheDocument();
  });

  it("should render info icon", () => {
    const { container } = render(<FrameViewTab frame={mockFrame} />);

    expect(container.querySelector(".codicon-info")).toBeInTheDocument();
  });

  it("should handle different frame types", () => {
    const frameTypes: Array<"I" | "P" | "B" | "Unknown"> = [
      "I",
      "P",
      "B",
      "Unknown",
    ];

    frameTypes.forEach((frameType) => {
      const frame = { ...mockFrame, frame_type: frameType };
      const { container, unmount } = render(<FrameViewTab frame={frame} />);

      const typeBadge = container.querySelector(".frame-info-type-badge");
      expect(typeBadge).toHaveTextContent(frameType);
      unmount();
    });
  });

  it("should render all labels correctly", () => {
    const { container } = render(<FrameViewTab frame={mockFrame} />);

    const labels = container.querySelectorAll(".frame-info-label");
    const labelTexts = Array.from(labels).map((el) => el.textContent);

    expect(labelTexts).toContain("Frame Index:");
    expect(labelTexts).toContain("Frame Type:");
    expect(labelTexts).toContain("Size:");
    expect(labelTexts).toContain("PTS:");
    expect(labelTexts).toContain("Temporal ID:");
    expect(labelTexts).toContain("Display Order:");
    expect(labelTexts).toContain("Coding Order:");
    expect(labelTexts).toContain("References:");
    expect(labelTexts).toContain("Ref Frames:");
  });
});
