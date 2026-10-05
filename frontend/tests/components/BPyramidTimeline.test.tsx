/**
 * BPyramidTimeline Component Tests
 * Tests B-Pyramid timeline visualization
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@/test/test-utils";
import {
  BPyramidTimeline,
  analyzeTemporalLevels,
} from "../Filmstrip/views/BPyramidTimeline";
import type { FrameInfo } from "@/types/video";
import { usePreRenderedArrows } from "@/components/usePreRenderedArrows";
import { getFrameTypeColor } from "@/types/video";

// Mock usePreRenderedArrows hook
vi.mock("@/components/usePreRenderedArrows", () => ({
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

// A single 9-frame GOP encoding the classic dyadic hierarchical-B reference structure, using
// real `display_order` + `ref_frames` (frame_index pointers) exactly as the real app populates
// them (see FileStateContext.tsx's applyDisplayOrder + bitvue-indexer's PNL-03 ref_frames fix) --
// NOT AV1's raw frame_type, which is only ever "I"/"P" in this app (AV1 has no B frame_type at
// all). Decode order (frame_index, array position) is: I0, P8, then the mid/quarter/eighth-point
// B-frames, each referencing its two nearest already-coded neighbors in display order:
//   frame_index: 0    1    2    3    4    5    6    7    8
//   display_order: 0    8    4    2    6    1    3    5    7
//   refs (by frame_index): --   [0]  [0,1] [0,2] [2,1] [0,3] [3,2] [2,4] [4,1]
// Expected derived levels: {0,1}->0 (I/P anchors, forward-only) {2}->1 {3,4}->2 {5,6,7,8}->3.
const mockFrames: FrameInfo[] = [
  {
    frame_index: 0,
    frame_type: "I",
    size: 50000,
    display_order: 0,
    key_frame: true,
  },
  {
    frame_index: 1,
    frame_type: "P",
    size: 40000,
    display_order: 8,
    ref_frames: [0],
  },
  {
    frame_index: 2,
    frame_type: "P",
    size: 20000,
    display_order: 4,
    ref_frames: [0, 1],
  },
  {
    frame_index: 3,
    frame_type: "P",
    size: 20000,
    display_order: 2,
    ref_frames: [0, 2],
  },
  {
    frame_index: 4,
    frame_type: "P",
    size: 20000,
    display_order: 6,
    ref_frames: [2, 1],
  },
  {
    frame_index: 5,
    frame_type: "P",
    size: 22000,
    display_order: 1,
    ref_frames: [0, 3],
  },
  {
    frame_index: 6,
    frame_type: "P",
    size: 22000,
    display_order: 3,
    ref_frames: [3, 2],
  },
  {
    frame_index: 7,
    frame_type: "P",
    size: 22000,
    display_order: 5,
    ref_frames: [2, 4],
  },
  {
    frame_index: 8,
    frame_type: "P",
    size: 22000,
    display_order: 7,
    ref_frames: [4, 1],
  },
];

const defaultProps = {
  frames: mockFrames,
  currentFrameIndex: 4,
  onFrameClick: vi.fn(),
  getFrameTypeColorClass: vi.fn(
    (type: string) => `frame-type-${type.toLowerCase()}`,
  ),
  levels: analyzeTemporalLevels(mockFrames).levels,
  frameMap: analyzeTemporalLevels(mockFrames).frameMap,
  gopBoundaries: analyzeTemporalLevels(mockFrames).gopBoundaries,
};

describe("BPyramidTimeline", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should render B-Pyramid timeline", () => {
    render(<BPyramidTimeline {...defaultProps} />);

    expect(
      document.querySelector(".bpyramid-timeline-container"),
    ).toBeInTheDocument();
  });

  it("should display pyramid structure", () => {
    render(<BPyramidTimeline {...defaultProps} />);

    const pyramid = document.querySelector(".bpyramid-levels");
    expect(pyramid).toBeInTheDocument();
  });

  it("should show temporal layers", () => {
    render(<BPyramidTimeline {...defaultProps} />);

    const layers = document.querySelectorAll(".bpyramid-level-row");
    expect(layers.length).toBeGreaterThan(0);
  });

  it("should highlight current frame", () => {
    render(<BPyramidTimeline {...defaultProps} />);

    const currentFrame = document.querySelector(
      '[data-frame-index="4"].selected',
    );
    expect(currentFrame).toBeInTheDocument();
  });

  it("should use React.memo for performance", () => {
    const { rerender } = render(<BPyramidTimeline {...defaultProps} />);

    rerender(<BPyramidTimeline {...defaultProps} />);

    expect(
      document.querySelector(".bpyramid-timeline-container"),
    ).toBeInTheDocument();
  });

  it("should display POC values in tooltips", () => {
    render(<BPyramidTimeline {...defaultProps} />);

    const frame = document.querySelector('[data-frame-index="0"]');
    expect(frame).toHaveAttribute("title");
    expect(frame?.getAttribute("title")).toContain("Frame 0");
  });
});

describe("BPyramidTimeline references", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset to default mock return value
    vi.mocked(usePreRenderedArrows).mockReturnValue({
      allArrowData: [],
      svgWidth: 0,
    });
  });

  it("should show reference arrows when arrow data exists", () => {
    vi.mocked(usePreRenderedArrows).mockReturnValue({
      allArrowData: [
        {
          sourceFrameIndex: 4,
          targetFrameIndex: 0,
          slotIndex: 0,
          label: "L0",
          color: "#4444ff",
          pathData: "M0,0 L10,10",
          sourceX: 100,
          sourceY: 50,
          labelY: 45,
        },
      ],
      svgWidth: 1000,
    });

    render(<BPyramidTimeline {...defaultProps} />);

    const svg = document.querySelector(".bpyramid-arrows-overlay");
    expect(svg).toBeInTheDocument();
  });

  it("should not render SVG overlay when no arrow data", () => {
    render(<BPyramidTimeline {...defaultProps} />);

    const svg = document.querySelector(".bpyramid-arrows-overlay");
    // When svgWidth is 0 or allArrowData is empty, SVG should not be in document
    expect(svg).not.toBeInTheDocument();
  });
});

describe("BPyramidTimeline interactions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should support frame click navigation", () => {
    render(<BPyramidTimeline {...defaultProps} />);

    const frame = document.querySelector('[data-frame-index="2"]');
    expect(frame).toBeInTheDocument();
    fireEvent.click(frame!);

    expect(defaultProps.onFrameClick).toHaveBeenCalledWith(2);
  });

  it("should show hover information via title attribute", () => {
    render(<BPyramidTimeline {...defaultProps} />);

    const frame = document.querySelector('[data-frame-index="0"]');
    expect(frame).toHaveAttribute("title");
    const title = frame?.getAttribute("title");
    expect(title).toContain("Frame 0");
    expect(title).toContain("I");
    expect(title).toContain("Level");
  });
});

describe("BPyramidTimeline edge cases", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should handle empty frames", () => {
    const emptyAnalysis = analyzeTemporalLevels([]);
    const props = {
      ...defaultProps,
      frames: [],
      levels: emptyAnalysis.levels,
      frameMap: emptyAnalysis.frameMap,
      gopBoundaries: emptyAnalysis.gopBoundaries,
    };

    render(<BPyramidTimeline {...props} />);

    expect(
      document.querySelector(".bpyramid-timeline-container"),
    ).toBeInTheDocument();
  });

  it("should handle single frame", () => {
    const singleFrame = [
      {
        frame_index: 0,
        frame_type: "I",
        size: 50000,
        poc: 0,
        temporal_id: 0,
        key_frame: true,
      },
    ] as FrameInfo[];
    const singleAnalysis = analyzeTemporalLevels(singleFrame);
    const props = {
      ...defaultProps,
      frames: singleFrame,
      currentFrameIndex: 0,
      levels: singleAnalysis.levels,
      frameMap: singleAnalysis.frameMap,
      gopBoundaries: singleAnalysis.gopBoundaries,
    };

    render(<BPyramidTimeline {...props} />);

    expect(
      document.querySelector(".bpyramid-timeline-container"),
    ).toBeInTheDocument();
    const circle = document.querySelector('[data-frame-index="0"]');
    expect(circle).toBeInTheDocument();
  });

  it("should handle frames without temporal_id", () => {
    const framesWithoutTemporal = [
      { frame_index: 0, frame_type: "I", size: 50000, poc: 0 },
      { frame_index: 1, frame_type: "P", size: 30000, poc: 1 },
    ] as FrameInfo[];
    const analysis = analyzeTemporalLevels(framesWithoutTemporal);
    const props = {
      ...defaultProps,
      frames: framesWithoutTemporal,
      levels: analysis.levels,
      frameMap: analysis.frameMap,
      gopBoundaries: analysis.gopBoundaries,
    };

    render(<BPyramidTimeline {...props} />);

    expect(
      document.querySelector(".bpyramid-timeline-container"),
    ).toBeInTheDocument();
  });

  it("should render level labels", () => {
    render(<BPyramidTimeline {...defaultProps} />);

    expect(screen.getByText("L3")).toBeInTheDocument();
    expect(screen.getByText("L2")).toBeInTheDocument();
    expect(screen.getByText("L1")).toBeInTheDocument();
    expect(screen.getByText("L0")).toBeInTheDocument();
  });

  it("should mark GOP boundary frames", () => {
    render(<BPyramidTimeline {...defaultProps} />);

    const gopBoundary = document.querySelector(
      '[data-frame-index="0"].gop-boundary',
    );
    expect(gopBoundary).toBeInTheDocument();
  });
});

describe("BPyramidTimeline frame circles", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should have correct size", () => {
    render(<BPyramidTimeline {...defaultProps} />);

    const circle = document.querySelector('[data-frame-index="0"]');
    expect(circle).toHaveStyle({ width: "12px", height: "12px" });
  });

  it("should have background color from frame type", () => {
    render(<BPyramidTimeline {...defaultProps} />);

    expect(getFrameTypeColor).toHaveBeenCalledWith("I");
    expect(getFrameTypeColor).toHaveBeenCalledWith("B");
  });

  it("should handle current frame selection change", () => {
    const { rerender } = render(<BPyramidTimeline {...defaultProps} />);

    rerender(<BPyramidTimeline {...defaultProps} currentFrameIndex={2} />);

    const selectedCircle = document.querySelector(
      '[data-frame-index="2"].selected',
    );
    expect(selectedCircle).toBeInTheDocument();

    const previouslySelected = document.querySelector(
      '[data-frame-index="4"].selected',
    );
    expect(previouslySelected).not.toBeInTheDocument();
  });

  it("should render empty cells for frames not in level", () => {
    render(<BPyramidTimeline {...defaultProps} />);

    const emptyCells = document.querySelectorAll(".bpyramid-cell-empty");
    expect(emptyCells.length).toBeGreaterThan(0);
  });

  it("should display levels in descending order", () => {
    render(<BPyramidTimeline {...defaultProps} />);

    const levelLabels = document.querySelectorAll(".bpyramid-level-label");
    expect(levelLabels[0]).toHaveTextContent("L3");
    expect(levelLabels[levelLabels.length - 1]).toHaveTextContent("L0");
  });
});

describe("BPyramidTimeline SVG arrows", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset to default mock return value
    vi.mocked(usePreRenderedArrows).mockReturnValue({
      allArrowData: [],
      svgWidth: 0,
    });
  });

  it("should render arrow marker definition", () => {
    vi.mocked(usePreRenderedArrows).mockReturnValue({
      allArrowData: [
        {
          sourceFrameIndex: 4,
          targetFrameIndex: 0,
          slotIndex: 0,
          label: "L0",
          color: "#4444ff",
          pathData: "M0,0 L10,10",
          sourceX: 100,
          sourceY: 50,
          labelY: 45,
        },
      ],
      svgWidth: 1000,
    });

    render(<BPyramidTimeline {...defaultProps} />);

    const marker = document.querySelector("#bpyramid-arrowhead");
    expect(marker).toBeInTheDocument();
  });

  it("should show arrows only for current frame", () => {
    vi.mocked(usePreRenderedArrows).mockReturnValue({
      allArrowData: [
        {
          sourceFrameIndex: 4,
          targetFrameIndex: 0,
          slotIndex: 0,
          label: "L0",
          color: "#4444ff",
          pathData: "M0,0 L10,10",
          sourceX: 100,
          sourceY: 50,
          labelY: 45,
        },
      ],
      svgWidth: 1000,
    });

    render(<BPyramidTimeline {...defaultProps} currentFrameIndex={4} />);

    // Select path elements that are outside of defs (not the marker definition)
    const paths = document.querySelectorAll(
      ".bpyramid-arrows-overlay > g > path",
    );
    // Each arrow creates a <g> with a <path> inside
    expect(paths.length).toBe(1);
    // The arrow should be visible when current frame matches source
    expect(paths[0]).toHaveAttribute("visibility", "visible");
  });

  it("should hide arrows when not on current frame", () => {
    vi.mocked(usePreRenderedArrows).mockReturnValue({
      allArrowData: [
        {
          sourceFrameIndex: 4,
          targetFrameIndex: 0,
          slotIndex: 0,
          label: "L0",
          color: "#4444ff",
          pathData: "M0,0 L10,10",
          sourceX: 100,
          sourceY: 50,
          labelY: 45,
        },
      ],
      svgWidth: 1000,
    });

    render(<BPyramidTimeline {...defaultProps} currentFrameIndex={2} />);

    // Select path elements that are outside of defs (not the marker definition)
    const paths = document.querySelectorAll(
      ".bpyramid-arrows-overlay > g > path",
    );
    const path = paths[0];
    // Arrow is hidden via visibility attribute when current frame doesn't match
    expect(path).toHaveAttribute("visibility", "hidden");
  });

  it("should show all arrows (dimmed for non-selected sources) when showAllArrows is true", () => {
    vi.mocked(usePreRenderedArrows).mockReturnValue({
      allArrowData: [
        {
          sourceFrameIndex: 4,
          targetFrameIndex: 0,
          slotIndex: 0,
          label: "L0",
          color: "#4444ff",
          pathData: "M0,0 L10,10",
          sourceX: 100,
          sourceY: 50,
          labelY: 45,
        },
        {
          sourceFrameIndex: 2,
          targetFrameIndex: 0,
          slotIndex: 0,
          label: "L0",
          color: "#4444ff",
          pathData: "M0,0 L20,20",
          sourceX: 60,
          sourceY: 30,
          labelY: 25,
        },
      ],
      svgWidth: 1000,
    });

    render(
      <BPyramidTimeline
        {...defaultProps}
        currentFrameIndex={4}
        showAllArrows
      />,
    );

    const paths = document.querySelectorAll(
      ".bpyramid-arrows-overlay > g > path",
    );
    expect(paths.length).toBe(2);
    // Both arrows are visible when showAllArrows is set...
    paths.forEach((path) => {
      expect(path).toHaveAttribute("visibility", "visible");
    });
    // ...but the non-selected source (frame 2) is dimmer than the selected one (frame 4).
    expect(paths[0]).toHaveAttribute("stroke-opacity", "0.7");
    expect(paths[1]).toHaveAttribute("stroke-opacity", "0.25");
  });

  it("should default showAllArrows to false (unchanged per-selection behavior)", () => {
    vi.mocked(usePreRenderedArrows).mockReturnValue({
      allArrowData: [
        {
          sourceFrameIndex: 2,
          targetFrameIndex: 0,
          slotIndex: 0,
          label: "L0",
          color: "#4444ff",
          pathData: "M0,0 L10,10",
          sourceX: 100,
          sourceY: 50,
          labelY: 45,
        },
      ],
      svgWidth: 1000,
    });

    render(<BPyramidTimeline {...defaultProps} currentFrameIndex={4} />);

    const path = document.querySelector(".bpyramid-arrows-overlay > g > path");
    expect(path).toHaveAttribute("visibility", "hidden");
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
    expect(result.frameMap.size).toBe(9);

    const frame0 = result.frameMap.get(0);
    expect(frame0?.level).toBe(0);
    expect(frame0?.isKeyframe).toBe(true);
    expect(frame0?.hasBackwardRef).toBe(false);

    // frame_index 1 (display_order 8) only references frame_index 0 (display_order 0) --
    // forward-only, so it's a level-0 anchor exactly like the keyframe, not a hierarchy-deep
    // frame. This is the case the old temporal_id-based leveling got wrong (it reported 2).
    const frame1 = result.frameMap.get(1);
    expect(frame1?.level).toBe(0);
    expect(frame1?.hasBackwardRef).toBe(false);
  });

  it("should derive hierarchy depth from the reference graph, not temporal_id (real dyadic B-pyramid shape)", () => {
    const result = analyzeTemporalLevels(mockFrames);

    // See mockFrames' header comment for the derivation of this exact table.
    const expected: Record<number, { level: number; hasBackwardRef: boolean }> =
      {
        0: { level: 0, hasBackwardRef: false },
        1: { level: 0, hasBackwardRef: false },
        2: { level: 1, hasBackwardRef: true },
        3: { level: 2, hasBackwardRef: true },
        4: { level: 2, hasBackwardRef: true },
        5: { level: 3, hasBackwardRef: true },
        6: { level: 3, hasBackwardRef: true },
        7: { level: 3, hasBackwardRef: true },
        8: { level: 3, hasBackwardRef: true },
      };

    for (const [frameIndex, exp] of Object.entries(expected)) {
      const frame = result.frameMap.get(Number(frameIndex));
      expect(frame?.level).toBe(exp.level);
      expect(frame?.hasBackwardRef).toBe(exp.hasBackwardRef);
    }
  });

  it("should organize levels in descending order", () => {
    const result = analyzeTemporalLevels(mockFrames);

    expect(result.levels.length).toBe(4);
    expect(result.levels[0].level).toBe(3);
    expect(result.levels[1].level).toBe(2);
    expect(result.levels[2].level).toBe(1);
    expect(result.levels[3].level).toBe(0);
  });

  it("should default to level 0 when there is no display_order or temporal_id", () => {
    const framesWithoutTemporal = [
      { frame_index: 0, frame_type: "I", size: 50000 },
    ] as FrameInfo[];

    const result = analyzeTemporalLevels(framesWithoutTemporal);

    const frame = result.frameMap.get(0);
    expect(frame?.level).toBe(0);
    expect(frame?.hasBackwardRef).toBe(false);
  });

  it("should fall back to temporal_id only when the ref-graph method has no display_order to work with", () => {
    const frames = [
      { frame_index: 0, frame_type: "I", size: 1000, key_frame: true },
      { frame_index: 1, frame_type: "P", size: 1000, temporal_id: 2 },
    ] as FrameInfo[];

    const result = analyzeTemporalLevels(frames);

    const frame1 = result.frameMap.get(1);
    expect(frame1?.level).toBe(2);
    // Still honestly "we don't know it's backward-referenced" -- the SVC hint sets the level
    // number, but isn't proof of hierarchical (backward) prediction.
    expect(frame1?.hasBackwardRef).toBe(false);
  });

  it("should not infer a backward reference when the referenced frame's display_order is unknown", () => {
    const frames = [
      {
        frame_index: 0,
        frame_type: "I",
        size: 1000,
        display_order: 0,
        key_frame: true,
      },
      {
        frame_index: 1,
        frame_type: "P",
        size: 1000,
        display_order: 5,
        ref_frames: [0, 2],
      },
      // frame_index 2 has no display_order -- frame 1's reference to it can't be proven
      // backward, so it must not be fabricated as one.
      { frame_index: 2, frame_type: "P", size: 1000, ref_frames: [0] },
    ] as FrameInfo[];

    const result = analyzeTemporalLevels(frames);

    const frame1 = result.frameMap.get(1);
    expect(frame1?.hasBackwardRef).toBe(false);
    expect(frame1?.level).toBe(0);
  });

  it("should identify I frames as keyframes", () => {
    const result = analyzeTemporalLevels(mockFrames);

    const frame0 = result.frameMap.get(0);
    expect(frame0?.isKeyframe).toBe(true);

    const frame1 = result.frameMap.get(1);
    expect(frame1?.isKeyframe).toBe(false);
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
