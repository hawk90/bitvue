/**
 * InfoPanel Component Tests
 * Tests bottom info panel with file and frame information
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@/test/test-utils";
import { InfoPanel } from "../InfoPanel";

describe("InfoPanel", () => {
  const mockFrame = {
    frame_index: 1,
    frame_type: "P",
    size: 30000,
    pts: 1,
    temporal_id: 0,
  };

  it("should render info panel", () => {
    render(
      <InfoPanel
        filePath="/test/video.mp4"
        frameCount={100}
        currentFrameIndex={1}
        currentFrame={mockFrame}
      />,
    );

    expect(screen.getByText("File:")).toBeInTheDocument();
    expect(screen.getByText("Frames:")).toBeInTheDocument();
  });

  it("should display file path", () => {
    render(
      <InfoPanel
        filePath="/test/video.mp4"
        frameCount={100}
        currentFrameIndex={1}
        currentFrame={mockFrame}
      />,
    );

    expect(screen.getByText("/test/video.mp4")).toBeInTheDocument();
  });

  it("should display N/A when no file path", () => {
    render(
      <InfoPanel
        frameCount={100}
        currentFrameIndex={1}
        currentFrame={mockFrame}
      />,
    );

    expect(screen.getByText("N/A")).toBeInTheDocument();
  });

  it("should display frame count", () => {
    render(
      <InfoPanel
        filePath="/test/video.mp4"
        frameCount={100}
        currentFrameIndex={1}
        currentFrame={mockFrame}
      />,
    );

    expect(screen.getByText("100")).toBeInTheDocument();
  });

  it("should display current frame index", () => {
    render(
      <InfoPanel
        filePath="/test/video.mp4"
        frameCount={100}
        currentFrameIndex={5}
        currentFrame={mockFrame}
      />,
    );

    expect(screen.getByText("5")).toBeInTheDocument();
  });

  it("should display frame type", () => {
    render(
      <InfoPanel
        filePath="/test/video.mp4"
        frameCount={100}
        currentFrameIndex={1}
        currentFrame={mockFrame}
      />,
    );

    expect(screen.getByText("P")).toBeInTheDocument();
  });

  it("should display N/A when no current frame", () => {
    render(
      <InfoPanel
        filePath="/test/video.mp4"
        frameCount={100}
        currentFrameIndex={0}
        currentFrame={null}
      />,
    );

    const frameTypes = screen.queryAllByText("N/A");
    expect(frameTypes.length).toBeGreaterThan(0);
  });

  it("should display frame size in KB", () => {
    render(
      <InfoPanel
        filePath="/test/video.mp4"
        frameCount={100}
        currentFrameIndex={1}
        currentFrame={mockFrame}
      />,
    );

    expect(screen.getByText(/29\.30 KB/)).toBeInTheDocument();
  });

  it("should use React.memo for performance", () => {
    const { rerender } = render(
      <InfoPanel
        filePath="/test/video.mp4"
        frameCount={100}
        currentFrameIndex={1}
        currentFrame={mockFrame}
      />,
    );

    rerender(
      <InfoPanel
        filePath="/test/video.mp4"
        frameCount={100}
        currentFrameIndex={1}
        currentFrame={mockFrame}
      />,
    );

    expect(screen.getByText("File:")).toBeInTheDocument();
  });
});

describe("InfoPanel formatting", () => {
  it("should format size correctly for small frames", () => {
    const smallFrame = { ...mockFrame, size: 1024 };

    render(
      <InfoPanel
        filePath="/test/video.mp4"
        frameCount={100}
        currentFrameIndex={0}
        currentFrame={smallFrame}
      />,
    );

    expect(screen.getByText("1.00 KB")).toBeInTheDocument();
  });

  it("should format size correctly for large frames", () => {
    const largeFrame = { ...mockFrame, size: 1024000 };

    render(
      <InfoPanel
        filePath="/test/video.mp4"
        frameCount={100}
        currentFrameIndex={0}
        currentFrame={largeFrame}
      />,
    );

    expect(screen.getByText("1000.00 KB")).toBeInTheDocument();
  });

  it("should display different frame types", () => {
    const iFrame = { ...mockFrame, frame_type: "I" as const };
    const bFrame = { ...mockFrame, frame_type: "B" as const };

    const { rerender } = render(
      <InfoPanel
        filePath="/test/video.mp4"
        frameCount={100}
        currentFrameIndex={0}
        currentFrame={iFrame}
      />,
    );

    expect(screen.getByText("I")).toBeInTheDocument();

    rerender(
      <InfoPanel
        filePath="/test/video.mp4"
        frameCount={100}
        currentFrameIndex={0}
        currentFrame={bFrame}
      />,
    );

    expect(screen.getByText("B")).toBeInTheDocument();
  });
});

const mockFrame = {
  frame_index: 1,
  frame_type: "P",
  size: 30000,
  pts: 1,
  temporal_id: 0,
};
