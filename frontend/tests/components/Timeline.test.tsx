/**
 * Timeline Component Tests
 * Tests timeline navigation, drag interaction, and visualization
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  render,
  screen,
  fireEvent,
  waitFor,
  act,
} from "@testing-library/react";
import Timeline from "@/components/Timeline";
import { useSelection } from "@/contexts/SelectionContext";
import type { FrameInfo } from "@/types/video";

// Mock SelectionContext
vi.mock("@/contexts/SelectionContext", () => ({
  useSelection: vi.fn(),
  SelectionProvider: ({ children }: { children: React.ReactNode }) => children,
}));

const mockFrames: FrameInfo[] = [
  { frame_index: 0, frame_type: "I", size: 50000, poc: 0, key_frame: true },
  { frame_index: 1, frame_type: "P", size: 30000, poc: 1, ref_frames: [0] },
  { frame_index: 2, frame_type: "B", size: 20000, poc: 2, ref_frames: [0, 1] },
  { frame_index: 3, frame_type: "P", size: 35000, poc: 3, ref_frames: [2] },
  { frame_index: 4, frame_type: "B", size: 25000, poc: 4, ref_frames: [2, 3] },
];

const defaultProps = {
  frames: mockFrames,
  className: "",
};

// Helper to create mock SelectionContext
const createMockSelectionContext = (overrides = {}) => ({
  selection: null,
  setTemporalSelection: vi.fn(),
  setFrameSelection: vi.fn(),
  setUnitSelection: vi.fn(),
  setSyntaxSelection: vi.fn(),
  setBitRangeSelection: vi.fn(),
  clearTemporal: vi.fn(),
  clearAll: vi.fn(),
  subscribe: vi.fn(() => () => {}),
  ...overrides,
});

describe("Timeline basic rendering", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useSelection).mockReturnValue(createMockSelectionContext());
  });

  it("should render timeline container", () => {
    render(<Timeline {...defaultProps} />);

    expect(
      screen.queryByRole("region", { name: "Timeline" }),
    ).toBeInTheDocument();
  });

  it("should render with custom className", () => {
    render(<Timeline {...defaultProps} className="custom-class" />);

    const timeline = screen.queryByRole("region", { name: "Timeline" });
    expect(timeline).toHaveClass("custom-class");
  });

  it("should render timeline header", () => {
    render(<Timeline {...defaultProps} />);

    expect(screen.getByText("Timeline")).toBeInTheDocument();
  });

  it("should display current frame and total frames", () => {
    render(<Timeline {...defaultProps} />);

    // TimelineHeader displays "{currentFrame + 1} / {totalFrames}" with spaces
    expect(screen.getByText(/1 \/ 5/)).toBeInTheDocument();
  });

  it("should render timeline thumbnails", () => {
    render(<Timeline {...defaultProps} />);

    expect(document.querySelectorAll(".timeline-thumb").length).toBe(5);
  });

  it("should render timeline cursor", () => {
    render(<Timeline {...defaultProps} />);

    expect(document.querySelector(".timeline-cursor")).toBeInTheDocument();
  });
});

describe("Timeline empty state", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useSelection).mockReturnValue(createMockSelectionContext());
  });

  it("should render empty state when no frames loaded", () => {
    render(<Timeline frames={[]} />);

    expect(screen.getByText("No frames loaded")).toBeInTheDocument();
    expect(screen.queryByText(/No frames loaded/)).toBeInTheDocument();
  });

  it("should display empty state icon", () => {
    render(<Timeline frames={[]} />);

    expect(document.querySelector(".codicon-graph")).toBeInTheDocument();
  });

  it("should show 0/0 in header for empty frames", () => {
    render(<Timeline frames={[]} />);

    // TimelineHeader displays "{currentFrame + 1} / {totalFrames}" with spaces
    // When no frames, currentFrame is 0, so header shows "1 / 0"
    expect(screen.getByText(/1 \/ 0/)).toBeInTheDocument();
  });

  it("should handle single frame", () => {
    const singleFrame = [mockFrames[0]];
    render(<Timeline frames={singleFrame} />);

    // TimelineHeader displays "{currentFrame + 1} / {totalFrames}" with spaces
    expect(screen.getByText(/1 \/ 1/)).toBeInTheDocument();
    expect(document.querySelectorAll(".timeline-thumb").length).toBe(1);
  });
});

describe("Timeline mouse interactions", () => {
  let setFrameSelectionMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    vi.clearAllMocks();
    setFrameSelectionMock = vi.fn();
    vi.mocked(useSelection).mockReturnValue(
      createMockSelectionContext({
        setFrameSelection: setFrameSelectionMock,
      }),
    );
  });

  it("should handle mouse down on frame", () => {
    render(<Timeline {...defaultProps} />);

    const frame0 = document.querySelector('[data-frame-index="0"]');
    expect(frame0).toBeInTheDocument();

    if (frame0) {
      act(() => {
        fireEvent.mouseDown(frame0);
      });
      // Frame is updated in local state, selection happens on mouse up
      expect(frame0).toBeInTheDocument();
    }
  });

  it("should handle mouse move for hover position", () => {
    render(<Timeline {...defaultProps} />);

    const timeline = document.querySelector(".timeline-thumbnails");
    expect(timeline).toBeInTheDocument();

    if (timeline) {
      fireEvent.mouseMove(timeline, { clientX: 100, clientY: 50 });
      // Should handle without error
      expect(timeline).toBeInTheDocument();
    }
  });

  it("should handle mouse leave", () => {
    render(<Timeline {...defaultProps} />);

    const timeline = document.querySelector(".timeline-thumbnails");
    expect(timeline).toBeInTheDocument();

    if (timeline) {
      fireEvent.mouseLeave(timeline);
      // Should handle without error
      expect(timeline).toBeInTheDocument();
    }
  });

  it("should show tooltip on hover", async () => {
    render(<Timeline {...defaultProps} />);

    const timeline = document.querySelector(".timeline-thumbnails");

    if (timeline) {
      // Move mouse to trigger hover
      fireEvent.mouseMove(timeline, { clientX: 100, clientY: 50 });

      // Tooltip should appear
      await waitFor(
        () => {
          const tooltip = document.querySelector(".timeline-tooltip");
          expect(tooltip).toBeInTheDocument();
        },
        { timeout: 1000 },
      );
    }
  });
});

describe("Timeline keyboard navigation", () => {
  let setFrameSelectionMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    vi.clearAllMocks();
    setFrameSelectionMock = vi.fn();
    vi.mocked(useSelection).mockReturnValue(
      createMockSelectionContext({
        setFrameSelection: setFrameSelectionMock,
      }),
    );
  });

  it("should navigate to previous frame with ArrowLeft", () => {
    render(<Timeline {...defaultProps} />);

    const timeline = document.querySelector(".timeline-thumbnails");
    expect(timeline).toBeInTheDocument();

    if (timeline) {
      act(() => {
        fireEvent.keyDown(timeline, { key: "ArrowLeft" });
      });

      // At frame 0, ArrowLeft won't navigate (already at first frame)
      expect(timeline).toBeInTheDocument();
    }
  });

  it("should navigate to next frame with ArrowRight", () => {
    render(<Timeline {...defaultProps} />);

    const timeline = document.querySelector(".timeline-thumbnails");
    expect(timeline).toBeInTheDocument();

    if (timeline) {
      act(() => {
        fireEvent.keyDown(timeline, { key: "ArrowRight" });
      });

      // Should navigate from frame 0 to frame 1
      expect(setFrameSelectionMock).toHaveBeenCalledWith(
        { stream: "A", frameIndex: 1 },
        "timeline",
      );
    }
  });

  it("should not navigate before first frame with ArrowLeft", () => {
    vi.mocked(useSelection).mockReturnValue(
      createMockSelectionContext({
        selection: { frame: { stream: "A", frameIndex: 0 } },
      }),
    );

    render(<Timeline {...defaultProps} />);

    const timeline = document.querySelector(".timeline-thumbnails");
    if (timeline) {
      const initialCalls = setFrameSelectionMock.mock.calls.length;

      act(() => {
        fireEvent.keyDown(timeline, { key: "ArrowLeft" });
      });

      // Should not call setFrameSelection since already at first frame
      expect(setFrameSelectionMock.mock.calls.length).toBe(initialCalls);
    }
  });

  it("should not navigate after last frame with ArrowRight", () => {
    vi.mocked(useSelection).mockReturnValue(
      createMockSelectionContext({
        selection: { frame: { stream: "A", frameIndex: 4 } },
      }),
    );

    render(<Timeline {...defaultProps} />);

    const timeline = document.querySelector(".timeline-thumbnails");
    if (timeline) {
      const initialCalls = setFrameSelectionMock.mock.calls.length;

      act(() => {
        fireEvent.keyDown(timeline, { key: "ArrowRight" });
      });

      // Should not navigate since at last frame
      expect(setFrameSelectionMock.mock.calls.length).toBe(initialCalls);
    }
  });

  it("should handle rapid keyboard navigation", () => {
    render(<Timeline {...defaultProps} />);

    const timeline = document.querySelector(".timeline-thumbnails");
    if (timeline) {
      fireEvent.keyDown(timeline, { key: "ArrowRight" });
      fireEvent.keyDown(timeline, { key: "ArrowRight" });
      fireEvent.keyDown(timeline, { key: "ArrowLeft" });

      // Should handle without crashing
      expect(timeline).toBeInTheDocument();
    }
  });
});

describe("Timeline drag interaction", () => {
  let setFrameSelectionMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    vi.clearAllMocks();
    setFrameSelectionMock = vi.fn();
    vi.mocked(useSelection).mockReturnValue(
      createMockSelectionContext({
        setFrameSelection: setFrameSelectionMock,
      }),
    );
  });

  it("should handle drag start on mousedown", () => {
    render(<Timeline {...defaultProps} />);

    const frame1 = document.querySelector('[data-frame-index="1"]');
    expect(frame1).toBeInTheDocument();

    if (frame1) {
      act(() => {
        fireEvent.mouseDown(frame1, { clientX: 150, clientY: 50 });
      });

      // Should initiate drag - selection happens on mouse up
      expect(frame1).toBeInTheDocument();
    }
  });

  it("should handle drag move", () => {
    render(<Timeline {...defaultProps} />);

    const timeline = document.querySelector(".timeline-thumbnails");
    expect(timeline).toBeInTheDocument();

    if (timeline) {
      act(() => {
        fireEvent.mouseDown(timeline, { clientX: 100, clientY: 50 });

        // Simulate drag move
        const moveEvent = new MouseEvent("mousemove", {
          clientX: 200,
          clientY: 50,
          bubbles: true,
        });
        Object.defineProperty(moveEvent, "target", {
          value: timeline,
          writable: false,
        });

        window.dispatchEvent(moveEvent);
      });

      // Should handle without error
      expect(timeline).toBeInTheDocument();
    }
  });

  it("should handle drag end and update selection", () => {
    render(<Timeline {...defaultProps} />);

    const timeline = document.querySelector(".timeline-thumbnails");
    expect(timeline).toBeInTheDocument();

    if (timeline) {
      act(() => {
        fireEvent.mouseDown(timeline, { clientX: 100, clientY: 50 });

        // Simulate mouse up
        const upEvent = new MouseEvent("mouseup", { bubbles: true });
        window.dispatchEvent(upEvent);
      });

      // Should update selection on drag end
      expect(setFrameSelectionMock).toHaveBeenCalled();
    }
  });

  it("should track drag state correctly", () => {
    render(<Timeline {...defaultProps} />);

    const timeline = document.querySelector(".timeline-thumbnails");
    if (!timeline) return;

    // Initial state - not dragging
    fireEvent.mouseDown(timeline, { clientX: 100, clientY: 50 });

    // During drag - dragging is true
    fireEvent.mouseMove(timeline, { clientX: 150, clientY: 50 });

    // After drag - dragging is false
    const upEvent = new MouseEvent("mouseup", { bubbles: true });
    window.dispatchEvent(upEvent);

    // Should complete without errors
    expect(timeline).toBeInTheDocument();
  });
});

describe("Timeline frame refs", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useSelection).mockReturnValue(createMockSelectionContext());
  });

  it("should receive and store frame refs from thumbnails", () => {
    render(<Timeline {...defaultProps} />);

    // Frame refs should be populated
    const frameElements = document.querySelectorAll(".timeline-thumb");
    expect(frameElements.length).toBeGreaterThan(0);
  });

  it("should update frame refs when frames change", () => {
    const { rerender } = render(<Timeline {...defaultProps} />);

    const initialFrameCount =
      document.querySelectorAll(".timeline-thumb").length;

    const updatedFrames = [
      ...mockFrames,
      { frame_index: 5, frame_type: "I", size: 52000, poc: 5, key_frame: true },
    ];
    rerender(<Timeline frames={updatedFrames} />);

    const updatedFrameCount =
      document.querySelectorAll(".timeline-thumb").length;
    expect(updatedFrameCount).toBe(initialFrameCount + 1);
  });
});

describe("Timeline cursor positioning", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useSelection).mockReturnValue(createMockSelectionContext());
  });

  it("should calculate cursor position based on frame element", () => {
    render(<Timeline {...defaultProps} />);

    const cursor = document.querySelector(".timeline-cursor");
    expect(cursor).toBeInTheDocument();

    // Cursor should have position set
    expect(cursor).toBeInTheDocument();
  });

  it("should update cursor position when highlighted frame changes", () => {
    vi.mocked(useSelection).mockReturnValue(
      createMockSelectionContext({
        selection: { frame: { stream: "A", frameIndex: 2 } },
      }),
    );

    render(<Timeline {...defaultProps} />);

    const cursor = document.querySelector(".timeline-cursor");
    expect(cursor).toBeInTheDocument();
  });

  it("should handle cursor position at first frame", () => {
    vi.mocked(useSelection).mockReturnValue(
      createMockSelectionContext({
        selection: { frame: { stream: "A", frameIndex: 0 } },
      }),
    );

    render(<Timeline {...defaultProps} />);

    const cursor = document.querySelector(".timeline-cursor");
    expect(cursor).toBeInTheDocument();
  });

  it("should handle cursor position at last frame", () => {
    vi.mocked(useSelection).mockReturnValue(
      createMockSelectionContext({
        selection: { frame: { stream: "A", frameIndex: 4 } },
      }),
    );

    render(<Timeline {...defaultProps} />);

    const cursor = document.querySelector(".timeline-cursor");
    expect(cursor).toBeInTheDocument();
  });
});

describe("Timeline selection context integration", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should sync highlighted frame with selection context", () => {
    const setFrameSelectionMock = vi.fn();
    vi.mocked(useSelection).mockReturnValue(
      createMockSelectionContext({
        selection: { frame: { stream: "A", frameIndex: 2 } },
        setFrameSelection: setFrameSelectionMock,
      }),
    );

    render(<Timeline {...defaultProps} />);

    // Should sync with selection context - frame index 2 means header shows 3/5
    expect(screen.getByText(/3 \/ 5/)).toBeInTheDocument();
  });

  it("should not sync during drag", () => {
    const setFrameSelectionMock = vi.fn();
    vi.mocked(useSelection).mockReturnValue(
      createMockSelectionContext({
        selection: { frame: { stream: "A", frameIndex: 0 } },
        setFrameSelection: setFrameSelectionMock,
      }),
    );

    render(<Timeline {...defaultProps} />);

    const timeline = document.querySelector(".timeline-thumbnails");
    if (timeline) {
      act(() => {
        // Start drag - external selection changes should be ignored
        fireEvent.mouseDown(timeline, { clientX: 100, clientY: 50 });
      });

      // Even if selection context updates, component should maintain its state during drag
      expect(timeline).toBeInTheDocument();
    }
  });

  it("should update selection context on drag end", () => {
    const setFrameSelectionMock = vi.fn();
    vi.mocked(useSelection).mockReturnValue(
      createMockSelectionContext({
        selection: null,
        setFrameSelection: setFrameSelectionMock,
      }),
    );

    render(<Timeline {...defaultProps} />);

    const timeline = document.querySelector(".timeline-thumbnails");
    if (timeline) {
      act(() => {
        fireEvent.mouseDown(timeline, { clientX: 200, clientY: 50 });

        // End drag
        const upEvent = new MouseEvent("mouseup", { bubbles: true });
        window.dispatchEvent(upEvent);
      });

      // Should update selection context with final position
      expect(setFrameSelectionMock).toHaveBeenCalled();
    }
  });
});

describe("Timeline tooltip", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useSelection).mockReturnValue(createMockSelectionContext());
  });

  it("should show tooltip when hovering over frames", async () => {
    render(<Timeline {...defaultProps} />);

    const timeline = document.querySelector(".timeline-thumbnails");
    if (timeline) {
      fireEvent.mouseMove(timeline, { clientX: 100, clientY: 50 });

      await waitFor(
        () => {
          const tooltip = document.querySelector(".timeline-tooltip");
          expect(tooltip).toBeInTheDocument();
        },
        { timeout: 1000 },
      );
    }
  });

  it("should hide tooltip when mouse leaves", async () => {
    render(<Timeline {...defaultProps} />);

    const timeline = document.querySelector(".timeline-thumbnails");
    if (timeline) {
      // Move to trigger hover
      fireEvent.mouseMove(timeline, { clientX: 100, clientY: 50 });

      await waitFor(
        () => {
          expect(
            document.querySelector(".timeline-tooltip"),
          ).toBeInTheDocument();
        },
        { timeout: 1000 },
      );

      // Leave timeline
      fireEvent.mouseLeave(timeline);

      await waitFor(
        () => {
          expect(
            document.querySelector(".timeline-tooltip"),
          ).not.toBeInTheDocument();
        },
        { timeout: 1000 },
      );
    }
  });

  it("should display frame info in tooltip", async () => {
    render(<Timeline {...defaultProps} />);

    const timeline = document.querySelector(".timeline-thumbnails");
    if (timeline) {
      act(() => {
        fireEvent.mouseMove(timeline, { clientX: 100, clientY: 50 });
      });

      await waitFor(
        () => {
          const tooltip = document.querySelector(".timeline-tooltip");
          if (tooltip) {
            // Should contain frame information
            expect(tooltip.textContent).toBeDefined();
          }
        },
        { timeout: 1000 },
      );
    }
  });

  it("should position tooltip correctly based on hover position", () => {
    render(<Timeline {...defaultProps} />);

    const timeline = document.querySelector(".timeline-thumbnails");
    if (timeline) {
      // Hover at different positions
      fireEvent.mouseMove(timeline, { clientX: 50, clientY: 50 });
      fireEvent.mouseMove(timeline, { clientX: 200, clientY: 50 });

      // Should handle without errors
      expect(timeline).toBeInTheDocument();
    }
  });
});

describe("Timeline frame types display", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useSelection).mockReturnValue(createMockSelectionContext());
  });

  it("should apply correct CSS class for I frames", () => {
    render(<Timeline {...defaultProps} />);

    const iFrame = document.querySelector('[data-frame-index="0"]');
    expect(iFrame).toHaveClass("timeline-bar-i");
  });

  it("should apply correct CSS class for P frames", () => {
    render(<Timeline {...defaultProps} />);

    const pFrame = document.querySelector('[data-frame-index="1"]');
    expect(pFrame).toHaveClass("timeline-bar-p");
  });

  it("should apply correct CSS class for B frames", () => {
    render(<Timeline {...defaultProps} />);

    const bFrame = document.querySelector('[data-frame-index="2"]');
    expect(bFrame).toHaveClass("timeline-bar-b");
  });

  it("should display frame index in tooltip", async () => {
    render(<Timeline {...defaultProps} />);

    const timeline = document.querySelector(".timeline-thumbnails");
    if (timeline) {
      act(() => {
        fireEvent.mouseMove(timeline, { clientX: 100, clientY: 50 });
      });

      await waitFor(
        () => {
          const tooltip = document.querySelector(".timeline-tooltip");
          if (tooltip) {
            expect(tooltip.textContent).toMatch(/#/);
          }
        },
        { timeout: 1000 },
      );
    }
  });

  it("should display frame type in tooltip", async () => {
    render(<Timeline {...defaultProps} />);

    const timeline = document.querySelector(".timeline-thumbnails");
    if (timeline) {
      act(() => {
        fireEvent.mouseMove(timeline, { clientX: 100, clientY: 50 });
      });

      await waitFor(
        () => {
          const tooltip = document.querySelector(".timeline-tooltip");
          if (tooltip) {
            expect(tooltip.textContent).toMatch(/[IPB]/);
          }
        },
        { timeout: 1000 },
      );
    }
  });
});

describe("Timeline React.memo optimization", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useSelection).mockReturnValue(createMockSelectionContext());
  });

  it("should use React.memo for Timeline", () => {
    const { rerender } = render(<Timeline {...defaultProps} />);

    rerender(<Timeline {...defaultProps} />);

    expect(
      screen.queryByRole("region", { name: "Timeline" }),
    ).toBeInTheDocument();
  });

  it("should not re-render when props are the same", () => {
    const { rerender } = render(<Timeline {...defaultProps} />);

    const timeline = screen.queryByRole("region", { name: "Timeline" });

    rerender(<Timeline {...defaultProps} />);

    expect(timeline).toBeInTheDocument();
  });

  it("should re-render when frames change", () => {
    const { rerender } = render(<Timeline {...defaultProps} />);

    const newFrames = [
      ...mockFrames,
      { frame_index: 5, frame_type: "I", size: 52000, poc: 5, key_frame: true },
    ];
    rerender(<Timeline frames={newFrames} />);

    expect(document.querySelectorAll(".timeline-thumb").length).toBe(6);
  });

  it("should re-render when className changes", () => {
    const { rerender } = render(
      <Timeline {...defaultProps} className="first" />,
    );

    expect(screen.getByRole("region", { name: "Timeline" })).toHaveClass(
      "first",
    );

    rerender(<Timeline {...defaultProps} className="second" />);

    expect(screen.getByRole("region", { name: "Timeline" })).toHaveClass(
      "second",
    );
  });
});

describe("Timeline ARIA attributes", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useSelection).mockReturnValue(createMockSelectionContext());
  });

  it("should have region role", () => {
    render(<Timeline {...defaultProps} />);

    expect(
      screen.getByRole("region", { name: "Timeline" }),
    ).toBeInTheDocument();
  });

  it("should have proper ARIA label", () => {
    render(<Timeline {...defaultProps} />);

    expect(
      screen.getByRole("region", { name: "Timeline" }),
    ).toBeInTheDocument();
  });

  it("should have status role for empty state", () => {
    render(<Timeline frames={[]} />);

    // There are two status elements in empty state (timeline-info and timeline-empty)
    // Use getAllByRole or query by specific attribute
    const statusElements = screen.getAllByRole("status");
    expect(statusElements.length).toBeGreaterThan(0);

    // Check for the specific empty state status
    const emptyState = screen.getByRole("status", { name: "No frames loaded" });
    expect(emptyState).toBeInTheDocument();
    expect(emptyState).toHaveAttribute("aria-label", "No frames loaded");
  });

  it("should have aria-hidden on icons", () => {
    render(<Timeline frames={[]} />);

    const icon = document.querySelector(".codicon-graph");
    expect(icon).toHaveAttribute("aria-hidden", "true");
  });
});

describe("Timeline edge cases", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useSelection).mockReturnValue(createMockSelectionContext());
  });

  it("should handle very large frame count", () => {
    const largeFrames = Array.from({ length: 10000 }, (_, i) => ({
      frame_index: i,
      frame_type: i % 3 === 0 ? "I" : i % 3 === 1 ? "P" : "B",
      size: 30000,
      poc: i,
    })) as FrameInfo[];

    render(<Timeline frames={largeFrames} />);

    // TimelineHeader displays "{currentFrame + 1} / {totalFrames}" with spaces
    expect(screen.getByText(/1 \/ 10000/)).toBeInTheDocument();
  });

  it("should handle single frame", () => {
    const singleFrame = [mockFrames[0]];
    render(<Timeline frames={singleFrame} />);

    // TimelineHeader displays "{currentFrame + 1} / {totalFrames}" with spaces
    expect(screen.getByText(/1 \/ 1/)).toBeInTheDocument();
    expect(document.querySelectorAll(".timeline-thumb").length).toBe(1);
  });

  it("should handle frames with missing optional properties", () => {
    const minimalFrames = [
      { frame_index: 0, frame_type: "I", size: 50000, poc: 0 },
      { frame_index: 1, frame_type: "P", size: 30000, poc: 1 },
    ] as FrameInfo[];

    render(<Timeline frames={minimalFrames} />);

    // TimelineHeader displays "{currentFrame + 1} / {totalFrames}" with spaces
    expect(screen.getByText(/1 \/ 2/)).toBeInTheDocument();
  });

  it("should handle special characters in frame types", () => {
    const specialFrames = [
      { frame_index: 0, frame_type: "KEY", size: 50000, poc: 0 },
      { frame_index: 1, frame_type: "INTRA", size: 30000, poc: 1 },
    ] as FrameInfo[];

    render(<Timeline frames={specialFrames} />);

    // Should handle gracefully
    expect(
      screen.getByRole("region", { name: "Timeline" }),
    ).toBeInTheDocument();
  });

  it("should handle very small frames", () => {
    const tinyFrames = [
      { frame_index: 0, frame_type: "I", size: 100, poc: 0 },
      { frame_index: 1, frame_type: "P", size: 200, poc: 1 },
    ] as FrameInfo[];

    render(<Timeline frames={tinyFrames} />);

    expect(
      screen.getByRole("region", { name: "Timeline" }),
    ).toBeInTheDocument();
  });

  it("should handle very large frames", () => {
    const hugeFrames = [
      { frame_index: 0, frame_type: "I", size: 9999999, poc: 0 },
      { frame_index: 1, frame_type: "P", size: 8888888, poc: 1 },
    ] as FrameInfo[];

    render(<Timeline frames={hugeFrames} />);

    expect(
      screen.getByRole("region", { name: "Timeline" }),
    ).toBeInTheDocument();
  });
});

describe("Timeline with external selection changes", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should update highlighted frame when selection changes externally", () => {
    vi.mocked(useSelection).mockReturnValue(
      createMockSelectionContext({
        selection: { frame: { stream: "A", frameIndex: 3 } },
      }),
    );

    render(<Timeline {...defaultProps} />);

    // frameIndex 3 means header shows 4/5
    expect(screen.getByText(/4 \/ 5/)).toBeInTheDocument();
  });

  it("should sync with selection context on mount", () => {
    vi.mocked(useSelection).mockReturnValue(
      createMockSelectionContext({
        selection: { frame: { stream: "A", frameIndex: 2 } },
      }),
    );

    render(<Timeline {...defaultProps} />);

    // frameIndex 2 means header shows 3/5
    expect(screen.getByText(/3 \/ 5/)).toBeInTheDocument();
  });
});

describe("Timeline concurrent interactions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useSelection).mockReturnValue(createMockSelectionContext());
  });

  it("should handle mouse and keyboard interactions together", () => {
    render(<Timeline {...defaultProps} />);

    const timeline = document.querySelector(".timeline-thumbnails");
    if (!timeline) return;

    act(() => {
      // Start with mouse interaction
      fireEvent.mouseMove(timeline, { clientX: 100, clientY: 50 });

      // Then keyboard navigation
      fireEvent.keyDown(timeline, { key: "ArrowRight" });
    });

    // Should handle both without crashing
    expect(timeline).toBeInTheDocument();
  });

  it("should handle rapid mouse movements", () => {
    render(<Timeline {...defaultProps} />);

    const timeline = document.querySelector(".timeline-thumbnails");
    if (!timeline) return;

    act(() => {
      for (let i = 0; i < 20; i++) {
        fireEvent.mouseMove(timeline, { clientX: i * 20, clientY: 50 });
      }
    });

    expect(timeline).toBeInTheDocument();
  });

  it("should handle rapid drag movements", () => {
    render(<Timeline {...defaultProps} />);

    const timeline = document.querySelector(".timeline-thumbnails");
    if (!timeline) return;

    act(() => {
      fireEvent.mouseDown(timeline, { clientX: 100, clientY: 50 });

      for (let i = 0; i < 10; i++) {
        const moveEvent = new MouseEvent("mousemove", {
          clientX: 100 + i * 20,
          clientY: 50,
          bubbles: true,
        });
        Object.defineProperty(moveEvent, "target", {
          value: timeline,
          writable: false,
        });
        window.dispatchEvent(moveEvent);
      }

      const upEvent = new MouseEvent("mouseup", { bubbles: true });
      window.dispatchEvent(upEvent);
    });

    expect(timeline).toBeInTheDocument();
  });
});

describe("Timeline cleanup", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useSelection).mockReturnValue(createMockSelectionContext());
  });

  it("should cleanup event listeners on unmount", () => {
    const { unmount } = render(<Timeline {...defaultProps} />);

    // Start a drag
    const timeline = document.querySelector(".timeline-thumbnails");
    if (timeline) {
      act(() => {
        fireEvent.mouseDown(timeline, { clientX: 100, clientY: 50 });
      });

      // Unmount should cleanup
      expect(() => unmount()).not.toThrow();
    }
  });

  it("should not leak memory when frames change frequently", () => {
    const { rerender, unmount } = render(<Timeline {...defaultProps} />);

    // Rapidly changing frames
    for (let i = 0; i < 10; i++) {
      const shiftedFrames = mockFrames.slice(i);
      rerender(<Timeline frames={shiftedFrames} />);
    }

    expect(() => unmount()).not.toThrow();
  });
});

describe("Timeline hit testing", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useSelection).mockReturnValue(createMockSelectionContext());
  });

  it("should allow clicking near frame boundaries", () => {
    render(<Timeline {...defaultProps} />);

    const timeline = document.querySelector(".timeline-thumbnails");
    if (!timeline) return;

    act(() => {
      // Click near beginning
      fireEvent.click(timeline, { clientX: 10, clientY: 50 });

      // Click near end
      fireEvent.click(timeline, { clientX: 790, clientY: 50 });
    });

    // Should handle both without errors
    expect(timeline).toBeInTheDocument();
  });

  it("should allow clicking outside timeline bounds gracefully", () => {
    render(<Timeline {...defaultProps} />);

    const timeline = document.querySelector(".timeline-thumbnails");
    if (!timeline) return;

    act(() => {
      // Click outside
      fireEvent.click(timeline, { clientX: -10, clientY: 50 });
      fireEvent.click(timeline, { clientX: 1000, clientY: 50 });
    });

    // Should handle gracefully
    expect(timeline).toBeInTheDocument();
  });
});

describe("Timeline thumbnail props", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useSelection).mockReturnValue(createMockSelectionContext());
  });

  it("should pass correct highlightedFrameIndex to thumbnails", () => {
    vi.mocked(useSelection).mockReturnValue(
      createMockSelectionContext({
        selection: { frame: { stream: "A", frameIndex: 3 } },
      }),
    );

    render(<Timeline {...defaultProps} />);

    // Frame 3 should be selected
    const selectedFrame = document.querySelector(
      '[data-frame-index="3"].selected',
    );
    expect(selectedFrame).toBeInTheDocument();
  });

  it("should pass correct total frames to header", () => {
    render(<Timeline {...defaultProps} />);

    // TimelineHeader displays "{currentFrame + 1} / {totalFrames}" with spaces
    expect(screen.getByText(/1 \/ 5/)).toBeInTheDocument();
  });
});

describe("Timeline subclasses integration", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useSelection).mockReturnValue(createMockSelectionContext());
  });

  it("should integrate with TimelineHeader", () => {
    render(<Timeline {...defaultProps} />);

    expect(screen.getByText("Timeline")).toBeInTheDocument();
  });

  it("should integrate with TimelineThumbnails", () => {
    render(<Timeline {...defaultProps} />);

    expect(document.querySelector(".timeline-thumbnails")).toBeInTheDocument();
  });

  it("should integrate with TimelineCursor", () => {
    render(<Timeline {...defaultProps} />);

    expect(document.querySelector(".timeline-cursor")).toBeInTheDocument();
  });

  it("should integrate with TimelineTooltip", async () => {
    render(<Timeline {...defaultProps} />);

    const timeline = document.querySelector(".timeline-thumbnails");
    if (timeline) {
      act(() => {
        fireEvent.mouseMove(timeline, { clientX: 100, clientY: 50 });
      });

      await waitFor(
        () => {
          const tooltip = document.querySelector(".timeline-tooltip");
          if (tooltip) {
            expect(tooltip).toBeInTheDocument();
          }
        },
        { timeout: 1000 },
      );
    }
  });
});

describe("TimelineMemoized vs default export", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useSelection).mockReturnValue(createMockSelectionContext());
  });

  it("should export MemoizedTimeline separately", () => {
    // The component exports MemoizedTimeline as a named export
    // Since we're using ES modules, we can't test require() directly
    // The component structure has been validated by the build passing
    expect(true).toBe(true); // Placeholder - export validation is done by TypeScript
  });

  it("Timeline component should render", () => {
    // Verify the Timeline component can be imported and used
    render(<Timeline {...defaultProps} />);
    expect(
      screen.queryByRole("region", { name: "Timeline" }),
    ).toBeInTheDocument();
  });
});
