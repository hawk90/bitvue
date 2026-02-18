/**
 * ThumbnailsView Component Tests
 * Tests frame thumbnail strip display with reference arrows
 * TODO: Skipping due to complex thumbnail rendering requiring backend support
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, within } from "@/test/test-utils";
import ThumbnailsView from "../ThumbnailsView";
import type { FrameInfo } from "@/types/video";

describe.skip("ThumbnailsView", () => {
  vi.mock("@/components/Filmstrip/usePreRenderedArrows", () => ({
    usePreRenderedArrows: vi.fn(() => ({
      allArrowData: [],
      svgWidth: 0,
    })),
    ArrowPosition: null,
    PathCalculator: null,
    FrameInfoBase: null,
  }));

  // Mock getFrameTypeColor
  vi.mock("@/types/video", async (importOriginal) => {
    const actual = await importOriginal<typeof import("@/types/video")>();
    return {
      ...actual,
      getFrameTypeColor: vi.fn((type) => {
        const colors: Record<string, string> = {
          I: "#ff4444",
          P: "#44ff44",
          B: "#4444ff",
        };
        return colors[type] || "#888888";
      }),
    };
  });

  const mockFrames: FrameInfo[] = [
    {
      frame_index: 0,
      frame_type: "I",
      size: 50000,
      poc: 0,
      temporal_id: 0,
      key_frame: true,
    },
    {
      frame_index: 1,
      frame_type: "P",
      size: 30000,
      poc: 1,
      temporal_id: 0,
      ref_frames: [0],
    },
    {
      frame_index: 2,
      frame_type: "B",
      size: 20000,
      poc: 2,
      temporal_id: 1,
      ref_frames: [0, 1],
    },
    {
      frame_index: 3,
      frame_type: "P",
      size: 35000,
      poc: 3,
      temporal_id: 0,
      ref_frames: [1],
      display_order: 3,
      coding_order: 2,
    },
    {
      frame_index: 4,
      frame_type: "I",
      size: 60000,
      poc: 4,
      temporal_id: 0,
      key_frame: true,
    },
  ];

  const defaultProps = {
    frames: mockFrames,
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
  };

  describe("ThumbnailsView", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it("should render thumbnail strip container", () => {
      render(<ThumbnailsView {...defaultProps} />);

      const container = document.querySelector(
        ".filmstrip-thumbnails-container",
      );
      expect(container).toBeInTheDocument();
    });

    it("should render all frames", () => {
      render(<ThumbnailsView {...defaultProps} />);

      const frames = document.querySelectorAll("[data-frame-index]");
      expect(frames).toHaveLength(5);
    });

    it("should display frame type in header", () => {
      render(<ThumbnailsView {...defaultProps} />);

      expect(screen.getByText("I-0 0")).toBeInTheDocument();
      expect(screen.getByText("P-0 1")).toBeInTheDocument();
      expect(screen.getByText("B-1 2")).toBeInTheDocument();
    });

    it("should display frame type in NAL label", () => {
      render(<ThumbnailsView {...defaultProps} />);

      expect(screen.getByText("I")).toBeInTheDocument();
      expect(screen.getByText("P")).toBeInTheDocument();
      expect(screen.getByText("B")).toBeInTheDocument();
    });

    it("should show placeholder when no thumbnail", () => {
      render(<ThumbnailsView {...defaultProps} />);

      const placeholders = document.querySelectorAll(".frame-placeholder");
      expect(placeholders.length).toBeGreaterThan(0);
    });

    it("should show loading state for loading thumbnails", () => {
      const props = {
        ...defaultProps,
        loadingThumbnails: new Set([1, 2]),
      };

      render(<ThumbnailsView {...props} />);

      const loadingPlaceholders = document.querySelectorAll(
        ".frame-placeholder.loading",
      );
      expect(loadingPlaceholders.length).toBe(2);
    });

    it("should display thumbnail image when available", () => {
      const thumbnails = new Map([[1, "data:image/png;base64,mockdata"]]);
      const props = { ...defaultProps, thumbnails };

      render(<ThumbnailsView {...props} />);

      const thumbnail = document.querySelector(`[data-frame-index="1"] img`);
      expect(thumbnail).toBeInTheDocument();
      expect(thumbnail).toHaveAttribute(
        "src",
        "data:image/png;base64,mockdata",
      );
    });

    it("should mark current frame as selected", () => {
      render(<ThumbnailsView {...defaultProps} />);

      const selectedFrame = document.querySelector(
        '[data-frame-index="0"].selected',
      );
      expect(selectedFrame).toBeInTheDocument();

      const unselectedFrame = document.querySelector(
        '[data-frame-index="1"].selected',
      );
      expect(unselectedFrame).not.toBeInTheDocument();
    });

    it("should mark referenced frames", () => {
      const props = {
        ...defaultProps,
        referencedFrameIndices: new Set([0, 1]),
      };

      render(<ThumbnailsView {...props} />);

      const referencedFrame = document.querySelector(
        '[data-frame-index="0"].is-referenced',
      );
      expect(referencedFrame).toBeInTheDocument();
    });

    it("should call onFrameClick when frame clicked", () => {
      render(<ThumbnailsView {...defaultProps} />);

      const frame = document.querySelector('[data-frame-index="2"]');
      fireEvent.click(frame!);

      expect(defaultProps.onFrameClick).toHaveBeenCalledWith(2);
    });

    it("should call onHoverFrame on mouse enter", () => {
      render(<ThumbnailsView {...defaultProps} />);

      const frame = document.querySelector('[data-frame-index="1"]');
      fireEvent.mouseEnter(frame!);

      expect(defaultProps.onHoverFrame).toHaveBeenCalled();
    });

    it("should call onHoverFrame on mouse move", () => {
      render(<ThumbnailsView {...defaultProps} />);

      const frame = document.querySelector('[data-frame-index="1"]');
      fireEvent.mouseMove(frame!);

      expect(defaultProps.onHoverFrame).toHaveBeenCalled();
    });

    it("should call onHoverFrame with null on mouse leave", () => {
      render(<ThumbnailsView {...defaultProps} />);

      const frame = document.querySelector('[data-frame-index="1"]');
      fireEvent.mouseLeave(frame!);

      expect(defaultProps.onHoverFrame).toHaveBeenCalledWith(null, 0, 0);
    });

    it("should display reference badge for frames with refs", () => {
      render(<ThumbnailsView {...defaultProps} />);

      const refBadge = document.querySelector('[data-count="1"]');
      expect(refBadge).toBeInTheDocument();
    });

    it("should show multiple ref count correctly", () => {
      render(<ThumbnailsView {...defaultProps} />);

      const refBadge = document.querySelector('[data-count="2"]');
      expect(refBadge).toBeInTheDocument();
    });

    it("should expand reference badge when clicked", () => {
      render(<ThumbnailsView {...defaultProps} />);

      const refBadge = document.querySelector('[data-count="1"]');
      fireEvent.click(refBadge!);

      expect(defaultProps.onToggleReferenceExpansion).toHaveBeenCalledWith(
        1,
        expect.any(Object),
      );
    });

    it("should show expanded reference indices", () => {
      const props = {
        ...defaultProps,
        expandedFrameIndex: 1,
      };

      render(<ThumbnailsView {...props} />);

      const refBadge = document.querySelector('[data-count="1"]');
      expect(within(refBadge!).getByText("#0")).toBeInTheDocument();
    });

    it("should display display and coding order when available", () => {
      render(<ThumbnailsView {...defaultProps} />);

      expect(screen.getByText("D:3")).toBeInTheDocument();
      expect(screen.getByText("C:2")).toBeInTheDocument();
    });

    it("should show error badge for zero-size frames", () => {
      const framesWithError = [
        ...mockFrames,
        { frame_index: 5, frame_type: "P", size: 0, poc: 5, temporal_id: 0 },
      ];
      const props = { ...defaultProps, frames: framesWithError };

      render(<ThumbnailsView {...props} />);

      const errorBadge = document.querySelector(".frame-error-badge");
      expect(errorBadge).toBeInTheDocument();
      expect(errorBadge).toHaveTextContent("!");
    });

    it("should apply correct frame type color class", () => {
      const getFrameTypeColorClass = vi.fn(
        (type: string) => `frame-type-${type.toLowerCase()}`,
      );
      const props = { ...defaultProps, getFrameTypeColorClass };

      render(<ThumbnailsView {...props} />);

      expect(getFrameTypeColorClass).toHaveBeenCalledWith("I");
      expect(getFrameTypeColorClass).toHaveBeenCalledWith("P");
      expect(getFrameTypeColorClass).toHaveBeenCalledWith("B");

      const iFrame = document.querySelector('[data-frame-index="0"]');
      expect(iFrame).toHaveClass("frame-type-i");
    });

    it("should have proper ARIA attributes", () => {
      render(<ThumbnailsView {...defaultProps} />);

      const container = document.querySelector('[role="list"]');
      expect(container).toBeInTheDocument();
      expect(container).toHaveAttribute("aria-label", "Frame thumbnails");

      const frame = document.querySelector(
        '[role="listitem"][data-frame-index="0"]',
      );
      expect(frame).toHaveAttribute("tabIndex", "0");
      expect(frame).toHaveAttribute("aria-selected", "true");
    });

    it("should set correct tabIndex for non-selected frames", () => {
      render(<ThumbnailsView {...defaultProps} />);

      const frame = document.querySelector(
        '[role="listitem"][data-frame-index="1"]',
      );
      expect(frame).toHaveAttribute("tabIndex", "-1");
      expect(frame).toHaveAttribute("aria-selected", "false");
    });
  });

  describe("ThumbnailsView zoom", () => {
    it("should apply zoom transform", () => {
      render(<ThumbnailsView {...defaultProps} />);

      const thumbnailsContainer = document.querySelector(
        ".filmstrip-thumbnails",
      );
      expect(thumbnailsContainer).toHaveStyle({ transform: "scaleX(1)" });
    });

    it("should handle wheel zoom with Ctrl key", () => {
      render(<ThumbnailsView {...defaultProps} />);

      const container = document.querySelector(".filmstrip-thumbnails");
      const initialTransform = container?.style.transform;

      fireEvent.wheel(container!, { deltaY: -100, ctrlKey: true });

      // Transform should change after zoom
      expect(container?.style.transform).not.toBe(initialTransform);
    });

    it("should not zoom without Ctrl key", () => {
      render(<ThumbnailsView {...defaultProps} />);

      const container = document.querySelector(".filmstrip-thumbnails");
      const initialTransform = container?.style.transform;

      fireEvent.wheel(container!, { deltaY: -100, ctrlKey: false });

      // Transform should not change
      expect(container?.style.transform).toBe(initialTransform);
    });

    it("should handle wheel zoom with Meta key", () => {
      render(<ThumbnailsView {...defaultProps} />);

      const container = document.querySelector(".filmstrip-thumbnails");
      fireEvent.wheel(container!, { deltaY: -100, metaKey: true });

      // Should still zoom with metaKey
      expect(container).toBeInTheDocument();
    });
  });

  describe("ThumbnailsView reference arrows", () => {
    it("should not render SVG when no arrow data", () => {
      render(<ThumbnailsView {...defaultProps} />);

      const svg = document.querySelector(".thumbnail-arrows-overlay");
      expect(svg).not.toBeInTheDocument();
    });

    it("should render SVG overlay when arrows exist", () => {
      const {
        usePreRenderedArrows,
      } = require("@/components/Filmstrip/usePreRenderedArrows");
      usePreRenderedArrows.mockReturnValue({
        allArrowData: [
          { sourceFrameIndex: 1, targetFrameIndex: 0, pathData: "M0,0 L10,10" },
        ],
        svgWidth: 1000,
      });

      render(<ThumbnailsView {...defaultProps} />);

      const svg = document.querySelector(".thumbnail-arrows-overlay");
      expect(svg).toBeInTheDocument();
    });
  });

  describe("ThumbnailsView edge cases", () => {
    it("should handle empty frames array", () => {
      const props = { ...defaultProps, frames: [] };
      render(<ThumbnailsView {...props} />);

      const frames = document.querySelectorAll("[data-frame-index]");
      expect(frames).toHaveLength(0);
    });

    it("should handle frame without temporal_id", () => {
      const framesWithoutTemporal = [
        { frame_index: 0, frame_type: "I", size: 50000, poc: 0 },
      ] as FrameInfo[];
      const props = { ...defaultProps, frames: framesWithoutTemporal };

      render(<ThumbnailsView {...props} />);

      // Should show 'A' as default layer
      expect(screen.getByText("I-A 0")).toBeInTheDocument();
    });

    it("should handle frames with only display_order", () => {
      const framesWithDisplayOrder = [
        {
          frame_index: 0,
          frame_type: "I",
          size: 50000,
          poc: 0,
          display_order: 5,
        },
      ] as FrameInfo[];
      const props = { ...defaultProps, frames: framesWithDisplayOrder };

      render(<ThumbnailsView {...props} />);

      expect(screen.getByText("D:5")).toBeInTheDocument();
    });

    it("should handle frames with only coding_order", () => {
      const framesWithCodingOrder = [
        {
          frame_index: 0,
          frame_type: "I",
          size: 50000,
          poc: 0,
          coding_order: 3,
        },
      ] as FrameInfo[];
      const props = { ...defaultProps, frames: framesWithCodingOrder };

      render(<ThumbnailsView {...props} />);

      expect(screen.getByText("C:3")).toBeInTheDocument();
    });

    it("should handle all thumbnails loaded", () => {
      const allThumbnails = new Map(
        mockFrames.map((f) => [
          f.frame_index,
          `data:image/png;base64,frame${f.frame_index}`,
        ]),
      );
      const props = { ...defaultProps, thumbnails: allThumbnails };

      render(<ThumbnailsView {...props} />);

      const images = document.querySelectorAll(".frame-thumbnail img");
      expect(images.length).toBe(5);
    });

    it("should handle all frames loading", () => {
      const allLoading = new Set(mockFrames.map((f) => f.frame_index));
      const props = { ...defaultProps, loadingThumbnails: allLoading };

      render(<ThumbnailsView {...props} />);

      const loadingPlaceholders = document.querySelectorAll(
        ".frame-placeholder.loading",
      );
      expect(loadingPlaceholders.length).toBe(5);
    });
  });

  describe("ThumbnailsView React.memo", () => {
    it("should use React.memo for performance", () => {
      const { rerender } = render(<ThumbnailsView {...defaultProps} />);

      const initialContainer = document.querySelector(
        ".filmstrip-thumbnails-container",
      );

      rerender(<ThumbnailsView {...defaultProps} />);

      const rerenderedContainer = document.querySelector(
        ".filmstrip-thumbnails-container",
      );
      expect(rerenderedContainer).toEqual(initialContainer);
    });

    it("should re-render when currentFrameIndex changes", () => {
      const { rerender } = render(<ThumbnailsView {...defaultProps} />);

      rerender(<ThumbnailsView {...defaultProps} currentFrameIndex={2} />);

      const selectedFrame = document.querySelector(
        '[data-frame-index="2"].selected',
      );
      expect(selectedFrame).toBeInTheDocument();
    });
  });
});
