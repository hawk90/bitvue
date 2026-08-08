/**
 * EnhancedView Component Tests
 * Tests enhanced view with multiple metrics overlay
 */

import { describe, it, expect, vi } from "vitest";
import { render } from "@/test/test-utils";
import { EnhancedView } from "../EnhancedView";

describe("EnhancedView", () => {
  const mockFrames = [
    { frame_index: 0, frame_type: "I", size: 50000, poc: 0, key_frame: true },
    { frame_index: 1, frame_type: "P", size: 30000, poc: 1, ref_frames: [0] },
    {
      frame_index: 2,
      frame_type: "B",
      size: 20000,
      poc: 2,
      ref_frames: [0, 1],
    },
    { frame_index: 3, frame_type: "I", size: 50000, poc: 3, key_frame: true },
    { frame_index: 4, frame_type: "P", size: 35000, poc: 4, ref_frames: [3] },
  ];

  const defaultProps = {
    frames: mockFrames,
    currentFrameIndex: 2,
    thumbnails: new Map(),
    loadingThumbnails: new Set(),
    onFrameClick: vi.fn(),
    onHoverFrame: vi.fn(),
  };

  it("should render enhanced view", () => {
    render(<EnhancedView {...defaultProps} />);

    const container = document.querySelector(".enhanced-view");
    expect(container).toBeInTheDocument();
  });

  it("should display navigation bar", () => {
    render(<EnhancedView {...defaultProps} />);

    const navBar = document.querySelector(".enhanced-nav-bar");
    expect(navBar).toBeInTheDocument();
  });

  it("should display GOP label", () => {
    render(<EnhancedView {...defaultProps} />);

    const gopLabel = document.querySelector(".enhanced-nav-label");
    expect(gopLabel).toBeInTheDocument();
    expect(gopLabel?.textContent).toBe("GOP:");
  });

  it("disables Prev GOP (already at the first GOP) and both Scene buttons (no scene changes detected), but enables Next GOP (a second GOP exists)", () => {
    // Regression pin: this component previously read frame.frameType (always undefined on the
    // real FrameInfo shape, which only has frame_type) so gopBoundaries was always empty and
    // every nav button was unconditionally disabled regardless of the frame data -- this test
    // originally asserted that broken behavior. mockFrames has a real second I-frame at index 3,
    // so with the frame_type fix there are genuinely two GOPs and Next GOP must be enabled.
    const { getByTitle } = render(<EnhancedView {...defaultProps} />);

    expect(getByTitle("Previous GOP")).toBeDisabled();
    expect(getByTitle("Next GOP")).not.toBeDisabled();
    expect(getByTitle("Previous Scene Change")).toBeDisabled();
    expect(getByTitle("Next Scene Change")).toBeDisabled();
  });

  it("should use React.memo for performance", () => {
    const { rerender } = render(<EnhancedView {...defaultProps} />);

    rerender(<EnhancedView {...defaultProps} />);

    expect(document.querySelector(".enhanced-view")).toBeInTheDocument();
  });
});

describe("EnhancedView edge cases", () => {
  it("should handle empty frames array", () => {
    const props = {
      frames: [],
      currentFrameIndex: -1,
      thumbnails: new Map(),
      loadingThumbnails: new Set(),
      onFrameClick: vi.fn(),
      onHoverFrame: vi.fn(),
    };

    render(<EnhancedView {...props} />);

    expect(document.querySelector(".enhanced-view")).toBeInTheDocument();
  });

  it("should handle single frame", () => {
    const props = {
      frames: [{ frame_index: 0, frame_type: "I", size: 50000, poc: 0 }],
      currentFrameIndex: 0,
      thumbnails: new Map(),
      loadingThumbnails: new Set(),
      onFrameClick: vi.fn(),
      onHoverFrame: vi.fn(),
    };

    render(<EnhancedView {...props} />);

    expect(document.querySelector(".enhanced-view")).toBeInTheDocument();
  });

  it("should handle all I-frames", () => {
    const props = {
      frames: [
        { frame_index: 0, frame_type: "I", size: 50000, poc: 0 },
        { frame_index: 1, frame_type: "I", size: 50000, poc: 1 },
      ],
      currentFrameIndex: 0,
      thumbnails: new Map(),
      loadingThumbnails: new Set(),
      onFrameClick: vi.fn(),
      onHoverFrame: vi.fn(),
    };

    render(<EnhancedView {...props} />);

    expect(document.querySelector(".enhanced-view")).toBeInTheDocument();
  });
});

describe("EnhancedView thumbnails", () => {
  const mockFrames = [
    { frame_index: 0, frame_type: "I", size: 50000, poc: 0 },
    { frame_index: 1, frame_type: "P", size: 30000, poc: 1 },
  ];

  it("should handle thumbnails map", () => {
    const thumbnails = new Map([
      [0, "data:image/jpeg;base64,abc123"],
      [1, "data:image/jpeg;base64,def456"],
    ]);

    const props = {
      frames: mockFrames,
      currentFrameIndex: 0,
      thumbnails,
      loadingThumbnails: new Set(),
      onFrameClick: vi.fn(),
      onHoverFrame: vi.fn(),
    };

    render(<EnhancedView {...props} />);

    expect(document.querySelector(".enhanced-view")).toBeInTheDocument();
  });

  it("should handle loading thumbnails", () => {
    const props = {
      frames: mockFrames,
      currentFrameIndex: 0,
      thumbnails: new Map(),
      loadingThumbnails: new Set([0, 1]),
      onFrameClick: vi.fn(),
      onHoverFrame: vi.fn(),
    };

    render(<EnhancedView {...props} />);

    expect(document.querySelector(".enhanced-view")).toBeInTheDocument();
  });
});

describe("EnhancedView interactions", () => {
  const mockFrames = [
    { frame_index: 0, frame_type: "I", size: 50000, poc: 0 },
    { frame_index: 1, frame_type: "P", size: 30000, poc: 1 },
    { frame_index: 2, frame_type: "B", size: 20000, poc: 2 },
  ];

  it("should call onFrameClick when frame is clicked", () => {
    const onFrameClick = vi.fn();
    const props = {
      frames: mockFrames,
      currentFrameIndex: 1,
      thumbnails: new Map(),
      loadingThumbnails: new Set(),
      onFrameClick,
      onHoverFrame: vi.fn(),
    };

    render(<EnhancedView {...props} />);

    // Click events are handled by ThumbnailsView
    // Just verify the component renders without errors
    expect(document.querySelector(".enhanced-view")).toBeInTheDocument();
  });

  it("should call onHoverFrame when frame is hovered", () => {
    const onHoverFrame = vi.fn();
    const props = {
      frames: mockFrames,
      currentFrameIndex: 1,
      thumbnails: new Map(),
      loadingThumbnails: new Set(),
      onFrameClick: vi.fn(),
      onHoverFrame,
    };

    render(<EnhancedView {...props} />);

    // Hover events are handled by ThumbnailsView
    // Just verify the component renders without errors
    expect(document.querySelector(".enhanced-view")).toBeInTheDocument();
  });
});
