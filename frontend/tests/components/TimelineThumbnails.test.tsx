/**
 * TimelineThumbnails Component Tests
 * Tests frame bar strip with drag interaction
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  render,
  screen,
  fireEvent,
  renderWithoutProviders,
} from "@/test/test-utils";
import { TimelineThumbnails } from "@/components/TimelineThumbnails";

// Mock Tauri invoke to prevent actual calls during tests
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(() => Promise.resolve([])),
}));

// Simple wrapper for tests that don't need providers
function TestWrapper({ children }: { children: React.ReactNode }) {
  return <div data-testid="test-wrapper">{children}</div>;
}

const mockFrames = [
  { frame_index: 0, frame_type: "I", size: 50000, poc: 0, key_frame: true },
  { frame_index: 1, frame_type: "P", size: 30000, poc: 1 },
  { frame_index: 2, frame_type: "B", size: 20000, poc: 2 },
  { frame_index: 3, frame_type: "P", size: 35000, poc: 3 },
  { frame_index: 4, frame_type: "B", size: 25000, poc: 4 },
];

const defaultProps = {
  frames: mockFrames,
  highlightedFrameIndex: 0,
  onMouseDown: vi.fn(),
  onMouseMove: vi.fn(),
  onMouseLeave: vi.fn(),
  onKeyDown: vi.fn(),
};

describe("TimelineThumbnails", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should render timeline thumbnails container", () => {
    renderWithoutProviders(<TimelineThumbnails {...defaultProps} />);

    const container = document.querySelector(".timeline-thumbnails");
    expect(container).toBeInTheDocument();
  });

  it("should render all frame thumbnails", () => {
    renderWithoutProviders(<TimelineThumbnails {...defaultProps} />);

    const thumbnails = document.querySelectorAll(".timeline-thumb");
    expect(thumbnails).toHaveLength(5);
  });

  it("should mark highlighted frame as selected", () => {
    renderWithoutProviders(<TimelineThumbnails {...defaultProps} />);

    const selectedThumb = document.querySelector(
      '[data-frame-index="0"].selected',
    );
    expect(selectedThumb).toBeInTheDocument();
  });

  it("should apply correct frame type class for I frames", () => {
    renderWithoutProviders(<TimelineThumbnails {...defaultProps} />);

    const iFrame = document.querySelector('[data-frame-index="0"]');
    expect(iFrame).toHaveClass("timeline-bar-i");
  });

  it("should apply correct frame type class for P frames", () => {
    renderWithoutProviders(<TimelineThumbnails {...defaultProps} />);

    const pFrame = document.querySelector('[data-frame-index="1"]');
    expect(pFrame).toHaveClass("timeline-bar-p");
  });

  it("should apply correct frame type class for B frames", () => {
    renderWithoutProviders(<TimelineThumbnails {...defaultProps} />);

    const bFrame = document.querySelector('[data-frame-index="2"]');
    expect(bFrame).toHaveClass("timeline-bar-b");
  });

  it("should handle KEY frame type", () => {
    const framesWithKey = [
      {
        frame_index: 0,
        frame_type: "KEY",
        size: 50000,
        poc: 0,
        key_frame: true,
      },
    ];
    const props = { ...defaultProps, frames: framesWithKey };

    renderWithoutProviders(<TimelineThumbnails {...props} />);

    const keyFrame = document.querySelector('[data-frame-index="0"]');
    expect(keyFrame).toHaveClass("timeline-bar-i");
  });

  it("should handle INTER frame type", () => {
    const framesWithInter = [
      { frame_index: 0, frame_type: "INTER", size: 50000, poc: 0 },
    ];
    const props = { ...defaultProps, frames: framesWithInter };

    renderWithoutProviders(<TimelineThumbnails {...props} />);

    const interFrame = document.querySelector('[data-frame-index="0"]');
    expect(interFrame).toHaveClass("timeline-bar-p");
  });

  it("should apply unknown class for unhandled frame types", () => {
    const framesWithUnknown = [
      { frame_index: 0, frame_type: "UNKNOWN", size: 50000, poc: 0 },
    ];
    const props = { ...defaultProps, frames: framesWithUnknown };

    renderWithoutProviders(<TimelineThumbnails {...props} />);

    const unknownFrame = document.querySelector('[data-frame-index="0"]');
    expect(unknownFrame).toHaveClass("timeline-bar-unknown");
  });

  it("should call onMouseDown when mouse down", () => {
    renderWithoutProviders(<TimelineThumbnails {...defaultProps} />);

    const container = document.querySelector(".timeline-thumbnails");
    fireEvent.mouseDown(container!);

    expect(defaultProps.onMouseDown).toHaveBeenCalled();
  });

  it("should call onMouseMove when mouse move", () => {
    renderWithoutProviders(<TimelineThumbnails {...defaultProps} />);

    const container = document.querySelector(".timeline-thumbnails");
    fireEvent.mouseMove(container!);

    expect(defaultProps.onMouseMove).toHaveBeenCalled();
  });

  it("should call onMouseLeave when mouse leaves", () => {
    renderWithoutProviders(<TimelineThumbnails {...defaultProps} />);

    const container = document.querySelector(".timeline-thumbnails");
    fireEvent.mouseLeave(container!);

    expect(defaultProps.onMouseLeave).toHaveBeenCalled();
  });

  it("should call onKeyDown when key pressed", () => {
    renderWithoutProviders(<TimelineThumbnails {...defaultProps} />);

    const container = document.querySelector(".timeline-thumbnails");
    fireEvent.keyDown(container!, { key: "ArrowLeft" });

    expect(defaultProps.onKeyDown).toHaveBeenCalled();
  });

  it("should have proper ARIA attributes", () => {
    renderWithoutProviders(<TimelineThumbnails {...defaultProps} />);

    const container = document.querySelector(".timeline-thumbnails");
    expect(container).toHaveAttribute("role", "slider");
    expect(container).toHaveAttribute("aria-label", "Frame position");
    expect(container).toHaveAttribute("aria-valuemin", "0");
    expect(container).toHaveAttribute("aria-valuemax", "4");
    expect(container).toHaveAttribute("aria-valuenow", "0");
    expect(container).toHaveAttribute("aria-valuetext", "Frame 0 of 5");
    expect(container).toHaveAttribute("tabIndex", "0");
  });

  it("should have title with usage hint", () => {
    renderWithoutProviders(<TimelineThumbnails {...defaultProps} />);

    const container = document.querySelector(".timeline-thumbnails");
    expect(container).toHaveAttribute("title", "Click to seek, drag to scrub");
  });

  it("should update aria-valuenow when highlighted frame changes", () => {
    const { rerender } = renderWithoutProviders(
      <TimelineThumbnails {...defaultProps} />,
    );

    rerender(
      <TimelineThumbnails {...defaultProps} highlightedFrameIndex={2} />,
    );

    const container = document.querySelector(".timeline-thumbnails");
    expect(container).toHaveAttribute("aria-valuenow", "2");
    expect(container).toHaveAttribute("aria-valuetext", "Frame 2 of 5");
  });

  it("should set aria-current on selected thumbnail", () => {
    renderWithoutProviders(<TimelineThumbnails {...defaultProps} />);

    const selectedThumb = document.querySelector('[data-frame-index="0"]');
    expect(selectedThumb).toHaveAttribute("aria-current", "true");

    const unselectedThumb = document.querySelector('[data-frame-index="1"]');
    expect(unselectedThumb).not.toHaveAttribute("aria-current");
  });

  it("should have data-frame-index on each thumbnail", () => {
    renderWithoutProviders(<TimelineThumbnails {...defaultProps} />);

    expect(
      document.querySelector('[data-frame-index="0"]'),
    ).toBeInTheDocument();
    expect(
      document.querySelector('[data-frame-index="1"]'),
    ).toBeInTheDocument();
    expect(
      document.querySelector('[data-frame-index="2"]'),
    ).toBeInTheDocument();
    expect(
      document.querySelector('[data-frame-index="3"]'),
    ).toBeInTheDocument();
    expect(
      document.querySelector('[data-frame-index="4"]'),
    ).toBeInTheDocument();
  });

  it("should have tooltip with frame info", () => {
    renderWithoutProviders(<TimelineThumbnails {...defaultProps} />);

    const thumb = document.querySelector('[data-frame-index="0"]');
    expect(thumb).toHaveAttribute("title", "Frame 0: I");
  });

  it("should have aria-label on each thumbnail", () => {
    renderWithoutProviders(<TimelineThumbnails {...defaultProps} />);

    const thumb = document.querySelector('[data-frame-index="0"]');
    expect(thumb).toHaveAttribute("aria-label", "Frame 0, type I");
  });
});

describe("TimelineThumbnails refs", () => {
  it("should call onFrameRefsChange when refs change", () => {
    const onFrameRefsChange = vi.fn();

    renderWithoutProviders(
      <TimelineThumbnails
        {...defaultProps}
        onFrameRefsChange={onFrameRefsChange}
      />,
    );

    expect(onFrameRefsChange).toHaveBeenCalled();
    expect(onFrameRefsChange).toHaveBeenCalledWith(expect.any(Array));
  });

  it("should update refs when frames length changes", () => {
    const onFrameRefsChange = vi.fn();

    const { rerender } = renderWithoutProviders(
      <TimelineThumbnails
        {...defaultProps}
        onFrameRefsChange={onFrameRefsChange}
      />,
    );

    rerender(
      <TimelineThumbnails
        {...defaultProps}
        frames={[mockFrames[0]]}
        onFrameRefsChange={onFrameRefsChange}
      />,
    );

    expect(onFrameRefsChange).toHaveBeenCalled();
  });
});

describe("TimelineThumbnails edge cases", () => {
  it("should handle empty frames array", () => {
    const props = { ...defaultProps, frames: [] };

    renderWithoutProviders(<TimelineThumbnails {...props} />);

    const thumbnails = document.querySelectorAll(".timeline-thumb");
    expect(thumbnails).toHaveLength(0);

    const container = document.querySelector(".timeline-thumbnails");
    expect(container).toHaveAttribute("aria-valuemax", "-1");
    expect(container).toHaveAttribute("aria-valuenow", "0");
  });

  it("should handle single frame", () => {
    const props = { ...defaultProps, frames: [mockFrames[0]] };

    renderWithoutProviders(<TimelineThumbnails {...props} />);

    const thumbnails = document.querySelectorAll(".timeline-thumb");
    expect(thumbnails).toHaveLength(1);

    const container = document.querySelector(".timeline-thumbnails");
    expect(container).toHaveAttribute("aria-valuemax", "0");
  });

  it("should handle highlighted frame out of range", () => {
    const props = { ...defaultProps, highlightedFrameIndex: 99 };

    renderWithoutProviders(<TimelineThumbnails {...props} />);

    const selectedThumb = document.querySelector(".selected");
    expect(selectedThumb).not.toBeInTheDocument();
  });

  it("should handle negative highlighted frame index", () => {
    const props = { ...defaultProps, highlightedFrameIndex: -1 };

    renderWithoutProviders(<TimelineThumbnails {...props} />);

    const selectedThumb = document.querySelector(".selected");
    expect(selectedThumb).not.toBeInTheDocument();
  });

  it("should handle lowercase frame types", () => {
    const framesWithLowercase = [
      { frame_index: 0, frame_type: "i", size: 50000, poc: 0 },
      { frame_index: 1, frame_type: "p", size: 30000, poc: 1 },
      { frame_index: 2, frame_type: "b", size: 20000, poc: 2 },
    ];
    const props = { ...defaultProps, frames: framesWithLowercase };

    renderWithoutProviders(<TimelineThumbnails {...props} />);

    expect(document.querySelector('[data-frame-index="0"]')).toHaveClass(
      "timeline-bar-i",
    );
    expect(document.querySelector('[data-frame-index="1"]')).toHaveClass(
      "timeline-bar-p",
    );
    expect(document.querySelector('[data-frame-index="2"]')).toHaveClass(
      "timeline-bar-b",
    );
  });

  it("should handle mixed case frame types", () => {
    const framesWithMixedCase = [
      { frame_index: 0, frame_type: "I", size: 50000, poc: 0 },
      { frame_index: 1, frame_type: "b", size: 20000, poc: 1 },
      { frame_index: 2, frame_type: "P", size: 30000, poc: 2 },
    ];
    const props = { ...defaultProps, frames: framesWithMixedCase };

    renderWithoutProviders(<TimelineThumbnails {...props} />);

    expect(document.querySelector('[data-frame-index="0"]')).toHaveClass(
      "timeline-bar-i",
    );
    expect(document.querySelector('[data-frame-index="1"]')).toHaveClass(
      "timeline-bar-b",
    );
    expect(document.querySelector('[data-frame-index="2"]')).toHaveClass(
      "timeline-bar-p",
    );
  });

  it("should handle frame types starting with B", () => {
    const framesWithBTypes = [
      { frame_index: 0, frame_type: "B", size: 20000, poc: 0 },
      { frame_index: 1, frame_type: "b", size: 20000, poc: 1 },
      { frame_index: 2, frame_type: "BLA", size: 20000, poc: 2 },
    ];
    const props = { ...defaultProps, frames: framesWithBTypes };

    renderWithoutProviders(<TimelineThumbnails {...props} />);

    expect(document.querySelector('[data-frame-index="0"]')).toHaveClass(
      "timeline-bar-b",
    );
    expect(document.querySelector('[data-frame-index="1"]')).toHaveClass(
      "timeline-bar-b",
    );
    expect(document.querySelector('[data-frame-index="2"]')).toHaveClass(
      "timeline-bar-b",
    );
  });
});

describe("TimelineThumbnails keyboard interaction", () => {
  it("should handle ArrowLeft key", () => {
    renderWithoutProviders(<TimelineThumbnails {...defaultProps} />);

    const container = document.querySelector(".timeline-thumbnails");
    fireEvent.keyDown(container!, { key: "ArrowLeft" });

    expect(defaultProps.onKeyDown).toHaveBeenCalledWith(
      expect.objectContaining({ key: "ArrowLeft" }),
    );
  });

  it("should handle ArrowRight key", () => {
    renderWithoutProviders(<TimelineThumbnails {...defaultProps} />);

    const container = document.querySelector(".timeline-thumbnails");
    fireEvent.keyDown(container!, { key: "ArrowRight" });

    expect(defaultProps.onKeyDown).toHaveBeenCalledWith(
      expect.objectContaining({ key: "ArrowRight" }),
    );
  });

  it("should handle Home key", () => {
    renderWithoutProviders(<TimelineThumbnails {...defaultProps} />);

    const container = document.querySelector(".timeline-thumbnails");
    fireEvent.keyDown(container!, { key: "Home" });

    expect(defaultProps.onKeyDown).toHaveBeenCalledWith(
      expect.objectContaining({ key: "Home" }),
    );
  });

  it("should handle End key", () => {
    renderWithoutProviders(<TimelineThumbnails {...defaultProps} />);

    const container = document.querySelector(".timeline-thumbnails");
    fireEvent.keyDown(container!, { key: "End" });

    expect(defaultProps.onKeyDown).toHaveBeenCalledWith(
      expect.objectContaining({ key: "End" }),
    );
  });

  it("should handle PageUp key", () => {
    renderWithoutProviders(<TimelineThumbnails {...defaultProps} />);

    const container = document.querySelector(".timeline-thumbnails");
    fireEvent.keyDown(container!, { key: "PageUp" });

    expect(defaultProps.onKeyDown).toHaveBeenCalledWith(
      expect.objectContaining({ key: "PageUp" }),
    );
  });

  it("should handle PageDown key", () => {
    renderWithoutProviders(<TimelineThumbnails {...defaultProps} />);

    const container = document.querySelector(".timeline-thumbnails");
    fireEvent.keyDown(container!, { key: "PageDown" });

    expect(defaultProps.onKeyDown).toHaveBeenCalledWith(
      expect.objectContaining({ key: "PageDown" }),
    );
  });
});

describe("TimelineThumbnails mouse interaction", () => {
  it("should handle mouse down at specific position", () => {
    renderWithoutProviders(<TimelineThumbnails {...defaultProps} />);

    const container = document.querySelector(".timeline-thumbnails");
    fireEvent.mouseDown(container!, { clientX: 100, clientY: 50 });

    expect(defaultProps.onMouseDown).toHaveBeenCalledWith(
      expect.objectContaining({ clientX: 100, clientY: 50 }),
    );
  });

  it("should handle mouse move at specific position", () => {
    renderWithoutProviders(<TimelineThumbnails {...defaultProps} />);

    const container = document.querySelector(".timeline-thumbnails");
    fireEvent.mouseMove(container!, { clientX: 150, clientY: 50 });

    expect(defaultProps.onMouseMove).toHaveBeenCalledWith(
      expect.objectContaining({ clientX: 150, clientY: 50 }),
    );
  });

  it("should handle mouse leave event", () => {
    renderWithoutProviders(<TimelineThumbnails {...defaultProps} />);

    const container = document.querySelector(".timeline-thumbnails");
    fireEvent.mouseLeave(container!);

    expect(defaultProps.onMouseLeave).toHaveBeenCalledWith();
  });
});

describe("TimelineThumbnails forwardRef", () => {
  it("should forward ref to container element", () => {
    const ref = vi.fn();

    renderWithoutProviders(<TimelineThumbnails {...defaultProps} ref={ref} />);

    expect(ref).toHaveBeenCalled();
    const container = document.querySelector(".timeline-thumbnails");
    expect(ref).toHaveBeenCalledWith(container);
  });
});
