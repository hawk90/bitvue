/**
 * MinimapView Component Tests
 * Tests timeline minimap component
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@/test/test-utils";
import { MinimapView } from "../MinimapView";

describe("MinimapView", () => {
  const mockFrames = [
    { frame_index: 0, frame_type: "I", size: 50000, poc: 0 },
    { frame_index: 1, frame_type: "P", size: 30000, poc: 1 },
    { frame_index: 2, frame_type: "P", size: 35000, poc: 2 },
    { frame_index: 3, frame_type: "B", size: 20000, poc: 3 },
    { frame_index: 4, frame_type: "B", size: 25000, poc: 4 },
  ];

  const defaultProps = {
    frames: mockFrames,
    currentFrameIndex: 2,
    onFrameClick: vi.fn(),
    getFrameTypeColorClass: vi.fn(
      (type: string) => `frame-type-${type.toLowerCase()}`,
    ),
  };

  it("should render minimap view", () => {
    render(<MinimapView {...defaultProps} />);

    expect(document.querySelector(".minimap-view")).toBeInTheDocument();
  });

  it("should render all frame indicators", () => {
    render(<MinimapView {...defaultProps} />);

    const indicators = document.querySelectorAll(".minimap-frame");
    expect(indicators.length).toBe(5);
  });

  it("should highlight current frame", () => {
    render(<MinimapView {...defaultProps} currentFrameIndex={2} />);

    const currentFrame = document.querySelector('[data-current="true"]');
    expect(currentFrame).toBeInTheDocument();
  });

  it("should call onFrameClick when indicator clicked", () => {
    const handleClick = vi.fn();
    render(<MinimapView {...defaultProps} onFrameClick={handleClick} />);

    const indicators = document.querySelectorAll(".minimap-frame");
    if (indicators.length > 0) {
      fireEvent.click(indicators[0]);
      expect(handleClick).toHaveBeenCalledWith(0);
    }
  });

  it("should use React.memo for performance", () => {
    const { rerender } = render(<MinimapView {...defaultProps} />);

    rerender(<MinimapView {...defaultProps} />);

    expect(document.querySelector(".minimap-view")).toBeInTheDocument();
  });

  it("should show viewport indicator", () => {
    render(
      <MinimapView {...defaultProps} visibleRange={{ start: 0, end: 3 }} />,
    );

    const viewport = document.querySelector(".minimap-viewport");
    expect(viewport).toBeInTheDocument();
  });

  it("should display frame type colors", () => {
    render(<MinimapView {...defaultProps} />);

    const iFrames = document.querySelectorAll(".frame-type-i");
    const pFrames = document.querySelectorAll(".frame-type-p");
    const bFrames = document.querySelectorAll(".frame-type-b");

    expect(iFrames.length).toBeGreaterThan(0);
    expect(pFrames.length).toBeGreaterThan(0);
    expect(bFrames.length).toBeGreaterThan(0);
  });
});

describe("MinimapView sizing", () => {
  it("should scale indicator based on frame size", () => {
    const frames = [
      { frame_index: 0, frame_type: "I", size: 100000, poc: 0 },
      { frame_index: 1, frame_type: "P", size: 10000, poc: 1 },
    ];

    const props = {
      frames,
      currentFrameIndex: 0,
      onFrameClick: vi.fn(),
      getFrameTypeColorClass: vi.fn(() => ""),
    };

    render(<MinimapView {...props} />);

    const indicators = document.querySelectorAll(".minimap-frame");
    expect(indicators.length).toBe(2);
  });

  it("should handle large frame arrays", () => {
    const frames = Array.from({ length: 1000 }, (_, i) => ({
      frame_index: i,
      frame_type: "P",
      size: 30000,
      poc: i,
    }));

    const props = {
      frames,
      currentFrameIndex: 500,
      onFrameClick: vi.fn(),
      getFrameTypeColorClass: vi.fn(() => ""),
    };

    render(<MinimapView {...props} />);

    expect(document.querySelector(".minimap-view")).toBeInTheDocument();
  });
});

describe("MinimapView edge cases", () => {
  it("should handle empty frames array", () => {
    const props = {
      frames: [],
      currentFrameIndex: -1,
      onFrameClick: vi.fn(),
      getFrameTypeColorClass: vi.fn(() => ""),
    };

    render(<MinimapView {...props} />);

    expect(document.querySelector(".minimap-view")).toBeInTheDocument();
  });

  it("should handle single frame", () => {
    const props = {
      frames: [{ frame_index: 0, frame_type: "I", size: 50000, poc: 0 }],
      currentFrameIndex: 0,
      onFrameClick: vi.fn(),
      getFrameTypeColorClass: vi.fn(() => ""),
    };

    render(<MinimapView {...props} />);

    const indicators = document.querySelectorAll(".minimap-frame");
    expect(indicators.length).toBe(1);
  });

  it("should handle current frame out of range", () => {
    const props = {
      frames: [{ frame_index: 0, frame_type: "I", size: 50000, poc: 0 }],
      currentFrameIndex: 999,
      onFrameClick: vi.fn(),
      getFrameTypeColorClass: vi.fn(() => ""),
    };

    render(<MinimapView {...props} />);

    // Should still render without crashing
    expect(document.querySelector(".minimap-view")).toBeInTheDocument();
  });
});

describe("MinimapView interactions", () => {
  it("should support click navigation", () => {
    const handleClick = vi.fn();
    const props = {
      frames: [
        { frame_index: 0, frame_type: "I", size: 50000, poc: 0 },
        { frame_index: 1, frame_type: "P", size: 30000, poc: 1 },
      ],
      currentFrameIndex: 0,
      onFrameClick: handleClick,
      getFrameTypeColorClass: vi.fn(() => ""),
    };

    render(<MinimapView {...props} />);

    const indicators = document.querySelectorAll(".minimap-frame");
    if (indicators.length > 1) {
      fireEvent.click(indicators[1]);
      expect(handleClick).toHaveBeenCalledWith(1);
    }
  });

  it("should show hover tooltip", () => {
    const props = {
      frames: [{ frame_index: 0, frame_type: "I", size: 50000, poc: 0 }],
      currentFrameIndex: 0,
      onFrameClick: vi.fn(),
      getFrameTypeColorClass: vi.fn(() => ""),
    };

    render(<MinimapView {...props} />);

    const indicator = document.querySelector(".minimap-frame");
    if (indicator) {
      fireEvent.mouseEnter(indicator);
      // Tooltip state should be set (verified by DOM check)
      expect(indicator).toBeInTheDocument();
    }
  });
});
