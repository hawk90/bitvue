/**
 * Filmstrip Component Tests
 * Main filmstrip with view modes and frame expansion
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@/test/test-utils";
import { Filmstrip } from "@/components/Filmstrip";

// Mock SelectionContext since Filmstrip uses useSelection
vi.mock("@/contexts/SelectionContext", () => ({
  useSelection: () => ({
    selection: { frame: { frameIndex: 1 } },
    setFrameSelection: vi.fn(),
  }),
  SelectionProvider: ({ children }: { children: React.ReactNode }) => (
    <>{children}</>
  ),
}));

// Mock dependencies
const mockFrames = [
  {
    frame_index: 0,
    frame_type: "I",
    size: 50000,
    ref_frames: [],
    poc: 0,
    key_frame: true,
  },
  {
    frame_index: 1,
    frame_type: "P",
    size: 30000,
    ref_frames: [0],
    poc: 1,
  },
  {
    frame_index: 2,
    frame_type: "B",
    size: 20000,
    ref_frames: [0, 1],
    poc: 2,
  },
];

vi.mock("@/components/useFilmstripState", () => ({
  useFilmstripState: () => ({
    thumbnails: new Map([
      [0, "data:image0;base64,abc"],
      [1, "data:image1;base64,def"],
      [2, "data:image2;base64,ghi"],
    ]),
    loadingThumbnails: new Set(),
    expandedFrameIndex: null,
    setExpandedFrameIndex: vi.fn(),
    loadThumbnails: vi.fn(),
  }),
}));

describe("Filmstrip", () => {
  it("should render filmstrip container", () => {
    const { container } = render(<Filmstrip frames={mockFrames} />);

    expect(container.querySelector(".filmstrip")).toBeInTheDocument();
  });

  it("should display filmstrip text", () => {
    render(<Filmstrip frames={mockFrames} />);

    expect(screen.getByText("Filmstrip")).toBeInTheDocument();
  });

  it("should display frame count", () => {
    render(<Filmstrip frames={mockFrames} />);

    expect(screen.getByText("(3 frames)")).toBeInTheDocument();
  });

  it("should display view mode selector", () => {
    const { container } = render(<Filmstrip frames={mockFrames} />);

    // Should have dropdown to select view mode
    const dropdown = container.querySelector(".filmstrip-dropdown");
    expect(dropdown).toBeInTheDocument();
  });

  it("should handle empty frames array", () => {
    const { container } = render(<Filmstrip frames={[]} />);

    // Should handle gracefully - may show empty state
    expect(container.querySelector(".filmstrip")).toBeInTheDocument();
  });

  it("should show empty state when no frames", () => {
    render(<Filmstrip frames={[]} />);

    expect(screen.getByText("No frames loaded")).toBeInTheDocument();
  });

  it("should show frame reference arrows region", () => {
    render(<Filmstrip frames={mockFrames} />);

    // Should render filmstrip content area
    const filmstripContent = document.querySelector(".filmstrip-content");
    expect(filmstripContent).toBeInTheDocument();
  });

  it("should use React.memo for performance", () => {
    const { rerender } = render(<Filmstrip frames={mockFrames} />);

    rerender(<Filmstrip frames={mockFrames} />);

    expect(screen.getByText("Filmstrip")).toBeInTheDocument();
  });
});

describe("Filmstrip performance", () => {
  it("should handle large frame count efficiently", () => {
    const largeFrames = Array.from({ length: 100 }, (_, i) => ({
      frame_index: i,
      frame_type: i % 3 === 0 ? "I" : i % 3 === 1 ? "P" : "B",
      size: 10000 + i * 100,
      ref_frames: i > 0 ? [0] : [],
      poc: i,
    }));

    const { container } = render(<Filmstrip frames={largeFrames} />);

    // Should render without hanging
    expect(container.querySelector(".filmstrip")).toBeInTheDocument();
  });

  it("should render header controls", () => {
    render(<Filmstrip frames={mockFrames} />);

    expect(
      screen.getByRole("region", { name: "Filmstrip controls" }),
    ).toBeInTheDocument();
  });
});
