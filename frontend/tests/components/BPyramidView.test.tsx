/**
 * BPyramidView Component Tests
 * Tests B-Pyramid GOP hierarchy visualization
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@/test/test-utils";
import { BPyramidView } from "../BPyramidView";
import { analyzeTemporalLevels } from "../BPyramidTimeline";
import type { FrameInfo } from "@/types/video";
import { usePreRenderedArrows } from "@/components/Filmstrip/usePreRenderedArrows";
import { getFrameTypeColor } from "@/types/video";

// Mock usePreRenderedArrows hook
vi.mock("@/components/Filmstrip/usePreRenderedArrows", () => ({
  usePreRenderedArrows: vi.fn(() => ({
    allArrowData: [],
    svgWidth: 0,
  })),
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
    frame_type: "B",
    size: 25000,
    poc: 3,
    temporal_id: 1,
    ref_frames: [1, 4],
  },
  {
    frame_index: 4,
    frame_type: "P",
    size: 35000,
    poc: 4,
    temporal_id: 0,
    ref_frames: [0],
  },
];

const defaultProps = {
  frames: mockFrames,
  currentFrameIndex: 0,
  onFrameClick: vi.fn(),
  getFrameTypeColorClass: vi.fn(
    (type: string) => `frame-type-${type.toLowerCase()}`,
  ),
};

describe("BPyramidView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should render B-Pyramid view container", () => {
    render(<BPyramidView {...defaultProps} />);

    const container = document.querySelector(".bpyramid-view");
    expect(container).toBeInTheDocument();
    expect(container).toHaveAttribute("role", "region");
    expect(container).toHaveAttribute("aria-label", "B-Pyramid");
  });

  it("should render timeline container", () => {
    render(<BPyramidView {...defaultProps} />);

    const timeline = document.querySelector(".bpyramid-timeline-container");
    expect(timeline).toBeInTheDocument();
  });

  it("should render temporal levels", () => {
    render(<BPyramidView {...defaultProps} />);

    const levels = document.querySelectorAll(".bpyramid-level-row");
    expect(levels.length).toBeGreaterThan(0);
  });

  it("should display level labels", () => {
    render(<BPyramidView {...defaultProps} />);

    expect(screen.getByText("L1")).toBeInTheDocument();
    expect(screen.getByText("L0")).toBeInTheDocument();
  });

  it("should render frame circles", () => {
    render(<BPyramidView {...defaultProps} />);

    const circles = document.querySelectorAll(".bpyramid-frame-circle");
    expect(circles.length).toBe(5);
  });

  it("should mark current frame as selected", () => {
    render(<BPyramidView {...defaultProps} />);

    const selectedCircle = document.querySelector(
      '[data-frame-index="0"].selected',
    );
    expect(selectedCircle).toBeInTheDocument();
  });

  it("should mark GOP boundary frames", () => {
    render(<BPyramidView {...defaultProps} />);

    const gopBoundary = document.querySelector(
      '[data-frame-index="0"].gop-boundary',
    );
    expect(gopBoundary).toBeInTheDocument();
  });

  it("should call onFrameClick when circle clicked", () => {
    render(<BPyramidView {...defaultProps} />);

    const circle = document.querySelector('[data-frame-index="2"]');
    fireEvent.click(circle!);

    expect(defaultProps.onFrameClick).toHaveBeenCalledWith(2);
  });

  it("should apply correct frame type color class", () => {
    const getFrameTypeColorClass = vi.fn(
      (type: string) => `frame-type-${type.toLowerCase()}`,
    );
    const props = { ...defaultProps, getFrameTypeColorClass };

    render(<BPyramidView {...props} />);

    expect(getFrameTypeColorClass).toHaveBeenCalledWith("I");
    expect(getFrameTypeColorClass).toHaveBeenCalledWith("P");
    expect(getFrameTypeColorClass).toHaveBeenCalledWith("B");
  });

  it("should have proper ARIA attributes", () => {
    render(<BPyramidView {...defaultProps} />);

    const container = document.querySelector(".bpyramid-view");
    expect(container).toHaveAttribute("role", "region");
    expect(container).toHaveAttribute("aria-label", "B-Pyramid");
  });
});

describe("BPyramidView empty state", () => {
  it("should show empty state when no frames", () => {
    const props = { ...defaultProps, frames: [] };
    render(<BPyramidView {...props} />);

    const emptyState = document.querySelector(".bpyramid-empty");
    expect(emptyState).toBeInTheDocument();

    expect(screen.getByText("No frames loaded")).toBeInTheDocument();
  });

  it("should show graph icon in empty state", () => {
    const props = { ...defaultProps, frames: [] };
    render(<BPyramidView {...props} />);

    const icon = document.querySelector(".codicon-graph");
    expect(icon).toBeInTheDocument();
  });

  it("should have correct ARIA for empty state", () => {
    const props = { ...defaultProps, frames: [] };
    render(<BPyramidView {...props} />);

    const emptyState = document.querySelector(".bpyramid-empty");
    expect(emptyState).toHaveAttribute("role", "status");
    expect(emptyState).toHaveAttribute("aria-label", "No frames loaded");
  });
});

describe("BPyramidView temporal levels", () => {
  it("should organize frames by temporal_id", () => {
    render(<BPyramidView {...defaultProps} />);

    const level0Circles = document.querySelectorAll(
      ".bpyramid-level-row:nth-child(2) .bpyramid-frame-circle",
    );
    const level1Circles = document.querySelectorAll(
      ".bpyramid-level-row:nth-child(1) .bpyramid-frame-circle",
    );

    // Level 0 should have I, P frames (temporal_id: 0)
    expect(level0Circles.length).toBeGreaterThanOrEqual(3);

    // Level 1 should have B frames (temporal_id: 1)
    expect(level1Circles.length).toBeGreaterThanOrEqual(2);
  });

  it("should render empty cells for frames not in level", () => {
    render(<BPyramidView {...defaultProps} />);

    const emptyCells = document.querySelectorAll(".bpyramid-cell-empty");
    expect(emptyCells.length).toBeGreaterThan(0);
  });

  it("should display levels in descending order", () => {
    render(<BPyramidView {...defaultProps} />);

    const levelLabels = document.querySelectorAll(".bpyramid-level-label");
    expect(levelLabels[0]).toHaveTextContent("L1");
    expect(levelLabels[1]).toHaveTextContent("L0");
  });
});

describe("BPyramidView reference arrows", () => {
  it("should not render SVG when no arrow data", () => {
    render(<BPyramidView {...defaultProps} />);

    const svg = document.querySelector(".bpyramid-arrows-overlay");
    expect(svg).not.toBeInTheDocument();
  });

  it("should render SVG overlay when arrows exist", () => {
    vi.mocked(usePreRenderedArrows).mockReturnValue({
      allArrowData: [
        {
          sourceFrameIndex: 2,
          targetFrameIndex: 0,
          pathData: "M0,0 L10,10",
          color: "#4444ff",
        },
      ],
      svgWidth: 1000,
    });

    render(<BPyramidView {...defaultProps} />);

    const svg = document.querySelector(".bpyramid-arrows-overlay");
    expect(svg).toBeInTheDocument();
  });

  it("should render arrow marker definition", () => {
    vi.mocked(usePreRenderedArrows).mockReturnValue({
      allArrowData: [
        {
          sourceFrameIndex: 2,
          targetFrameIndex: 0,
          pathData: "M0,0 L10,10",
          color: "#4444ff",
        },
      ],
      svgWidth: 1000,
    });

    render(<BPyramidView {...defaultProps} />);

    const marker = document.querySelector("#bpyramid-arrowhead");
    expect(marker).toBeInTheDocument();
  });
});

describe("BPyramidView frame circles", () => {
  it("should have correct size", () => {
    render(<BPyramidView {...defaultProps} />);

    const circle = document.querySelector('[data-frame-index="0"]');
    expect(circle).toHaveStyle({ width: "12px", height: "12px" });
  });

  it("should have background color from frame type", () => {
    render(<BPyramidView {...defaultProps} />);

    expect(getFrameTypeColor).toHaveBeenCalledWith("I");
    expect(getFrameTypeColor).toHaveBeenCalledWith("P");
    expect(getFrameTypeColor).toHaveBeenCalledWith("B");
  });

  it("should have tooltip with frame info", () => {
    render(<BPyramidView {...defaultProps} />);

    const circle = document.querySelector('[data-frame-index="0"]');
    expect(circle).toHaveAttribute("title");
    expect(circle?.getAttribute("title")).toContain("Frame 0");
    expect(circle?.getAttribute("title")).toContain("I");
    expect(circle?.getAttribute("title")).toContain("Level");
  });

  it("should handle current frame selection change", () => {
    const { rerender } = render(<BPyramidView {...defaultProps} />);

    rerender(<BPyramidView {...defaultProps} currentFrameIndex={2} />);

    const selectedCircle = document.querySelector(
      '[data-frame-index="2"].selected',
    );
    expect(selectedCircle).toBeInTheDocument();

    const previouslySelected = document.querySelector(
      '[data-frame-index="0"].selected',
    );
    expect(previouslySelected).not.toBeInTheDocument();
  });
});

describe("BPyramidView edge cases", () => {
  it("should handle frames without temporal_id", () => {
    const framesWithoutTemporal = [
      { frame_index: 0, frame_type: "I", size: 50000, poc: 0 },
    ] as FrameInfo[];
    const props = { ...defaultProps, frames: framesWithoutTemporal };

    render(<BPyramidView {...props} />);

    const circles = document.querySelectorAll(".bpyramid-frame-circle");
    expect(circles.length).toBe(1);
  });

  it("should handle single frame", () => {
    const singleFrame = [
      { frame_index: 0, frame_type: "I", size: 50000, poc: 0, temporal_id: 0 },
    ];
    const props = { ...defaultProps, frames: singleFrame };

    render(<BPyramidView {...props} />);

    const circles = document.querySelectorAll(".bpyramid-frame-circle");
    expect(circles.length).toBe(1);
  });

  it("should handle all frames at same temporal level", () => {
    const sameLevelFrames = mockFrames.map((f) => ({ ...f, temporal_id: 0 }));
    const props = { ...defaultProps, frames: sameLevelFrames };

    render(<BPyramidView {...props} />);

    const levels = document.querySelectorAll(".bpyramid-level-row");
    expect(levels.length).toBe(1);
  });

  it("should handle frames without ref_frames", () => {
    const framesWithoutRefs = [
      { frame_index: 0, frame_type: "I", size: 50000, poc: 0, temporal_id: 0 },
      { frame_index: 1, frame_type: "P", size: 30000, poc: 1, temporal_id: 0 },
    ] as FrameInfo[];
    const props = { ...defaultProps, frames: framesWithoutRefs };

    render(<BPyramidView {...props} />);

    const circles = document.querySelectorAll(".bpyramid-frame-circle");
    expect(circles.length).toBe(2);
  });

  it("should handle large temporal_id values", () => {
    const highTemporalFrames = [
      { frame_index: 0, frame_type: "I", size: 50000, poc: 0, temporal_id: 0 },
      { frame_index: 1, frame_type: "B", size: 20000, poc: 1, temporal_id: 5 },
    ] as FrameInfo[];
    const props = { ...defaultProps, frames: highTemporalFrames };

    render(<BPyramidView {...props} />);

    const levels = document.querySelectorAll(".bpyramid-level-row");
    expect(levels.length).toBe(2);
  });
});

describe("analyzeTemporalLevels", () => {
  it("should return empty analysis for empty frames", () => {
    const result = analyzeTemporalLevels([]);

    expect(result.levels).toEqual([]);
    expect(result.gopBoundaries).toEqual([]);
    expect(result.frameMap).toBeInstanceOf(Map);
    expect(result.framePositions).toBeInstanceOf(Map);
  });

  it("should identify keyframes as GOP boundaries", () => {
    const result = analyzeTemporalLevels(mockFrames);

    expect(result.gopBoundaries).toContain(0);
    expect(result.gopBoundaries.length).toBeGreaterThanOrEqual(1);
  });

  it("should map frames to temporal levels", () => {
    const result = analyzeTemporalLevels(mockFrames);

    expect(result.frameMap).toBeInstanceOf(Map);
    expect(result.frameMap.size).toBe(5);

    const frame0 = result.frameMap.get(0);
    expect(frame0?.level).toBe(0);
    expect(frame0?.isKeyframe).toBe(true);

    const frame2 = result.frameMap.get(2);
    expect(frame2?.level).toBe(1);
  });

  it("should organize levels in descending order", () => {
    const result = analyzeTemporalLevels(mockFrames);

    expect(result.levels.length).toBeGreaterThan(0);

    if (result.levels.length >= 2) {
      expect(result.levels[0].level).toBeGreaterThan(result.levels[1].level);
    }
  });

  it("should handle frames with temporal_id", () => {
    const framesWithTemporal = [
      { frame_index: 0, frame_type: "I", size: 50000, poc: 0, temporal_id: 2 },
      { frame_index: 1, frame_type: "B", size: 20000, poc: 1, temporal_id: 1 },
      { frame_index: 2, frame_type: "B", size: 25000, poc: 2, temporal_id: 0 },
    ] as FrameInfo[];

    const result = analyzeTemporalLevels(framesWithTemporal);

    expect(result.levels.length).toBe(3);
    expect(result.levels[0].level).toBe(2);
    expect(result.levels[2].level).toBe(0);
  });

  it("should default to level 0 when temporal_id is undefined", () => {
    const framesWithoutTemporal = [
      { frame_index: 0, frame_type: "I", size: 50000, poc: 0 },
    ] as FrameInfo[];

    const result = analyzeTemporalLevels(framesWithoutTemporal);

    const frame = result.frameMap.get(0);
    expect(frame?.level).toBe(0);
  });

  it("should identify I frames as keyframes", () => {
    const result = analyzeTemporalLevels(mockFrames);

    const frame0 = result.frameMap.get(0);
    expect(frame0?.isKeyframe).toBe(true);
  });

  it("should identify frames marked as key_frame", () => {
    const frames = [
      { frame_index: 0, frame_type: "P", size: 50000, poc: 0, key_frame: true },
    ] as FrameInfo[];

    const result = analyzeTemporalLevels(frames);

    const frame = result.frameMap.get(0);
    expect(frame?.isKeyframe).toBe(true);
  });

  it("should identify KEY frames as keyframes", () => {
    const frames = [
      { frame_index: 0, frame_type: "KEY", size: 50000, poc: 0 },
    ] as FrameInfo[];

    const result = analyzeTemporalLevels(frames);

    const frame = result.frameMap.get(0);
    expect(frame?.isKeyframe).toBe(true);
  });

  it("should handle frames with empty ref_frames", () => {
    const frames = [
      { frame_index: 0, frame_type: "I", size: 50000, poc: 0, ref_frames: [] },
    ] as FrameInfo[];

    const result = analyzeTemporalLevels(frames);

    const frame = result.frameMap.get(0);
    expect(frame?.refFrames).toEqual([]);
  });

  it("should create empty framePositions map", () => {
    const result = analyzeTemporalLevels(mockFrames);

    expect(result.framePositions).toBeInstanceOf(Map);
    expect(result.framePositions.size).toBe(0);
  });

  it("should sort frames within each level by index", () => {
    const result = analyzeTemporalLevels(mockFrames);

    result.levels.forEach((level) => {
      for (let i = 1; i < level.frames.length; i++) {
        expect(level.frames[i].frameIndex).toBeGreaterThan(
          level.frames[i - 1].frameIndex,
        );
      }
    });
  });

  it("should include first frame as GOP boundary", () => {
    const result = analyzeTemporalLevels(mockFrames);

    expect(result.gopBoundaries[0]).toBe(0);
  });

  it("should find all I frames as GOP boundaries", () => {
    const framesWithMultipleI = [
      { frame_index: 0, frame_type: "I", size: 50000, poc: 0 },
      { frame_index: 1, frame_type: "P", size: 30000, poc: 1 },
      { frame_index: 2, frame_type: "I", size: 50000, poc: 2 },
      { frame_index: 3, frame_type: "P", size: 30000, poc: 3 },
    ] as FrameInfo[];

    const result = analyzeTemporalLevels(framesWithMultipleI);

    expect(result.gopBoundaries).toContain(0);
    expect(result.gopBoundaries).toContain(2);
  });
});
