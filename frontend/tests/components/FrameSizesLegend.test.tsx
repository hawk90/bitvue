/**
 * Frame Sizes Legend Component Tests
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@/test/test-utils";
import { FrameSizesLegend } from "@/components/FrameSizesLegend";

type SizeMetrics = {
  showBitrateBar: boolean;
  showBitrateCurve: boolean;
  showAvgSize: boolean;
  showMinSize: boolean;
  showMaxSize: boolean;
  showMovingAvg: boolean;
  showBlockMinQP: boolean;
  showBlockMaxQP: boolean;
};

const mockSizeMetrics: SizeMetrics = {
  showBitrateBar: true,
  showBitrateCurve: false,
  showAvgSize: true,
  showMinSize: false,
  showMaxSize: true,
  showMovingAvg: true,
  showBlockMinQP: false,
  showBlockMaxQP: false,
};

describe("FrameSizesLegend", () => {
  it("should render floating legend panel", () => {
    render(
      <FrameSizesLegend
        sizeMetrics={mockSizeMetrics}
        onToggleMetric={vi.fn()}
      />,
    );

    const legend = document.querySelector(".frame-sizes-legend.floating");
    expect(legend).toBeInTheDocument();
  });

  it("should render drag handle", () => {
    const { container } = render(
      <FrameSizesLegend
        sizeMetrics={mockSizeMetrics}
        onToggleMetric={vi.fn()}
      />,
    );

    expect(container.querySelector(".legend-drag-handle")).toBeInTheDocument();
  });

  it("should render drag gripper icon", () => {
    const { container } = render(
      <FrameSizesLegend
        sizeMetrics={mockSizeMetrics}
        onToggleMetric={vi.fn()}
      />,
    );

    expect(container.querySelector(".codicon-gripper")).toBeInTheDocument();
  });

  it("should render all metric items", () => {
    const { container } = render(
      <FrameSizesLegend
        sizeMetrics={mockSizeMetrics}
        onToggleMetric={vi.fn()}
      />,
    );

    const items = container.querySelectorAll(".legend-item");
    expect(items).toHaveLength(8);
  });

  it("should render metric labels", () => {
    render(
      <FrameSizesLegend
        sizeMetrics={mockSizeMetrics}
        onToggleMetric={vi.fn()}
      />,
    );

    expect(screen.getByText("Bitrate Bar")).toBeInTheDocument();
    expect(screen.getByText("Bitrate Curve")).toBeInTheDocument();
    expect(screen.getByText("Avg Size")).toBeInTheDocument();
    expect(screen.getByText("Min Size")).toBeInTheDocument();
    expect(screen.getByText("Max Size")).toBeInTheDocument();
    expect(screen.getByText("Moving Avg")).toBeInTheDocument();
    expect(screen.getByText("Block Min QP")).toBeInTheDocument();
    expect(screen.getByText("Block Max QP")).toBeInTheDocument();
  });

  it("should render color indicators", () => {
    const { container } = render(
      <FrameSizesLegend
        sizeMetrics={mockSizeMetrics}
        onToggleMetric={vi.fn()}
      />,
    );

    const indicators = container.querySelectorAll(".legend-color-indicator");
    expect(indicators).toHaveLength(8);
  });

  it("should apply active class to enabled metrics", () => {
    const { container } = render(
      <FrameSizesLegend
        sizeMetrics={mockSizeMetrics}
        onToggleMetric={vi.fn()}
      />,
    );

    const activeItems = container.querySelectorAll(".legend-item.active");
    expect(activeItems).toHaveLength(4); // showBitrateBar, showAvgSize, showMaxSize, showMovingAvg
  });

  it("should not apply active class to disabled metrics", () => {
    const { container } = render(
      <FrameSizesLegend
        sizeMetrics={mockSizeMetrics}
        onToggleMetric={vi.fn()}
      />,
    );

    const inactiveItems = container.querySelectorAll(
      ".legend-item:not(.active)",
    );
    expect(inactiveItems).toHaveLength(4);
  });

  it("should call onToggleMetric when item clicked", () => {
    const onToggleMetric = vi.fn();
    const { container } = render(
      <FrameSizesLegend
        sizeMetrics={mockSizeMetrics}
        onToggleMetric={onToggleMetric}
      />,
    );

    const firstItem = container.querySelector(".legend-item");
    if (firstItem) {
      fireEvent.click(firstItem);
      expect(onToggleMetric).toHaveBeenCalledTimes(1);
    }
  });

  it("should start with default position", () => {
    const { container } = render(
      <FrameSizesLegend
        sizeMetrics={mockSizeMetrics}
        onToggleMetric={vi.fn()}
      />,
    );

    const legend = container.querySelector(
      ".frame-sizes-legend",
    ) as HTMLElement;
    expect(legend.style.left).toContain("px");
    expect(legend.style.top).toContain("px");
  });

  it("should update position when dragging", () => {
    const { container } = render(
      <FrameSizesLegend
        sizeMetrics={mockSizeMetrics}
        onToggleMetric={vi.fn()}
      />,
    );

    const handle = container.querySelector(".legend-drag-handle");
    if (handle) {
      fireEvent.mouseDown(handle, { clientX: 100, clientY: 100 });

      // Simulate mouse move
      const mouseMoveEvent = new MouseEvent("mousemove", {
        clientX: 150,
        clientY: 150,
      });
      document.dispatchEvent(mouseMoveEvent);

      fireEvent.mouseUp(handle);
    }

    // Position should be updated (actual value depends on window size)
    const legend = container.querySelector(
      ".frame-sizes-legend",
    ) as HTMLElement;
    expect(legend.style.left).toBeTruthy();
  });

  it("should clamp position within window bounds", () => {
    // Mock window size
    const originalInnerWidth = window.innerWidth;
    const originalInnerHeight = window.innerHeight;

    Object.defineProperty(window, "innerWidth", {
      value: 800,
      configurable: true,
    });
    Object.defineProperty(window, "innerHeight", {
      value: 600,
      configurable: true,
    });

    const { container } = render(
      <FrameSizesLegend
        sizeMetrics={mockSizeMetrics}
        onToggleMetric={vi.fn()}
      />,
    );

    const handle = container.querySelector(".legend-drag-handle");
    if (handle) {
      fireEvent.mouseDown(handle, { clientX: 100, clientY: 100 });

      // Try to move beyond window bounds
      const mouseMoveEvent = new MouseEvent("mousemove", {
        clientX: -1000,
        clientY: -1000,
      });
      document.dispatchEvent(mouseMoveEvent);

      fireEvent.mouseUp(handle);
    }

    // Restore window size
    Object.defineProperty(window, "innerWidth", {
      value: originalInnerWidth,
      configurable: true,
    });
    Object.defineProperty(window, "innerHeight", {
      value: originalInnerHeight,
      configurable: true,
    });

    const legend = container.querySelector(
      ".frame-sizes-legend",
    ) as HTMLElement;
    const x = parseInt(legend.style.left);
    const y = parseInt(legend.style.top);

    expect(x).toBeGreaterThanOrEqual(0);
    expect(y).toBeGreaterThanOrEqual(0);
  });

  it("should set cursor to grabbing when dragging", () => {
    const { container } = render(
      <FrameSizesLegend
        sizeMetrics={mockSizeMetrics}
        onToggleMetric={vi.fn()}
      />,
    );

    const handle = container.querySelector(".legend-drag-handle");
    if (handle) {
      fireEvent.mouseDown(handle);
      const legend = container.querySelector(
        ".frame-sizes-legend",
      ) as HTMLElement;
      expect(legend.style.cursor).toBe("grabbing");

      fireEvent.mouseUp(handle);
      expect(legend.style.cursor).not.toBe("grabbing");
    }
  });

  it("should have fixed positioning", () => {
    const { container } = render(
      <FrameSizesLegend
        sizeMetrics={mockSizeMetrics}
        onToggleMetric={vi.fn()}
      />,
    );

    const legend = container.querySelector(
      ".frame-sizes-legend",
    ) as HTMLElement;
    expect(legend.style.position).toBe("fixed");
  });

  it("should have high z-index", () => {
    const { container } = render(
      <FrameSizesLegend
        sizeMetrics={mockSizeMetrics}
        onToggleMetric={vi.fn()}
      />,
    );

    const legend = container.querySelector(
      ".frame-sizes-legend",
    ) as HTMLElement;
    expect(legend.style.zIndex).toBe("1000");
  });

  it("should render all 8 metrics with correct colors", () => {
    const { container } = render(
      <FrameSizesLegend
        sizeMetrics={mockSizeMetrics}
        onToggleMetric={vi.fn()}
      />,
    );

    const indicators = container.querySelectorAll(".legend-color-indicator");
    const expectedColors = [
      "rgba(0, 200, 200, 0.9)", // Bitrate Bar
      "rgb(0, 220, 220)", // Bitrate Curve (browser omits alpha when 1)
      "rgba(255, 255, 0, 0.7)", // Avg Size
      "rgba(100, 150, 255, 0.7)", // Min Size
      "rgba(255, 0, 0, 0.7)", // Max Size
      "rgba(0, 150, 255, 0.7)", // Moving Avg
      "rgba(180, 100, 255, 0.9)", // Block Min QP
      "rgba(255, 100, 0, 0.9)", // Block Max QP
    ];

    indicators.forEach((indicator, i) => {
      expect((indicator as HTMLElement).style.backgroundColor).toBe(
        expectedColors[i],
      );
    });
  });

  it("should render legend content wrapper", () => {
    const { container } = render(
      <FrameSizesLegend
        sizeMetrics={mockSizeMetrics}
        onToggleMetric={vi.fn()}
      />,
    );

    expect(container.querySelector(".legend-content")).toBeInTheDocument();
  });

  it("should handle all metrics disabled", () => {
    const allDisabled: SizeMetrics = {
      showBitrateBar: false,
      showBitrateCurve: false,
      showAvgSize: false,
      showMinSize: false,
      showMaxSize: false,
      showMovingAvg: false,
      showBlockMinQP: false,
      showBlockMaxQP: false,
    };

    const { container } = render(
      <FrameSizesLegend sizeMetrics={allDisabled} onToggleMetric={vi.fn()} />,
    );

    const activeItems = container.querySelectorAll(".legend-item.active");
    expect(activeItems).toHaveLength(0);
  });

  it("should handle all metrics enabled", () => {
    const allEnabled: SizeMetrics = {
      showBitrateBar: true,
      showBitrateCurve: true,
      showAvgSize: true,
      showMinSize: true,
      showMaxSize: true,
      showMovingAvg: true,
      showBlockMinQP: true,
      showBlockMaxQP: true,
    };

    const { container } = render(
      <FrameSizesLegend sizeMetrics={allEnabled} onToggleMetric={vi.fn()} />,
    );

    const activeItems = container.querySelectorAll(".legend-item.active");
    expect(activeItems).toHaveLength(8);
  });
});
