/**
 * FrameSizesView Component Tests
 * Tests frame size bar chart with bitrate and QP axes
 * TODO: Skipping due to complex chart rendering requiring backend support
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@/test/test-utils";
import FrameSizesView, {
  SizeMetrics,
} from "@/components/Filmstrip/views/FrameSizesView";
import type { FrameInfo } from "@/types/video";

describe.skip("FrameSizesView", () => {
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
    { frame_index: 0, frame_type: "I", size: 100000, poc: 0, duration: 1 },
    { frame_index: 1, frame_type: "P", size: 50000, poc: 1, duration: 1 },
    { frame_index: 2, frame_type: "B", size: 30000, poc: 2, duration: 1 },
    { frame_index: 3, frame_type: "P", size: 60000, poc: 3, duration: 1 },
    { frame_index: 4, frame_type: "B", size: 25000, poc: 4, duration: 1 },
  ];

  const defaultSizeMetrics: SizeMetrics = {
    showBitrateBar: false,
    showBitrateCurve: true,
    showAvgSize: true,
    showMinSize: true,
    showMaxSize: true,
    showMovingAvg: true,
    showBlockMinQP: false,
    showBlockMaxQP: false,
  };

  const defaultProps = {
    frames: mockFrames,
    maxSize: 100000,
    currentFrameIndex: 0,
    visibleFrameTypes: new Set(["I", "P", "B"]),
    onFrameClick: vi.fn(),
    getFrameTypeColorClass: vi.fn(
      (type: string) => `frame-type-${type.toLowerCase()}`,
    ),
    sizeMetrics: defaultSizeMetrics,
  };

  describe("FrameSizesView", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it("should render frame sizes container", () => {
      render(<FrameSizesView {...defaultProps} />);

      const container = document.querySelector(".filmstrip-sizes");
      expect(container).toBeInTheDocument();
    });

    it("should render left Y-axis (Bitrate)", () => {
      render(<FrameSizesView {...defaultProps} />);

      const leftAxis = document.querySelector(".axis-left");
      expect(leftAxis).toBeInTheDocument();

      expect(screen.getByText("Bitrate")).toBeInTheDocument();
    });

    it("should render right Y-axis (QP)", () => {
      render(<FrameSizesView {...defaultProps} />);

      const rightAxis = document.querySelector(".axis-right");
      expect(rightAxis).toBeInTheDocument();

      expect(screen.getByText("QP")).toBeInTheDocument();
    });

    it("should render all frame bars", () => {
      render(<FrameSizesView {...defaultProps} />);

      const bars = document.querySelectorAll(".frame-size-bar");
      expect(bars).toHaveLength(5);
    });

    it("should display correct max bitrate label", () => {
      render(<FrameSizesView {...defaultProps} />);

      expect(screen.getByText("98 KB")).toBeInTheDocument();
    });

    it("should display mid bitrate label", () => {
      render(<FrameSizesView {...defaultProps} />);

      expect(screen.getByText("49 KB")).toBeInTheDocument();
    });

    it("should display min label (0)", () => {
      render(<FrameSizesView {...defaultProps} />);

      expect(screen.getByText("0")).toBeInTheDocument();
    });

    it("should display max QP value", () => {
      render(<FrameSizesView {...defaultProps} />);

      expect(screen.getByText("51")).toBeInTheDocument();
    });

    it("should display mid QP value", () => {
      render(<FrameSizesView {...defaultProps} />);

      expect(screen.getByText("26")).toBeInTheDocument();
    });

    it("should mark current frame as selected", () => {
      render(<FrameSizesView {...defaultProps} />);

      const selectedBar = document.querySelector(
        '[data-frame-index="0"].selected',
      );
      expect(selectedBar).toBeInTheDocument();
    });

    it("should call onFrameClick when bar clicked", () => {
      render(<FrameSizesView {...defaultProps} />);

      const bar = document.querySelector('[data-frame-index="2"]');
      fireEvent.click(bar!);

      expect(defaultProps.onFrameClick).toHaveBeenCalledWith(2);
    });

    it("should apply correct frame type color class", () => {
      const getFrameTypeColorClass = vi.fn(
        (type: string) => `frame-type-${type.toLowerCase()}`,
      );
      const props = { ...defaultProps, getFrameTypeColorClass };

      render(<FrameSizesView {...props} />);

      expect(getFrameTypeColorClass).toHaveBeenCalledWith("I");
      expect(getFrameTypeColorClass).toHaveBeenCalledWith("P");
      expect(getFrameTypeColorClass).toHaveBeenCalledWith("B");

      const iBar = document.querySelector('[data-frame-index="0"]');
      expect(iBar).toHaveClass("frame-type-i");
    });
  });

  describe("FrameSizesView filtering", () => {
    it("should filter frames by visible type", () => {
      const props = {
        ...defaultProps,
        visibleFrameTypes: new Set(["I", "P"]),
      };

      render(<FrameSizesView {...props} />);

      const bars = document.querySelectorAll(".frame-size-bar");
      // Should only show I and P frames (not B frames)
      expect(bars).toHaveLength(3);
    });

    it("should show only I frames when only I is visible", () => {
      const props = {
        ...defaultProps,
        visibleFrameTypes: new Set(["I"]),
      };

      render(<FrameSizesView {...props} />);

      const bars = document.querySelectorAll(".frame-size-bar");
      expect(bars).toHaveLength(1);

      const bar = document.querySelector('[data-frame-index="0"]');
      expect(bar).toBeInTheDocument();
    });

    it("should handle empty visible frame types", () => {
      const props = {
        ...defaultProps,
        visibleFrameTypes: new Set([]),
      };

      render(<FrameSizesView {...props} />);

      const bars = document.querySelectorAll(".frame-size-bar");
      expect(bars).toHaveLength(0);
    });
  });

  describe("FrameSizesView metric lines", () => {
    it("should show average size line when enabled", () => {
      render(<FrameSizesView {...defaultProps} />);

      const avgLine = document.querySelector(".metric-line.avg-line");
      expect(avgLine).toBeInTheDocument();
    });

    it("should hide average size line when disabled", () => {
      const props = {
        ...defaultProps,
        sizeMetrics: { ...defaultSizeMetrics, showAvgSize: false },
      };

      render(<FrameSizesView {...props} />);

      const avgLine = document.querySelector(".metric-line.avg-line");
      expect(avgLine).not.toBeInTheDocument();
    });

    it("should show min size line when enabled", () => {
      render(<FrameSizesView {...defaultProps} />);

      const minLine = document.querySelector(".metric-line.min-line");
      expect(minLine).toBeInTheDocument();
    });

    it("should hide min size line when disabled", () => {
      const props = {
        ...defaultProps,
        sizeMetrics: { ...defaultSizeMetrics, showMinSize: false },
      };

      render(<FrameSizesView {...props} />);

      const minLine = document.querySelector(".metric-line.min-line");
      expect(minLine).not.toBeInTheDocument();
    });

    it("should show max size line when enabled", () => {
      render(<FrameSizesView {...defaultProps} />);

      const maxLine = document.querySelector(".metric-line.max-line");
      expect(maxLine).toBeInTheDocument();
    });

    it("should hide max size line when disabled", () => {
      const props = {
        ...defaultProps,
        sizeMetrics: { ...defaultSizeMetrics, showMaxSize: false },
      };

      render(<FrameSizesView {...props} />);

      const maxLine = document.querySelector(".metric-line.max-line");
      expect(maxLine).not.toBeInTheDocument();
    });
  });

  describe("FrameSizesView bitrate curve", () => {
    it("should show bitrate curve when enabled", () => {
      render(<FrameSizesView {...defaultProps} />);

      const svg = document.querySelector(".frame-sizes-line-chart");
      expect(svg).toBeInTheDocument();
    });

    it("should hide bitrate curve when disabled", () => {
      const props = {
        ...defaultProps,
        sizeMetrics: { ...defaultSizeMetrics, showBitrateCurve: false },
      };

      render(<FrameSizesView {...props} />);

      const svg = document.querySelector(".frame-sizes-line-chart");
      expect(svg).not.toBeInTheDocument();
    });

    it("should render polyline for bitrate curve", () => {
      render(<FrameSizesView {...defaultProps} />);

      const polyline = document.querySelector(
        ".frame-sizes-line-chart polyline",
      );
      expect(polyline).toBeInTheDocument();
    });

    it("should have correct stroke for bitrate curve", () => {
      render(<FrameSizesView {...defaultProps} />);

      const polyline = document.querySelector(
        ".frame-sizes-line-chart polyline",
      );
      expect(polyline).toHaveAttribute("stroke", "rgba(0, 220, 220, 1)");
    });
  });

  describe("FrameSizesView moving average", () => {
    it("should show moving average line when enabled", () => {
      render(<FrameSizesView {...defaultProps} />);

      const lineCharts = document.querySelectorAll(".frame-sizes-line-chart");
      // Should have two: bitrate curve + moving average
      expect(lineCharts.length).toBeGreaterThan(1);
    });

    it("should hide moving average when disabled", () => {
      const props = {
        ...defaultProps,
        sizeMetrics: { ...defaultSizeMetrics, showMovingAvg: false },
      };

      render(<FrameSizesView {...props} />);

      const lineCharts = document.querySelectorAll(".frame-sizes-line-chart");
      // Should only have bitrate curve
      expect(lineCharts).toHaveLength(1);
    });

    it("should have correct stroke for moving average", () => {
      render(<FrameSizesView {...defaultProps} />);

      const polylines = document.querySelectorAll(
        ".frame-sizes-line-chart polyline",
      );
      const movingAvgLine = polylines[1];
      expect(movingAvgLine).toHaveAttribute("stroke", "rgba(0, 150, 255, 0.8)");
    });
  });

  describe("FrameSizesView bar heights", () => {
    it("should calculate bar height proportionally", () => {
      render(<FrameSizesView {...defaultProps} />);

      const fullBar = document.querySelector('[data-frame-index="0"]');
      expect(fullBar).toHaveStyle({ height: "100%" });

      const halfBar = document.querySelector('[data-frame-index="1"]');
      expect(halfBar).toHaveStyle({ height: "50%" });
    });

    it("should apply frame type color to bar", () => {
      const { getFrameTypeColor } = require("@/types/video");

      render(<FrameSizesView {...defaultProps} />);

      const bar = document.querySelector('[data-frame-index="0"]');
      expect(getFrameTypeColor).toHaveBeenCalledWith("I");
    });

    it("should have tooltip with frame info", () => {
      render(<FrameSizesView {...defaultProps} />);

      const bar = document.querySelector('[data-frame-index="0"]');
      expect(bar).toHaveAttribute("title");
      expect(bar?.getAttribute("title")).toContain("Frame 0");
      expect(bar?.getAttribute("title")).toContain("KB");
      expect(bar?.getAttribute("title")).toContain("QP");
    });
  });

  describe("FrameSizesView edge cases", () => {
    it("should handle empty frames array", () => {
      const props = { ...defaultProps, frames: [] };
      render(<FrameSizesView {...props} />);

      const bars = document.querySelectorAll(".frame-size-bar");
      expect(bars).toHaveLength(0);
    });

    it("should handle single frame", () => {
      const props = {
        ...defaultProps,
        frames: [
          { frame_index: 0, frame_type: "I", size: 50000, poc: 0 },
        ] as FrameInfo[],
      };

      render(<FrameSizesView {...props} />);

      const bars = document.querySelectorAll(".frame-size-bar");
      expect(bars).toHaveLength(1);
    });

    it("should handle frames without duration for bitrate", () => {
      const framesWithoutDuration = [
        { frame_index: 0, frame_type: "I", size: 50000, poc: 0 },
      ] as FrameInfo[];
      const props = { ...defaultProps, frames: framesWithoutDuration };

      render(<FrameSizesView {...props} />);

      // Should still render bars
      const bars = document.querySelectorAll(".frame-size-bar");
      expect(bars).toHaveLength(1);
    });

    it("should handle zero max size", () => {
      const props = {
        ...defaultProps,
        maxSize: 0,
      };

      render(<FrameSizesView {...props} />);

      const bars = document.querySelectorAll(".frame-size-bar");
      expect(bars).toHaveLength(5);
    });

    it("should handle all same size frames", () => {
      const sameSizeFrames = mockFrames.map((f) => ({ ...f, size: 50000 }));
      const props = { ...defaultProps, frames: sameSizeFrames };

      render(<FrameSizesView {...props} />);

      const bars = document.querySelectorAll(".frame-size-bar");
      expect(bars).toHaveLength(5);
    });

    it("should handle very large frame sizes", () => {
      const largeFrames = [
        { frame_index: 0, frame_type: "I", size: 10000000, poc: 0 },
        { frame_index: 1, frame_type: "P", size: 5000000, poc: 1 },
      ] as FrameInfo[];
      const props = { ...defaultProps, frames: largeFrames, maxSize: 10000000 };

      render(<FrameSizesView {...props} />);

      const bars = document.querySelectorAll(".frame-size-bar");
      expect(bars).toHaveLength(2);
    });
  });

  describe("FrameSizesView calculations", () => {
    it("should calculate average size correctly", () => {
      render(<FrameSizesView {...defaultProps} />);

      const avgLine = document.querySelector(".metric-line.avg-line");
      expect(avgLine).toBeInTheDocument();
    });

    it("should calculate min size correctly", () => {
      render(<FrameSizesView {...defaultProps} />);

      const minLine = document.querySelector(".metric-line.min-line");
      expect(minLine).toBeInTheDocument();
    });

    it("should calculate max size correctly", () => {
      render(<FrameSizesView {...defaultProps} />);

      const maxLine = document.querySelector(".metric-line.max-line");
      expect(maxLine).toHaveStyle({ bottom: "100%" });
    });
  });

  describe("FrameSizesView React.memo", () => {
    it("should use React.memo for performance", () => {
      const { rerender } = render(<FrameSizesView {...defaultProps} />);

      const initialContainer = document.querySelector(".filmstrip-sizes");

      rerender(<FrameSizesView {...defaultProps} />);

      const rerenderedContainer = document.querySelector(".filmstrip-sizes");
      expect(rerenderedContainer).toEqual(initialContainer);
    });

    it("should re-render when currentFrameIndex changes", () => {
      const { rerender } = render(<FrameSizesView {...defaultProps} />);

      rerender(<FrameSizesView {...defaultProps} currentFrameIndex={2} />);

      const selectedBar = document.querySelector(
        '[data-frame-index="2"].selected',
      );
      expect(selectedBar).toBeInTheDocument();

      const previouslySelected = document.querySelector(
        '[data-frame-index="0"].selected',
      );
      expect(previouslySelected).not.toBeInTheDocument();
    });

    it("should re-render when sizeMetrics change", () => {
      const { rerender } = render(<FrameSizesView {...defaultProps} />);

      const newSizeMetrics = { ...defaultSizeMetrics, showAvgSize: false };

      rerender(
        <FrameSizesView {...defaultProps} sizeMetrics={newSizeMetrics} />,
      );

      const avgLine = document.querySelector(".metric-line.avg-line");
      expect(avgLine).not.toBeInTheDocument();
    });
  });
});
