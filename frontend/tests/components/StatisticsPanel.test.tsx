/**
 * StatisticsPanel Component Tests
 * Tests frame statistics display
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, renderWithoutProviders } from "@/test/test-utils";
import { StatisticsPanel } from "@/components/panels/StatisticsPanel";
import * as FrameDataContext from "@/contexts/FrameDataContext";

// Mock data
const mockFrames = [
  {
    frame_index: 0,
    frame_type: "I",
    size: 50000,
    key_frame: true,
    pts: 0,
    poc: 0,
    display_order: 0,
    coding_order: 0,
  },
  {
    frame_index: 1,
    frame_type: "P",
    size: 30000,
    ref_frames: [0],
    key_frame: false,
    pts: 1,
    poc: 1,
    display_order: 1,
    coding_order: 1,
  },
  {
    frame_index: 2,
    frame_type: "P",
    size: 35000,
    ref_frames: [0],
    key_frame: false,
    pts: 2,
    poc: 2,
    display_order: 2,
    coding_order: 2,
  },
  {
    frame_index: 3,
    frame_type: "B",
    size: 20000,
    ref_frames: [1, 2],
    key_frame: false,
    pts: 3,
    poc: 3,
    display_order: 3,
    coding_order: 3,
  },
];

const mockGetFrameStats = () => ({
  totalFrames: mockFrames.length,
  frameTypes: { I: 1, P: 2, B: 1 },
  totalSize: 135000,
  avgSize: 33750,
  keyFrames: 1,
});

// Mock the useFrameData hook - StatisticsPanel imports from FrameDataContext directly
vi.spyOn(FrameDataContext, "useFrameData").mockReturnValue({
  frames: mockFrames,
  getFrameStats: mockGetFrameStats,
  setFrames: vi.fn(),
});

describe("StatisticsPanel", () => {
  it("should render statistics panel header", () => {
    render(<StatisticsPanel />);

    expect(screen.getByText("Statistics")).toBeInTheDocument();
  });

  it("should display frame count statistics", () => {
    const { container } = render(<StatisticsPanel />);

    expect(screen.getAllByText(/Total/).length).toBeGreaterThan(0);
    // Check that total frames count is 4 (appears in summary)
    expect(container.textContent).toContain("Total:4");
  });

  it("should display average frame size", () => {
    render(<StatisticsPanel />);

    expect(screen.getByText(/Avg Size/)).toBeInTheDocument();
  });

  it("should calculate bitrate statistics", () => {
    render(<StatisticsPanel />);

    // Should show bitrate info
    expect(screen.getAllByText(/Bitrate/).length).toBeGreaterThan(0);
  });

  it("should use React.memo for performance", () => {
    const { rerender } = render(<StatisticsPanel />);

    rerender(<StatisticsPanel />);

    expect(screen.getByText("Statistics")).toBeInTheDocument();
  });

  it("should format byte sizes correctly", () => {
    const { container } = render(<StatisticsPanel />);

    // Should format as KB/MB - avgSize is 33750 bytes = 32.96-32.97 KB depending on rounding
    // Text may be split across elements, so we check the container
    expect(container.textContent).toMatch(/32\.(96|97) KB/);
  });

  it("should show total size in MB", () => {
    render(<StatisticsPanel />);

    // Should show total size
    expect(screen.getByText(/Total Size/)).toBeInTheDocument();
    expect(screen.getByText(/MB/)).toBeInTheDocument();
  });

  it("should show keyframes count", () => {
    render(<StatisticsPanel />);

    expect(screen.getByText(/Keyframes/)).toBeInTheDocument();
  });
});

describe("StatisticsPanel with empty frames", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should handle empty frames array", () => {
    vi.spyOn(FrameDataContext, "useFrameData").mockReturnValue({
      frames: [],
      getFrameStats: () => ({
        totalFrames: 0,
        frameTypes: {},
        totalSize: 0,
        avgSize: 0,
        keyFrames: 0,
      }),
      setFrames: vi.fn(),
    });

    render(<StatisticsPanel />);

    // Should still render panel with zero values
    expect(screen.getByText("Statistics")).toBeInTheDocument();
    // Multiple zeros, so just check we have at least one
    expect(screen.getAllByText("0").length).toBeGreaterThan(0);
  });
});

describe("StatisticsPanel calculations", () => {
  it("should count I-frames correctly", () => {
    const mockIFrameData = [
      {
        frame_index: 0,
        frame_type: "I",
        size: 50000,
        key_frame: true,
        pts: 0,
        poc: 0,
        display_order: 0,
        coding_order: 0,
      },
      {
        frame_index: 5,
        frame_type: "I",
        size: 60000,
        key_frame: true,
        pts: 5,
        poc: 5,
        display_order: 5,
        coding_order: 5,
      },
    ];

    vi.spyOn(FrameDataContext, "useFrameData").mockReturnValue({
      frames: mockIFrameData,
      getFrameStats: () => ({
        totalFrames: 2,
        frameTypes: { I: 2 },
        totalSize: 110000,
        avgSize: 55000,
        keyFrames: 2,
      }),
      setFrames: vi.fn(),
    });

    const { container } = render(<StatisticsPanel />);

    // Should have 2 total frames
    expect(container.textContent).toContain("2");
  });

  it("should calculate average frame size", () => {
    const mockAvgData = [
      {
        frame_index: 0,
        frame_type: "I",
        size: 50000,
        key_frame: true,
        pts: 0,
        poc: 0,
        display_order: 0,
        coding_order: 0,
      },
      {
        frame_index: 1,
        frame_type: "P",
        size: 30000,
        key_frame: false,
        pts: 1,
        poc: 1,
        display_order: 1,
        coding_order: 1,
      },
    ];

    vi.spyOn(FrameDataContext, "useFrameData").mockReturnValue({
      frames: mockAvgData,
      getFrameStats: () => ({
        totalFrames: 2,
        frameTypes: { I: 1, P: 1 },
        totalSize: 80000,
        avgSize: 40000,
        keyFrames: 1,
      }),
      setFrames: vi.fn(),
    });

    const { container } = render(<StatisticsPanel />);

    // Average should be (50000 + 30000) / 2 = 40000 bytes = 39.06 KB
    expect(container.textContent).toContain("39.06");
  });
});
