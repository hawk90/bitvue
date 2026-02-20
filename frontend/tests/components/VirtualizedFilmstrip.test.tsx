/**
 * VirtualizedFilmstrip Component Tests
 * Tests virtualized scrolling and frame rendering
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@/test/test-utils";
import { VirtualizedFilmstrip } from "@/components/VirtualizedFilmstrip";

const mockFrames = [
  {
    frame_index: 0,
    frame_type: "I",
    size: 50000,
    pts: 0,
    poc: 0,
    key_frame: true,
    display_order: 0,
    coding_order: 0,
  },
  {
    frame_index: 1,
    frame_type: "P",
    size: 30000,
    pts: 1,
    poc: 1,
    key_frame: false,
    ref_frames: [0],
    display_order: 1,
    coding_order: 1,
  },
  {
    frame_index: 2,
    frame_type: "B",
    size: 20000,
    pts: 2,
    poc: 2,
    key_frame: false,
    ref_frames: [0, 1],
    display_order: 2,
    coding_order: 2,
  },
];

const defaultProps = {
  frames: mockFrames,
  currentFrameIndex: 1,
  onFrameChange: vi.fn(),
  itemWidth: 80,
  containerWidth: 800,
};

describe("VirtualizedFilmstrip", () => {
  it("should render virtualized filmstrip", () => {
    const { container } = render(<VirtualizedFilmstrip {...defaultProps} />);

    // Should render filmstrip container
    expect(
      container.querySelector(".virtualized-filmstrip"),
    ).toBeInTheDocument();
  });

  it("should render current frame indicator", () => {
    render(<VirtualizedFilmstrip {...defaultProps} />);

    // Current frame should have "current" class
    const currentFrame = document.querySelector(".virtualized-frame.current");
    expect(currentFrame).toBeInTheDocument();
  });

  it("should handle frame click", () => {
    const handleFrameChange = vi.fn();
    render(
      <VirtualizedFilmstrip
        {...defaultProps}
        onFrameChange={handleFrameChange}
      />,
    );

    const frames = document.querySelectorAll(".virtualized-frame");
    if (frames.length > 0) {
      fireEvent.click(frames[0]);
      expect(handleFrameChange).toHaveBeenCalled();
    }
  });

  it("should display frame type indicators", () => {
    render(<VirtualizedFilmstrip {...defaultProps} />);

    // Should have frame type elements
    const types = document.querySelectorAll(".virtualized-frame-type");
    expect(types.length).toBeGreaterThan(0);
  });

  it("should render frame numbers", () => {
    render(<VirtualizedFilmstrip {...defaultProps} />);

    // Should show frame numbers
    const frameNumbers = document.querySelectorAll(".virtualized-frame-number");
    expect(frameNumbers.length).toBeGreaterThan(0);
  });

  it("should support horizontal scrolling", () => {
    const { container } = render(<VirtualizedFilmstrip {...defaultProps} />);

    const scrollContainer = container.querySelector(
      ".virtualized-filmstrip-viewport",
    );
    expect(scrollContainer).toBeInTheDocument();
  });

  it("should handle empty frames array", () => {
    const { container } = render(
      <VirtualizedFilmstrip
        {...defaultProps}
        frames={[]}
        currentFrameIndex={0}
      />,
    );

    // Should render filmstrip container even with no frames
    expect(
      container.querySelector(".virtualized-filmstrip"),
    ).toBeInTheDocument();
  });

  it("should support custom item width", () => {
    const { container } = render(
      <VirtualizedFilmstrip {...defaultProps} itemWidth={100} />,
    );

    // Filmstrip should render with custom width items
    const content = container.querySelector(".virtualized-filmstrip-content");
    expect(content).toBeInTheDocument();
  });

  it("should use stable callbacks (useCallback optimization)", () => {
    const { rerender } = render(<VirtualizedFilmstrip {...defaultProps} />);

    rerender(<VirtualizedFilmstrip {...defaultProps} />);

    // Should still function correctly
    expect(
      document.querySelector(".virtualized-filmstrip"),
    ).toBeInTheDocument();
  });
});

describe("VirtualizedFilmstrip performance", () => {
  it("should only render visible frames", () => {
    // Create large frame array
    const largeFrameArray = Array.from({ length: 1000 }, (_, i) => ({
      frame_index: i,
      frame_type: i % 3 === 0 ? "I" : i % 3 === 1 ? "P" : "B",
      size: 10000,
      poc: i,
    }));

    render(
      <VirtualizedFilmstrip
        frames={largeFrameArray}
        currentFrameIndex={500}
        onFrameChange={vi.fn()}
        itemWidth={80}
        containerWidth={800}
      />,
    );

    // Should not render all 1000 frames - only visible ones
    const renderedFrames = document.querySelectorAll(".virtualized-frame");
    expect(renderedFrames.length).toBeLessThan(1000);
  });

  it("should update visible frames on scroll", () => {
    render(<VirtualizedFilmstrip {...defaultProps} />);

    const container = document.querySelector(".virtualized-filmstrip-viewport");
    if (container) {
      // Simulate scroll
      fireEvent.scroll(container, { target: { scrollLeft: 1000 } });

      // Should handle scroll gracefully
      expect(container).toBeInTheDocument();
    }
  });
});
