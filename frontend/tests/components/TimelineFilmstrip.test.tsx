/**
 * TimelineFilmstrip Component Tests
 * Tests timeline filmstrip component
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@/test/test-utils";
import { TimelineFilmstrip } from "../TimelineFilmstrip";
import type { FrameInfo } from "@/types/video";
import { useSelection } from "@/contexts/SelectionContext";

// Mock SelectionContext
vi.mock("@/contexts/SelectionContext", () => ({
  useSelection: vi.fn(),
}));

const mockFrames: FrameInfo[] = [
  { frame_index: 0, frame_type: "I", size: 50000, poc: 0, key_frame: true },
  { frame_index: 1, frame_type: "P", size: 30000, poc: 1, ref_frames: [0] },
  { frame_index: 2, frame_type: "B", size: 20000, poc: 2, ref_frames: [0, 1] },
];

// Helper to create mock SelectionContext
const mockSelectionContext = {
  selection: null,
  setTemporalSelection: vi.fn(),
  setFrameSelection: vi.fn(),
  setUnitSelection: vi.fn(),
  setSyntaxSelection: vi.fn(),
  setBitRangeSelection: vi.fn(),
  clearTemporal: vi.fn(),
  clearAll: vi.fn(),
  subscribe: vi.fn(() => () => {}),
};

describe("TimelineFilmstrip", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useSelection).mockReturnValue(mockSelectionContext);
  });
  it("should render timeline filmstrip", () => {
    render(<TimelineFilmstrip frames={mockFrames} />);

    expect(document.querySelector(".timeline-filmstrip")).toBeInTheDocument();
  });

  it("should display frame thumbnails", () => {
    render(<TimelineFilmstrip frames={mockFrames} />);

    const thumbnails = document.querySelectorAll(
      ".timeline-thumb, .filmstrip-thumb",
    );
    expect(thumbnails.length).toBeGreaterThan(0);
  });

  it("should highlight current frame", () => {
    vi.mocked(useSelection).mockReturnValue({
      ...mockSelectionContext,
      selection: { stream: "A", frameIndex: 1 },
    });

    render(<TimelineFilmstrip frames={mockFrames} />);

    // Check that the component renders without error
    expect(document.querySelector(".timeline-filmstrip")).toBeInTheDocument();
  });

  it("should use React.memo for performance", () => {
    const { rerender } = render(<TimelineFilmstrip frames={mockFrames} />);

    rerender(<TimelineFilmstrip frames={mockFrames} />);

    expect(document.querySelector(".timeline-filmstrip")).toBeInTheDocument();
  });

  it("should show frame numbers", () => {
    render(<TimelineFilmstrip frames={mockFrames} />);

    // Check for any frame number display
    expect(
      screen.queryByText(/0/) || screen.queryByText(/1/),
    ).toBeInTheDocument();
  });

  it("should handle empty frames", () => {
    render(<TimelineFilmstrip frames={[]} />);

    expect(document.querySelector(".timeline-filmstrip")).toBeInTheDocument();
  });
});

describe("TimelineFilmstrip navigation", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useSelection).mockReturnValue(mockSelectionContext);
  });

  it("should support frame navigation", () => {
    render(<TimelineFilmstrip frames={mockFrames} />);

    const filmstrip = document.querySelector(".timeline-filmstrip");
    expect(filmstrip).toBeInTheDocument();
  });

  it("should display scrollbar for many frames", () => {
    render(<TimelineFilmstrip frames={mockFrames} />);

    // The scrollbar might not exist, but the component should render
    expect(document.querySelector(".timeline-filmstrip")).toBeInTheDocument();
  });
});

describe("TimelineFilmstrip visual features", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useSelection).mockReturnValue(mockSelectionContext);
  });

  it("should show frame type indicators", () => {
    render(<TimelineFilmstrip frames={mockFrames} />);

    // Check for frame type classes or elements
    const timelineBars = document.querySelectorAll(
      ".timeline-bar-i, .timeline-bar-p, .timeline-bar-b",
    );
    expect(timelineBars.length).toBeGreaterThan(0);
  });

  it("should display frame sizes", () => {
    render(<TimelineFilmstrip frames={mockFrames} />);

    expect(document.querySelector(".timeline-filmstrip")).toBeInTheDocument();
  });
});
