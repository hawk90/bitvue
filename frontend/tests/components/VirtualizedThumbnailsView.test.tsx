/**
 * VirtualizedThumbnailsView Component Tests
 * Covers the visible-window thumbnail loading and reference-arrow overlay added to fix:
 * (1) thumbnails freezing after the initial batch on large (>=200 frame) streams, and
 * (2) reference arrows never rendering at all in the virtualized filmstrip path.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render } from "@/test/test-utils";
import VirtualizedThumbnailsView from "../VirtualizedThumbnailsView";
import type { FrameInfo } from "@/types/video";
import { usePreRenderedArrows } from "@/components/usePreRenderedArrows";

vi.mock("@/components/usePreRenderedArrows", () => ({
  usePreRenderedArrows: vi.fn(() => ({
    allArrowData: [],
    svgWidth: 0,
  })),
}));

function makeFrames(count: number): FrameInfo[] {
  return Array.from({ length: count }, (_, i) => ({
    frame_index: i,
    frame_type: i === 0 ? "I" : "P",
    size: 1000 + i,
    poc: i,
    temporal_id: 0,
    key_frame: i === 0,
    ref_frames: i === 0 ? [] : [i - 1],
  }));
}

const defaultProps = {
  frames: makeFrames(300),
  currentFrameIndex: 0,
  thumbnails: new Map<number, string>(),
  loadingThumbnails: new Set<number>(),
  referencedFrameIndices: new Set<number>(),
  expandedFrameIndex: null,
  onFrameClick: vi.fn(),
  onToggleReferenceExpansion: vi.fn(),
  onHoverFrame: vi.fn(),
  getFrameTypeColorClass: vi.fn(
    (type: string) => `frame-type-${type.toLowerCase()}`,
  ),
  loadThumbnails: vi.fn(),
};

describe("VirtualizedThumbnailsView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(usePreRenderedArrows).mockReturnValue({
      allArrowData: [],
      svgWidth: 0,
    });
  });

  it("requests thumbnails for the currently visible window on mount", () => {
    render(
      <VirtualizedThumbnailsView {...defaultProps} currentFrameIndex={0} />,
    );

    expect(defaultProps.loadThumbnails).toHaveBeenCalled();
    const requested = defaultProps.loadThumbnails.mock.calls[0][0] as number[];
    // Window is [max(0, 0-50), min(300, 0+50+1)) = [0, 51)
    expect(requested).toContain(0);
    expect(requested).toContain(50);
    expect(requested).not.toContain(51);
  });

  it("requests thumbnails for a shifted window when the selection moves past the initial batch", () => {
    render(
      <VirtualizedThumbnailsView {...defaultProps} currentFrameIndex={120} />,
    );

    const requested = defaultProps.loadThumbnails.mock.calls.at(
      -1,
    )![0] as number[];
    // Window is [120-50, 120+50+1) = [70, 171) -- well past the old 50-frame batch cap.
    expect(requested).toContain(70);
    expect(requested).toContain(120);
    expect(requested).toContain(170);
    expect(requested).not.toContain(69);
    expect(requested).not.toContain(171);
  });

  it("re-requests thumbnails when the visible window changes across a rerender", () => {
    const { rerender } = render(
      <VirtualizedThumbnailsView {...defaultProps} currentFrameIndex={0} />,
    );
    defaultProps.loadThumbnails.mockClear();

    rerender(
      <VirtualizedThumbnailsView {...defaultProps} currentFrameIndex={200} />,
    );

    expect(defaultProps.loadThumbnails).toHaveBeenCalled();
    const requested = defaultProps.loadThumbnails.mock.calls.at(
      -1,
    )![0] as number[];
    expect(requested).toContain(200);
  });

  it("renders the reference-arrow SVG overlay when arrow data is present", () => {
    vi.mocked(usePreRenderedArrows).mockReturnValue({
      allArrowData: [
        {
          sourceFrameIndex: 1,
          targetFrameIndex: 0,
          slotIndex: 0,
          label: "L0",
          color: "#44ff44",
          pathData: "M0,0 L10,10",
          sourceX: 10,
          sourceY: 10,
          labelY: 10,
        },
      ],
      svgWidth: 1000,
    });

    render(
      <VirtualizedThumbnailsView {...defaultProps} currentFrameIndex={1} />,
    );

    expect(
      document.querySelector(".thumbnail-arrows-overlay"),
    ).toBeInTheDocument();
    const path = document.querySelector(".thumbnail-arrows-overlay > g > path");
    expect(path).toHaveAttribute("visibility", "visible");
  });

  it("does not render the arrow overlay when there is no arrow data", () => {
    render(<VirtualizedThumbnailsView {...defaultProps} />);

    expect(
      document.querySelector(".thumbnail-arrows-overlay"),
    ).not.toBeInTheDocument();
  });

  it("passes a recalcKey derived from the visible window so arrows recompute on scroll", () => {
    render(
      <VirtualizedThumbnailsView {...defaultProps} currentFrameIndex={0} />,
    );

    const firstCallArgs = vi.mocked(usePreRenderedArrows).mock.calls[0][0];
    expect(firstCallArgs.recalcKey).toBe("0-51");
  });
});
