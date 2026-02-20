/**
 * DualVideoView Component Tests
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { DualVideoView } from "../DualVideoView";

const mockLeftFrame = {
  frame_index: 100,
  frame_type: "I" as const,
  poc: 100,
  pts: 100,
  size: 50000,
  temporal_id: 0,
  spatial_id: 0,
  ref_frames: [],
};

const mockRightFrame = {
  frame_index: 100,
  frame_type: "I" as const,
  poc: 100,
  pts: 100,
  size: 35000,
  temporal_id: 0,
  spatial_id: 0,
  ref_frames: [],
};

describe("DualVideoView", () => {
  it("renders without crashing", () => {
    render(
      <DualVideoView
        leftFrame={null}
        rightFrame={null}
        leftWidth={1920}
        leftHeight={1080}
        rightWidth={1920}
        rightHeight={1080}
      />,
    );

    // "No video loaded" appears twice (for both sides)
    expect(screen.getAllByText("No video loaded")).toHaveLength(2);
  });

  it("renders both video placeholders", () => {
    render(
      <DualVideoView
        leftFrame={mockLeftFrame}
        rightFrame={mockRightFrame}
        leftWidth={1920}
        leftHeight={1080}
        rightWidth={1920}
        rightHeight={1080}
      />,
    );

    expect(screen.getByText("Reference (Original)")).toBeInTheDocument();
    expect(screen.getByText("Distorted (Encoded)")).toBeInTheDocument();
  });

  it("displays frame info for both sides", () => {
    render(
      <DualVideoView
        leftFrame={mockLeftFrame}
        rightFrame={mockRightFrame}
        leftWidth={1920}
        leftHeight={1080}
        rightWidth={1920}
        rightHeight={1080}
      />,
    );

    // Frame 100 appears twice (for left and right videos)
    expect(screen.getAllByText("Frame 100")).toHaveLength(2);
    expect(screen.getAllByText("I | 1920x1080")).toHaveLength(2);
  });

  it("renders view mode selector", () => {
    render(
      <DualVideoView
        leftFrame={mockLeftFrame}
        rightFrame={mockRightFrame}
        leftWidth={1920}
        leftHeight={1080}
        rightWidth={1920}
        rightHeight={1080}
      />,
    );

    expect(screen.getByLabelText("View:")).toBeInTheDocument();
  });

  it("switches view modes", async () => {
    const user = userEvent.setup();
    render(
      <DualVideoView
        leftFrame={mockLeftFrame}
        rightFrame={mockRightFrame}
        leftWidth={1920}
        leftHeight={1080}
        rightWidth={1920}
        rightHeight={1080}
      />,
    );

    const select = screen.getByLabelText("View:");
    await user.selectOptions(select, "top-bottom");

    expect(select).toHaveValue("top-bottom");
  });

  it("toggles sync option", async () => {
    const user = userEvent.setup();
    render(
      <DualVideoView
        leftFrame={mockLeftFrame}
        rightFrame={mockRightFrame}
        leftWidth={1920}
        leftHeight={1080}
        rightWidth={1920}
        rightHeight={1080}
      />,
    );

    const syncCheckbox = screen.getByLabelText(/sync/i);
    await user.click(syncCheckbox);

    expect(syncCheckbox).not.toBeChecked();
  });

  it("toggles grid option", async () => {
    const user = userEvent.setup();
    render(
      <DualVideoView
        leftFrame={mockLeftFrame}
        rightFrame={mockRightFrame}
        leftWidth={1920}
        leftHeight={1080}
        rightWidth={1920}
        rightHeight={1080}
      />,
    );

    const gridCheckbox = screen.getByLabelText(/grid/i);
    await user.click(gridCheckbox);

    expect(gridCheckbox).toBeChecked();
  });

  it("handles zoom in", async () => {
    const user = userEvent.setup();
    render(
      <DualVideoView
        leftFrame={mockLeftFrame}
        rightFrame={mockRightFrame}
        leftWidth={1920}
        leftHeight={1080}
        rightWidth={1920}
        rightHeight={1080}
      />,
    );

    const zoomInButton = screen.getByRole("button", { name: "+" });
    await user.click(zoomInButton);

    expect(screen.getByText("125%")).toBeInTheDocument();
  });

  it("handles zoom out", async () => {
    const user = userEvent.setup();
    render(
      <DualVideoView
        leftFrame={mockLeftFrame}
        rightFrame={mockRightFrame}
        leftWidth={1920}
        leftHeight={1080}
        rightWidth={1920}
        rightHeight={1080}
      />,
    );

    const zoomOutButton = screen.getByRole("button", { name: "−" });
    await user.click(zoomOutButton);

    expect(screen.getByText("75%")).toBeInTheDocument();
  });

  it("resets view when reset button clicked", async () => {
    const user = userEvent.setup();
    render(
      <DualVideoView
        leftFrame={mockLeftFrame}
        rightFrame={mockRightFrame}
        leftWidth={1920}
        leftHeight={1080}
        rightWidth={1920}
        rightHeight={1080}
      />,
    );

    // First zoom in
    await user.click(screen.getByRole("button", { name: "+" }));
    expect(screen.getByText("125%")).toBeInTheDocument();

    // Then reset
    await user.click(screen.getByRole("button", { name: "Reset" }));
    expect(screen.getByText("100%")).toBeInTheDocument();
  });

  it("renders difference view mode", () => {
    render(
      <DualVideoView
        leftFrame={mockLeftFrame}
        rightFrame={mockRightFrame}
        leftWidth={1920}
        leftHeight={1080}
        rightWidth={1920}
        rightHeight={1080}
        viewMode="difference"
      />,
    );

    expect(screen.getByText("Difference Map")).toBeInTheDocument();
    expect(screen.getByText(/heatmap visualization/i)).toBeInTheDocument();
  });

  it("renders slide view mode", () => {
    render(
      <DualVideoView
        leftFrame={mockLeftFrame}
        rightFrame={mockRightFrame}
        leftWidth={1920}
        leftHeight={1080}
        rightWidth={1920}
        rightHeight={1080}
        viewMode="slide"
      />,
    );

    expect(screen.getByText("Reference")).toBeInTheDocument();
    expect(screen.getByText("Distorted")).toBeInTheDocument();
  });

  it("shows frame info footer when enabled", () => {
    render(
      <DualVideoView
        leftFrame={mockLeftFrame}
        rightFrame={mockRightFrame}
        leftWidth={1920}
        leftHeight={1080}
        rightWidth={1920}
        rightHeight={1080}
        showFrameInfo={true}
      />,
    );

    expect(screen.getByText(/L:/)).toBeInTheDocument();
    expect(screen.getByText(/R:/)).toBeInTheDocument();
  });

  it("hides frame info footer when disabled", () => {
    render(
      <DualVideoView
        leftFrame={mockLeftFrame}
        rightFrame={mockRightFrame}
        leftWidth={1920}
        leftHeight={1080}
        rightWidth={1920}
        rightHeight={1080}
        showFrameInfo={false}
      />,
    );

    // Frame info footer should not be shown when disabled
    expect(screen.queryByText(/Hold Shift to scroll/)).not.toBeInTheDocument();
  });

  it("disables zoom in at max zoom", async () => {
    const user = userEvent.setup();
    render(
      <DualVideoView
        leftFrame={mockLeftFrame}
        rightFrame={mockRightFrame}
        leftWidth={1920}
        leftHeight={1080}
        rightWidth={1920}
        rightHeight={1080}
      />,
    );

    const zoomInButton = screen.getByRole("button", { name: "+" });

    // Zoom to max
    for (let i = 0; i < 15; i++) {
      await user.click(zoomInButton);
    }

    // Should be disabled at 400%
    expect(zoomInButton).toBeDisabled();
  });

  it("disables zoom out at min zoom", async () => {
    const user = userEvent.setup();
    render(
      <DualVideoView
        leftFrame={mockLeftFrame}
        rightFrame={mockRightFrame}
        leftWidth={1920}
        leftHeight={1080}
        rightWidth={1920}
        rightHeight={1080}
      />,
    );

    const zoomOutButton = screen.getByRole("button", { name: "−" });

    // Zoom to min
    for (let i = 0; i < 5; i++) {
      await user.click(zoomOutButton);
    }

    // Should be disabled at 25%
    expect(zoomOutButton).toBeDisabled();
  });
});
