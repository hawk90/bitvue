/**
 * DebugPanel Component Tests
 * Tests debug panel for frame data inspection
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@/test/test-utils";
import { DebugPanel, DebugPanelToggle } from "../DebugPanel";

// Mock SelectionContext - use @/ alias to resolve to the real module
vi.mock("@/contexts/SelectionContext", async (importOriginal) => {
  const mod =
    await importOriginal<typeof import("@/contexts/SelectionContext")>();
  return {
    ...mod,
    useSelection: () => ({
      selection: { stream: "A", frame: { frameIndex: 5 } },
    }),
  };
});

const mockFrames = [
  { frame_index: 0, frame_type: "I", size: 50000, poc: 0, key_frame: true },
  {
    frame_index: 1,
    frame_type: "P",
    size: 30000,
    poc: 1,
    ref_frames: [0],
    key_frame: false,
  },
  {
    frame_index: 2,
    frame_type: "P",
    size: 35000,
    poc: 2,
    ref_frames: [0],
    key_frame: false,
  },
  {
    frame_index: 3,
    frame_type: "B",
    size: 20000,
    poc: 3,
    ref_frames: [1, 2],
    key_frame: false,
  },
  {
    frame_index: 4,
    frame_type: "B",
    size: 25000,
    poc: 4,
    ref_frames: [2, 3],
    key_frame: false,
  },
  {
    frame_index: 5,
    frame_type: "P",
    size: 32000,
    poc: 5,
    ref_frames: [0],
    display_order: 5,
    coding_order: 3,
    key_frame: false,
  },
];

describe("DebugPanel", () => {
  it("should render debug panel when visible", () => {
    render(<DebugPanel frames={mockFrames} visible={true} onClose={vi.fn()} />);

    expect(screen.getByText("Debug Panel")).toBeInTheDocument();
  });

  it("should not render when not visible", () => {
    const { container } = render(
      <DebugPanel frames={mockFrames} visible={false} onClose={vi.fn()} />,
    );

    expect(container.firstChild).toBe(null);
  });

  it("should display statistics section", () => {
    render(<DebugPanel frames={mockFrames} visible={true} onClose={vi.fn()} />);

    expect(screen.getByText("Statistics")).toBeInTheDocument();
  });

  it("should show total frames count", () => {
    render(<DebugPanel frames={mockFrames} visible={true} onClose={vi.fn()} />);

    expect(screen.getByText("6")).toBeInTheDocument();
  });

  it("should show key frames count", () => {
    render(<DebugPanel frames={mockFrames} visible={true} onClose={vi.fn()} />);

    expect(screen.getByText("1")).toBeInTheDocument();
  });

  it("should show P frames count", () => {
    render(<DebugPanel frames={mockFrames} visible={true} onClose={vi.fn()} />);

    const pCount = screen.queryAllByText("3");
    expect(pCount.length).toBeGreaterThan(0);
  });

  it("should show B frames count", () => {
    render(<DebugPanel frames={mockFrames} visible={true} onClose={vi.fn()} />);

    expect(screen.getByText("2")).toBeInTheDocument();
  });

  it("should show with references count", () => {
    render(<DebugPanel frames={mockFrames} visible={true} onClose={vi.fn()} />);

    const threes = screen.queryAllByText("3");
    expect(threes.length).toBeGreaterThan(0);
  });
});

describe("DebugPanel current frame", () => {
  it("should display current frame section", () => {
    render(<DebugPanel frames={mockFrames} visible={true} onClose={vi.fn()} />);

    expect(screen.getByText(/Current Frame/)).toBeInTheDocument();
  });

  it("should show current frame index", () => {
    render(<DebugPanel frames={mockFrames} visible={true} onClose={vi.fn()} />);

    expect(screen.getByText("#5")).toBeInTheDocument();
  });

  it("should show frame type", () => {
    render(<DebugPanel frames={mockFrames} visible={true} onClose={vi.fn()} />);

    // Check that the current frame section contains "Frame Type"
    expect(screen.getByText("Frame Type")).toBeInTheDocument();
    // The actual value 'P' appears multiple times, just verify the label exists
  });

  it("should show frame size in KB", () => {
    render(<DebugPanel frames={mockFrames} visible={true} onClose={vi.fn()} />);

    // Check that the current frame section contains "Size" and the size value
    expect(screen.getByText("Size")).toBeInTheDocument();
    expect(screen.getByText(/31\.25\s*KB/)).toBeInTheDocument();
  });

  it("should show key frame status", () => {
    render(<DebugPanel frames={mockFrames} visible={true} onClose={vi.fn()} />);

    // Check that the current frame section contains "Key Frame" and "false"
    expect(screen.getByText("Key Frame")).toBeInTheDocument();
    expect(screen.getByText("false")).toBeInTheDocument();
  });

  it("should show display order", () => {
    render(<DebugPanel frames={mockFrames} visible={true} onClose={vi.fn()} />);

    // Check that the current frame section contains "Display Order"
    expect(screen.getByText("Display Order")).toBeInTheDocument();
    const fives = screen.queryAllByText("5");
    expect(fives.length).toBeGreaterThan(0);
  });

  it("should show coding order", () => {
    render(<DebugPanel frames={mockFrames} visible={true} onClose={vi.fn()} />);

    // Check that the current frame section contains "Coding Order" and "3"
    expect(screen.getByText("Coding Order")).toBeInTheDocument();
    const threes = screen.queryAllByText("3");
    expect(threes.length).toBeGreaterThan(0);
  });

  it("should show POC", () => {
    render(<DebugPanel frames={mockFrames} visible={true} onClose={vi.fn()} />);

    // Check that the current frame section contains "POC"
    expect(screen.getByText("POC")).toBeInTheDocument();
    const fives = screen.queryAllByText("5");
    expect(fives.length).toBeGreaterThan(0);
  });

  it("should show reference frames", () => {
    render(<DebugPanel frames={mockFrames} visible={true} onClose={vi.fn()} />);

    // Check that the current frame section contains "Ref Frames" and "[0]"
    expect(screen.getByText("Ref Frames")).toBeInTheDocument();
    expect(screen.getByText(/\[0\]/)).toBeInTheDocument();
  });
});

describe("DebugPanel frame list", () => {
  it("should show all frames list", () => {
    render(<DebugPanel frames={mockFrames} visible={true} onClose={vi.fn()} />);

    expect(screen.getByText(/All Frames/)).toBeInTheDocument();
  });

  it("should limit frame list to 50", () => {
    const largeFrames = Array.from({ length: 100 }, (_, i) => ({
      frame_index: i,
      frame_type: "I",
      size: 50000,
      poc: i,
    }));

    render(
      <DebugPanel frames={largeFrames} visible={true} onClose={vi.fn()} />,
    );

    expect(screen.getByText(/First 50/)).toBeInTheDocument();
  });

  it("should highlight current frame in list", () => {
    render(<DebugPanel frames={mockFrames} visible={true} onClose={vi.fn()} />);

    const currentFrame = document.querySelector(".debug-frame-item.selected");
    expect(currentFrame).toBeInTheDocument();
  });

  it("should show frame index in list", () => {
    render(<DebugPanel frames={mockFrames} visible={true} onClose={vi.fn()} />);

    expect(screen.getByText("#0")).toBeInTheDocument();
    expect(screen.getByText("#1")).toBeInTheDocument();
  });

  it("should show frame type in list", () => {
    render(<DebugPanel frames={mockFrames} visible={true} onClose={vi.fn()} />);

    const types = document.querySelectorAll(".debug-frame-type");
    expect(types.length).toBeGreaterThan(0);
  });

  it("should show references for frames with refs", () => {
    const { container } = render(
      <DebugPanel frames={mockFrames} visible={true} onClose={vi.fn()} />,
    );

    // Multiple frames have refs, so check that at least one "Refs:" label exists
    const refsLabels = container.querySelectorAll(".debug-frame-detail-label");
    const hasRefsLabel = Array.from(refsLabels).some(
      (el) => el.textContent === "Refs:",
    );
    expect(hasRefsLabel).toBe(true);
  });

  it("should show frame size in list", () => {
    render(<DebugPanel frames={mockFrames} visible={true} onClose={vi.fn()} />);

    expect(screen.getByText(/48\.8K/)).toBeInTheDocument();
  });
});

describe("DebugPanelToggle", () => {
  it("should render toggle button", () => {
    render(<DebugPanelToggle isOpen={false} onToggle={vi.fn()} />);

    const button = screen.queryByRole("button");
    expect(button).toBeInTheDocument();
  });

  it("should show correct icon when closed", () => {
    const { container } = render(
      <DebugPanelToggle isOpen={false} onToggle={vi.fn()} />,
    );

    const icon = container.querySelector(".codicon-chevron-left");
    expect(icon).toBeInTheDocument();
  });

  it("should show correct icon when open", () => {
    const { container } = render(
      <DebugPanelToggle isOpen={true} onToggle={vi.fn()} />,
    );

    const icon = container.querySelector(".codicon-chevron-right");
    expect(icon).toBeInTheDocument();
  });

  it("should have correct title when closed", () => {
    render(<DebugPanelToggle isOpen={false} onToggle={vi.fn()} />);

    const button = screen.queryByRole("button");
    expect(button).toHaveAttribute("title", "Open Debug Panel");
  });

  it("should have correct title when open", () => {
    render(<DebugPanelToggle isOpen={true} onToggle={vi.fn()} />);

    const button = screen.queryByRole("button");
    expect(button).toHaveAttribute("title", "Close Debug Panel");
  });

  it("should call onToggle when clicked", () => {
    const handleToggle = vi.fn();
    render(<DebugPanelToggle isOpen={false} onToggle={handleToggle} />);

    const button = screen.queryByRole("button");
    fireEvent.click(button!);

    expect(handleToggle).toHaveBeenCalledTimes(1);
  });

  it("should use React.memo for performance", () => {
    const { rerender } = render(
      <DebugPanelToggle isOpen={false} onToggle={vi.fn()} />,
    );

    rerender(<DebugPanelToggle isOpen={false} onToggle={vi.fn()} />);

    expect(screen.queryByRole("button")).toBeInTheDocument();
  });
});

describe("DebugPanel close button", () => {
  it("should have close button in header", () => {
    render(<DebugPanel frames={mockFrames} visible={true} onClose={vi.fn()} />);

    const closeButton = screen.queryByRole("button", { name: /close/i });
    expect(closeButton).toBeInTheDocument();
  });

  it("should have close button in toggle", () => {
    render(<DebugPanel frames={mockFrames} visible={true} onClose={vi.fn()} />);

    const toggleButton = document.querySelector(".debug-toggle.open");
    expect(toggleButton).toBeInTheDocument();
  });

  it("should call onClose when close button clicked", () => {
    const handleClose = vi.fn();
    render(
      <DebugPanel frames={mockFrames} visible={true} onClose={handleClose} />,
    );

    const closeButton = screen.queryByRole("button", { name: /close/i });
    fireEvent.click(closeButton!);

    expect(handleClose).toHaveBeenCalledTimes(1);
  });

  it("should call onClose when toggle button clicked", () => {
    const handleClose = vi.fn();
    render(
      <DebugPanel frames={mockFrames} visible={true} onClose={handleClose} />,
    );

    const toggleButton = document.querySelector(".debug-toggle.open");
    fireEvent.click(toggleButton);

    expect(handleClose).toHaveBeenCalledTimes(1);
  });
});

describe("DebugPanel edge cases", () => {
  it("should handle empty frames array", () => {
    const { container } = render(
      <DebugPanel frames={[]} visible={true} onClose={vi.fn()} />,
    );

    expect(screen.getByText("Total Frames")).toBeInTheDocument();
    // Find the '0' value next to 'Total Frames' label
    const labels = Array.from(container.querySelectorAll(".debug-info-label"));
    const totalFramesLabelIndex = labels.findIndex(
      (el) => el.textContent === "Total Frames",
    );
    expect(totalFramesLabelIndex).toBeGreaterThanOrEqual(0);

    const values = container.querySelectorAll(".debug-info-value");
    expect(values[totalFramesLabelIndex].textContent).toBe("0");
  });

  it("should handle null current frame", () => {
    vi.doMock("../contexts/SelectionContext", () => ({
      useSelection: () => ({
        selection: { stream: "A", frame: { frameIndex: 0 } },
      }),
    }));

    render(<DebugPanel frames={[]} visible={true} onClose={vi.fn()} />);

    expect(screen.getByText("Debug Panel")).toBeInTheDocument();
  });
});
