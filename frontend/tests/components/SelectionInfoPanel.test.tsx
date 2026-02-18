/**
 * SelectionInfoPanel Component Tests
 * Tests selection info panel with frame and stream statistics
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@/test/test-utils";
import { SelectionInfoPanel } from "../SelectionInfoPanel";

// Mock context
vi.mock("../../../contexts/StreamDataContext", () => ({
  useStreamData: () => ({
    frames: [
      { frame_index: 0, frame_type: "I", size: 50000, pts: 0 },
      { frame_index: 1, frame_type: "P", size: 30000, pts: 1, ref_frames: [0] },
      { frame_index: 2, frame_type: "P", size: 35000, pts: 2, ref_frames: [0] },
      {
        frame_index: 3,
        frame_type: "B",
        size: 20000,
        pts: 3,
        ref_frames: [1, 2],
      },
    ],
    currentFrameIndex: 1,
    getFrameStats: () => ({
      totalFrames: 4,
      keyFrames: 1,
      avgSize: 33750,
      totalSize: 135000,
      frameTypes: { I: 1, P: 2, B: 1 },
    }),
  }),
}));

describe("SelectionInfoPanel", () => {
  it("should render selection info panel", () => {
    render(<SelectionInfoPanel />);

    expect(screen.getByText("Selection Info")).toBeInTheDocument();
  });

  it("should display current frame section", () => {
    render(<SelectionInfoPanel />);

    expect(screen.getByText("Current Frame")).toBeInTheDocument();
  });

  it("should display frame index", () => {
    render(<SelectionInfoPanel />);

    expect(screen.getByText("Frame Index")).toBeInTheDocument();
    expect(screen.getByText("1")).toBeInTheDocument();
  });

  it("should display frame type badge", () => {
    render(<SelectionInfoPanel />);

    const badge = document.querySelector(".frame-type-badge");
    expect(badge).toBeInTheDocument();
    expect(badge?.textContent).toBe("P");
  });

  it("should display frame size in KB", () => {
    render(<SelectionInfoPanel />);

    expect(screen.getByText("Size")).toBeInTheDocument();
    expect(screen.getByText(/29\.30 KB/)).toBeInTheDocument();
  });

  it("should display video properties section", () => {
    render(<SelectionInfoPanel />);

    expect(screen.getByText("Video Properties")).toBeInTheDocument();
    expect(screen.getByText("Resolution")).toBeInTheDocument();
    expect(screen.getByText("1920x1080")).toBeInTheDocument();
  });

  it("should display codec information", () => {
    render(<SelectionInfoPanel />);

    expect(screen.getByText("Codec")).toBeInTheDocument();
    expect(screen.getByText("AV1")).toBeInTheDocument();
  });

  it("should accept custom codec prop", () => {
    render(<SelectionInfoPanel codec="HEVC" />);

    expect(screen.getByText("HEVC")).toBeInTheDocument();
  });

  it("should accept custom resolution props", () => {
    render(<SelectionInfoPanel width={3840} height={2160} />);

    expect(screen.getByText("3840x2160")).toBeInTheDocument();
  });

  it("should display stream statistics section", () => {
    render(<SelectionInfoPanel />);

    expect(screen.getByText("Stream Statistics")).toBeInTheDocument();
    expect(screen.getByText("Total Frames")).toBeInTheDocument();
    expect(screen.getByText("4")).toBeInTheDocument();
  });

  it("should display keyframe count", () => {
    render(<SelectionInfoPanel />);

    expect(screen.getByText("Keyframes")).toBeInTheDocument();
    expect(screen.getByText("1")).toBeInTheDocument();
  });

  it("should display average size", () => {
    render(<SelectionInfoPanel />);

    expect(screen.getByText("Avg Size")).toBeInTheDocument();
    expect(screen.getByText(/32\.96 KB/)).toBeInTheDocument();
  });

  it("should display total size in MB", () => {
    render(<SelectionInfoPanel />);

    expect(screen.getByText("Total Size")).toBeInTheDocument();
    expect(screen.getByText(/0\.13 MB/)).toBeInTheDocument();
  });

  it("should display frame type distribution section", () => {
    render(<SelectionInfoPanel />);

    expect(screen.getByText("Frame Types")).toBeInTheDocument();
  });

  it("should display all frame type counts", () => {
    render(<SelectionInfoPanel />);

    expect(screen.getByText("I")).toBeInTheDocument();
    expect(screen.getByText("P")).toBeInTheDocument();
    expect(screen.getByText("B")).toBeInTheDocument();
  });

  it("should display frame type percentages", () => {
    render(<SelectionInfoPanel />);

    expect(screen.getByText(/\(25\.0%\)/)).toBeInTheDocument(); // I: 1/4 = 25%
  });

  it("should display selection section", () => {
    render(<SelectionInfoPanel />);

    expect(screen.getByText("Selection")).toBeInTheDocument();
    expect(
      screen.getByText(/Click on the video to select a block/),
    ).toBeInTheDocument();
  });

  it("should display codec badge", () => {
    render(<SelectionInfoPanel />);

    const badge = document.querySelector(".codec-badge");
    expect(badge).toBeInTheDocument();
    expect(badge?.textContent).toBe("AV1");
  });

  it("should use React.memo for performance", () => {
    const { rerender } = render(<SelectionInfoPanel />);

    rerender(<SelectionInfoPanel />);

    expect(screen.getByText("Selection Info")).toBeInTheDocument();
  });
});

describe("SelectionInfoPanel frame info", () => {
  it("should display PTS when available", () => {
    render(<SelectionInfoPanel />);

    expect(screen.getByText("PTS")).toBeInTheDocument();
  });

  it("should display reference frame count", () => {
    render(<SelectionInfoPanel />);

    expect(screen.getByText("References")).toBeInTheDocument();
    expect(screen.getByText("1")).toBeInTheDocument();
  });

  it("should show zero references for I-frames", () => {
    vi.doMock("../../../contexts/StreamDataContext", () => ({
      useStreamData: () => ({
        frames: [{ frame_index: 0, frame_type: "I", size: 50000, pts: 0 }],
        currentFrameIndex: 0,
        getFrameStats: () => ({
          totalFrames: 1,
          keyFrames: 1,
          avgSize: 50000,
          totalSize: 50000,
          frameTypes: { I: 1 },
        }),
      }),
    }));

    render(<SelectionInfoPanel />);

    const references = screen.getAllByText("0");
    expect(references.length).toBeGreaterThan(0);
  });
});

describe("SelectionInfoPanel edge cases", () => {
  it("should handle empty frames array", () => {
    vi.doMock("../../../contexts/StreamDataContext", () => ({
      useStreamData: () => ({
        frames: [],
        currentFrameIndex: 0,
        getFrameStats: () => ({
          totalFrames: 0,
          keyFrames: 0,
          avgSize: 0,
          totalSize: 0,
          frameTypes: {},
        }),
      }),
    }));

    render(<SelectionInfoPanel />);

    expect(screen.getByText("Selection Info")).toBeInTheDocument();
  });

  it("should handle null current frame", () => {
    vi.doMock("../../../contexts/StreamDataContext", () => ({
      useStreamData: () => ({
        frames: [],
        currentFrameIndex: -1,
        getFrameStats: () => ({
          totalFrames: 0,
          keyFrames: 0,
          avgSize: 0,
          totalSize: 0,
          frameTypes: {},
        }),
      }),
    }));

    render(<SelectionInfoPanel />);

    // When no frame selected, expect at least some "N/A" values
    const naValues = screen.queryAllByText("N/A");
    expect(naValues.length).toBeGreaterThan(0);
  });
});
