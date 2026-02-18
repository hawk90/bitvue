/**
 * DPB View Tab Component Tests
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@/test/test-utils";
import { DpbViewTab } from "../DpbViewTab";

const mockFrames = [
  { frame_index: 5, frame_type: "I", pts: 100 },
  { frame_index: 10, frame_type: "P", pts: 200 },
  { frame_index: 20, frame_type: "P", pts: 400 },
  { frame_index: 30, frame_type: "B", pts: 600 },
];

const mockCurrentFrame = {
  frame_index: 40,
  frame_type: "P",
  pts: 800,
  ref_frames: [5, 10],
};

describe("DpbViewTab", () => {
  it("should render empty state when no current frame", () => {
    render(<DpbViewTab currentFrame={null} frames={mockFrames} />);

    expect(screen.getByText("No frame selected")).toBeInTheDocument();
  });

  it("should render empty state icon when no current frame", () => {
    const { container } = render(
      <DpbViewTab currentFrame={null} frames={mockFrames} />,
    );

    expect(container.querySelector(".codicon-database")).toBeInTheDocument();
  });

  it("should render DPB header", () => {
    render(<DpbViewTab currentFrame={mockCurrentFrame} frames={mockFrames} />);

    expect(screen.getByText("Decoded Picture Buffer")).toBeInTheDocument();
  });

  it("should render database icon", () => {
    const { container } = render(
      <DpbViewTab currentFrame={mockCurrentFrame} frames={mockFrames} />,
    );

    expect(container.querySelector(".codicon-database")).toBeInTheDocument();
  });

  it("should display current frame info", () => {
    render(<DpbViewTab currentFrame={mockCurrentFrame} frames={mockFrames} />);

    expect(screen.getByText("#40 (P)")).toBeInTheDocument();
  });

  it("should display current frame index and type", () => {
    const { container } = render(
      <DpbViewTab currentFrame={mockCurrentFrame} frames={mockFrames} />,
    );

    expect(container.textContent).toContain("40");
    expect(container.querySelector(".frame-type-p")).toBeInTheDocument();
  });

  it("should render table headers", () => {
    const { container } = render(
      <DpbViewTab currentFrame={mockCurrentFrame} frames={mockFrames} />,
    );

    const headers = container.querySelectorAll(".dpb-header-row span");
    const headerTexts = Array.from(headers).map((h) => h.textContent);

    expect(headerTexts).toContain("Idx");
    expect(headerTexts).toContain("Frame");
    expect(headerTexts).toContain("Type");
    expect(headerTexts).toContain("PTS");
    expect(headerTexts).toContain("Status");
  });

  it("should render current frame row", () => {
    const { container } = render(
      <DpbViewTab currentFrame={mockCurrentFrame} frames={mockFrames} />,
    );

    const currentRow = container.querySelector(".dpb-current");
    expect(currentRow).toBeInTheDocument();

    const statusCell = currentRow?.querySelector(".dpb-current");
    expect(statusCell?.textContent).toBe("Current");
  });

  it("should render reference frame rows", () => {
    const { container } = render(
      <DpbViewTab currentFrame={mockCurrentFrame} frames={mockFrames} />,
    );

    const refRows = container.querySelectorAll(".dpb-row");
    // Header row + Current row + 2 reference rows = 4 rows
    expect(refRows.length).toBeGreaterThanOrEqual(3);
  });

  it("should display reference frames info", () => {
    const { container } = render(
      <DpbViewTab currentFrame={mockCurrentFrame} frames={mockFrames} />,
    );

    expect(container.textContent).toContain("5");
    expect(container.textContent).toContain("10");
  });

  it("should render reference frame types", () => {
    const { container } = render(
      <DpbViewTab currentFrame={mockCurrentFrame} frames={mockFrames} />,
    );

    expect(container.querySelector(".frame-type-p")).toBeInTheDocument();
  });

  it("should render reference frame PTS values", () => {
    const { container } = render(
      <DpbViewTab currentFrame={mockCurrentFrame} frames={mockFrames} />,
    );

    // Check that some PTS values are rendered - 200 should appear somewhere in the output
    expect(container.textContent).toContain("200");
  });

  it('should show "Reference" status for ref frames', () => {
    const { container } = render(
      <DpbViewTab currentFrame={mockCurrentFrame} frames={mockFrames} />,
    );

    const refStatuses = container.querySelectorAll(".dpb-ref");
    expect(refStatuses.length).toBeGreaterThan(0);
    expect(refStatuses[0].textContent).toBe("Reference");
  });

  it("should handle missing reference frames", () => {
    const frameWithInvalidRefs = { ...mockCurrentFrame, ref_frames: [999] };
    const { container } = render(
      <DpbViewTab currentFrame={frameWithInvalidRefs} frames={mockFrames} />,
    );

    // Should render placeholders for missing frames
    const rows = container.querySelectorAll(".dpb-row");
    expect(rows.length).toBeGreaterThan(1);
  });

  it("should show placeholder for missing frame data", () => {
    const frameWithInvalidRefs = { ...mockCurrentFrame, ref_frames: [999] };
    const { container } = render(
      <DpbViewTab currentFrame={frameWithInvalidRefs} frames={mockFrames} />,
    );

    expect(container.textContent).toContain("-"); // Placeholder values
  });

  it("should render empty row message when no reference frames", () => {
    const frameWithoutRefs = { ...mockCurrentFrame, ref_frames: [] };
    render(<DpbViewTab currentFrame={frameWithoutRefs} frames={mockFrames} />);

    expect(
      screen.getByText("No reference frames (keyframe or intra-only)"),
    ).toBeInTheDocument();
  });

  it("should handle undefined ref_frames", () => {
    const frameWithUndefinedRefs = {
      ...mockCurrentFrame,
      ref_frames: undefined,
    };
    render(
      <DpbViewTab currentFrame={frameWithUndefinedRefs} frames={mockFrames} />,
    );

    expect(
      screen.getByText("No reference frames (keyframe or intra-only)"),
    ).toBeInTheDocument();
  });

  it("should render note about DPB contents", () => {
    render(<DpbViewTab currentFrame={mockCurrentFrame} frames={mockFrames} />);

    expect(
      screen.getByText("DPB shows current frame and its reference frames"),
    ).toBeInTheDocument();
  });

  it("should render info icon in note", () => {
    const { container } = render(
      <DpbViewTab currentFrame={mockCurrentFrame} frames={mockFrames} />,
    );

    expect(container.querySelector(".codicon-info")).toBeInTheDocument();
  });

  it("should handle different frame types", () => {
    const iFrame = { ...mockCurrentFrame, frame_type: "I", ref_frames: [] };
    const { container: iContainer, unmount } = render(
      <DpbViewTab currentFrame={iFrame} frames={mockFrames} />,
    );
    expect(iContainer.querySelector(".frame-type-i")).toBeInTheDocument();
    unmount();

    const pFrame = { ...mockCurrentFrame, frame_type: "P", ref_frames: [5] };
    const { container: pContainer, unmount: pUnmount } = render(
      <DpbViewTab currentFrame={pFrame} frames={mockFrames} />,
    );
    expect(pContainer.querySelector(".frame-type-p")).toBeInTheDocument();
    pUnmount();

    const bFrame = {
      ...mockCurrentFrame,
      frame_type: "B",
      ref_frames: [10, 20],
    };
    const { container: bContainer } = render(
      <DpbViewTab currentFrame={bFrame} frames={mockFrames} />,
    );
    expect(bContainer.querySelector(".frame-type-b")).toBeInTheDocument();
  });

  it("should display PTS or N/A for current frame", () => {
    const { container } = render(
      <DpbViewTab currentFrame={mockCurrentFrame} frames={mockFrames} />,
    );

    // Check that the current frame PTS is rendered somewhere in the output
    expect(container.textContent).toContain("800");
  });

  it("should display N/A for undefined PTS", () => {
    const frameWithoutPts = { ...mockCurrentFrame, pts: undefined };
    render(<DpbViewTab currentFrame={frameWithoutPts} frames={mockFrames} />);

    const naElements = screen.getAllByText("N/A");
    expect(naElements.length).toBeGreaterThan(0);
  });

  it("should handle multiple reference frames", () => {
    const frameWithManyRefs = {
      ...mockCurrentFrame,
      ref_frames: [5, 10, 20, 30],
    };
    const { container } = render(
      <DpbViewTab currentFrame={frameWithManyRefs} frames={mockFrames} />,
    );

    const refRows = container.querySelectorAll(".dpb-row");
    // Should have header + current + 4 ref rows
    expect(refRows.length).toBeGreaterThan(4);
  });
});
