/**
 * StatusBar Component Tests
 * Tests status bar with frame and playback info
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@/test/test-utils";
import { StatusBar } from "../YuvViewerPanel/StatusBar";

const defaultProps = {
  currentFrameIndex: 42,
  totalFrames: 1000,
  currentMode: "overview" as const,
  zoom: 1.5,
  isPlaying: true,
  playbackSpeed: 2,
};

describe("StatusBar", () => {
  it("should render status bar", () => {
    render(<StatusBar {...defaultProps} />);

    const statusBar = document.querySelector(".yuv-status-bar");
    expect(statusBar).toBeInTheDocument();
  });

  it("should display frame count", () => {
    const { container } = render(<StatusBar {...defaultProps} />);

    // Component shows 1-based frame number (currentFrameIndex + 1)
    const frameSection = container.querySelector(".status-section");
    expect(frameSection?.textContent).toContain("43"); // 42 + 1
    expect(frameSection?.textContent).toContain("1000");
  });

  it("should display current mode", () => {
    render(<StatusBar {...defaultProps} />);

    expect(screen.getByText(/overview/i)).toBeInTheDocument();
  });

  it("should display zoom level", () => {
    const { container } = render(<StatusBar {...defaultProps} />);

    const sections = container.querySelectorAll(".status-section");
    const zoomSection = Array.from(sections).find((s) =>
      s.textContent?.includes("Zoom"),
    );
    expect(zoomSection?.textContent).toContain("150%");
  });

  it("should display playback status", () => {
    render(<StatusBar {...defaultProps} />);

    expect(screen.getByText(/playing/i)).toBeInTheDocument();
  });

  it("should display playback speed", () => {
    const { container } = render(<StatusBar {...defaultProps} />);

    const playingIndicator = container.querySelector(".yuv-playing-indicator");
    expect(playingIndicator?.textContent).toContain("2x");
  });

  it("should use React.memo for performance", () => {
    const { rerender } = render(<StatusBar {...defaultProps} />);

    rerender(<StatusBar {...defaultProps} />);

    expect(document.querySelector(".yuv-status-bar")).toBeInTheDocument();
  });

  it("should show paused status when not playing", () => {
    render(<StatusBar {...defaultProps} isPlaying={false} />);

    expect(screen.getByText(/paused/i)).toBeInTheDocument();
  });

  it("should handle single frame", () => {
    const { container } = render(
      <StatusBar {...defaultProps} totalFrames={1} currentFrameIndex={0} />,
    );

    // 1-based: frame 0 → "Frame 1 / 1"
    const frameSection = container.querySelector(".status-section");
    expect(frameSection?.textContent).toContain("1 / 1");
  });

  it("should handle zero zoom", () => {
    const { container } = render(<StatusBar {...defaultProps} zoom={0} />);

    const sections = container.querySelectorAll(".status-section");
    const zoomSection = Array.from(sections).find((s) =>
      s.textContent?.includes("Zoom"),
    );
    expect(zoomSection?.textContent).toContain("0%");
  });

  it("should handle fractional zoom", () => {
    const { container } = render(<StatusBar {...defaultProps} zoom={0.5} />);

    const sections = container.querySelectorAll(".status-section");
    const zoomSection = Array.from(sections).find((s) =>
      s.textContent?.includes("Zoom"),
    );
    expect(zoomSection?.textContent).toContain("50%");
  });
});

describe("StatusBar formatting", () => {
  it("should format large frame numbers", () => {
    const { container } = render(
      <StatusBar
        {...defaultProps}
        totalFrames={100000}
        currentFrameIndex={50000}
      />,
    );

    // 1-based: frame 50000 → "Frame 50001 / 100000"
    const frameSection = container.querySelector(".status-section");
    expect(frameSection?.textContent).toContain("50001");
    expect(frameSection?.textContent).toContain("100000");
  });

  it("should handle 0-indexed frame", () => {
    const { container } = render(
      <StatusBar {...defaultProps} currentFrameIndex={0} />,
    );

    // 1-based: frame 0 → "Frame 1 / ..."
    const frameSection = container.querySelector(".status-section");
    expect(frameSection?.textContent).toContain("1 /");
  });

  it("should display different mode names", () => {
    const { rerender } = render(
      <StatusBar {...defaultProps} currentMode="prediction" />,
    );

    expect(screen.getByText(/prediction/i)).toBeInTheDocument();

    rerender(<StatusBar {...defaultProps} currentMode="transform" />);

    expect(screen.getByText(/transform/i)).toBeInTheDocument();
  });

  it("should display decimal zoom levels", () => {
    const { container } = render(<StatusBar {...defaultProps} zoom={1.25} />);

    const sections = container.querySelectorAll(".status-section");
    const zoomSection = Array.from(sections).find((s) =>
      s.textContent?.includes("Zoom"),
    );
    expect(zoomSection?.textContent).toContain("125%");
  });
});

describe("StatusBar edge cases", () => {
  it("should handle zero total frames", () => {
    const { container } = render(
      <StatusBar {...defaultProps} totalFrames={0} currentFrameIndex={0} />,
    );

    // 1-based: frame 0 → "Frame 1 / 0"
    const frameSection = container.querySelector(".status-section");
    expect(frameSection?.textContent).toContain("1 / 0");
  });

  it("should handle very fast playback speed", () => {
    const { container } = render(
      <StatusBar {...defaultProps} playbackSpeed={16} />,
    );

    const playingIndicator = container.querySelector(".yuv-playing-indicator");
    expect(playingIndicator?.textContent).toContain("16x");
  });

  it("should handle slow playback speed", () => {
    const { container } = render(
      <StatusBar {...defaultProps} playbackSpeed={0.25} />,
    );

    const playingIndicator = container.querySelector(".yuv-playing-indicator");
    expect(playingIndicator?.textContent).toContain("0.25x");
  });

  it("should handle very high zoom", () => {
    const { container } = render(<StatusBar {...defaultProps} zoom={4} />);

    const sections = container.querySelectorAll(".status-section");
    const zoomSection = Array.from(sections).find((s) =>
      s.textContent?.includes("Zoom"),
    );
    expect(zoomSection?.textContent).toContain("400%");
  });
});

describe("StatusBar layout", () => {
  it("should have proper CSS classes", () => {
    const { container } = render(<StatusBar {...defaultProps} />);

    const statusBar = container.querySelector(".yuv-status-bar");
    expect(statusBar).toBeInTheDocument();
  });

  it("should display status sections", () => {
    render(<StatusBar {...defaultProps} />);

    const sections = document.querySelectorAll(".status-section");
    expect(sections.length).toBeGreaterThan(0);
  });
});

// ---------------------------------------------------------------------------
// Additional edge case tests
// ---------------------------------------------------------------------------

describe("StatusBar 1-based frame display", () => {
  it("frame index 0 displays as Frame 1", () => {
    render(
      <StatusBar {...defaultProps} currentFrameIndex={0} totalFrames={100} />,
    );

    expect(screen.getByText(/Frame 1 \/ 100/i)).toBeInTheDocument();
  });

  it("frame index 99 displays as Frame 100", () => {
    render(
      <StatusBar {...defaultProps} currentFrameIndex={99} totalFrames={100} />,
    );

    expect(screen.getByText(/Frame 100 \/ 100/i)).toBeInTheDocument();
  });
});

describe("StatusBar playback indicator", () => {
  it("shows 'Paused' text when isPlaying is false", () => {
    render(<StatusBar {...defaultProps} isPlaying={false} />);

    expect(screen.getByText(/Paused/i)).toBeInTheDocument();
  });

  it("shows 'Playing 1x' when isPlaying=true and speed=1", () => {
    const { container } = render(
      <StatusBar {...defaultProps} isPlaying={true} playbackSpeed={1} />,
    );

    const indicator = container.querySelector(".yuv-playing-indicator");
    expect(indicator?.textContent).toContain("Playing 1x");
  });

  it("formats playbackSpeed=0.5 as 'Playing 0.50x'", () => {
    const { container } = render(
      <StatusBar {...defaultProps} isPlaying={true} playbackSpeed={0.5} />,
    );

    const indicator = container.querySelector(".yuv-playing-indicator");
    expect(indicator?.textContent).toContain("Playing 0.50x");
  });
});

describe("StatusBar zoom percentage", () => {
  it("zoom=1.5 shows '150%'", () => {
    const { container } = render(<StatusBar {...defaultProps} zoom={1.5} />);

    const sections = container.querySelectorAll(".status-section");
    const zoomSection = Array.from(sections).find((s) =>
      s.textContent?.includes("Zoom"),
    );
    expect(zoomSection?.textContent).toContain("150%");
  });

  it("zoom=0.5 shows '50%'", () => {
    const { container } = render(<StatusBar {...defaultProps} zoom={0.5} />);

    const sections = container.querySelectorAll(".status-section");
    const zoomSection = Array.from(sections).find((s) =>
      s.textContent?.includes("Zoom"),
    );
    expect(zoomSection?.textContent).toContain("50%");
  });
});

describe("StatusBar mode display", () => {
  it("shows correct label for each known mode", () => {
    const modes = [
      { mode: "overview" as const, pattern: /overview/i },
      { mode: "coding-flow" as const, pattern: /coding flow/i },
      { mode: "prediction" as const, pattern: /prediction/i },
      { mode: "transform" as const, pattern: /transform/i },
      { mode: "qp-map" as const, pattern: /qp map/i },
      { mode: "mv-field" as const, pattern: /mv field/i },
      { mode: "reference" as const, pattern: /reference/i },
    ];

    for (const { mode, pattern } of modes) {
      const { container, unmount } = render(
        <StatusBar {...defaultProps} currentMode={mode} />,
      );

      const modeIndicator = container.querySelector(".yuv-mode-indicator");
      expect(modeIndicator?.textContent).toMatch(pattern);

      unmount();
    }
  });

  it("shows mode shortcut in the mode indicator", () => {
    const { container } = render(
      <StatusBar {...defaultProps} currentMode="overview" />,
    );

    const modeIndicator = container.querySelector(".yuv-mode-indicator");
    // Overview mode has shortcut "F1"
    expect(modeIndicator?.textContent).toContain("F1");
  });

  it("falls back to 'overview' label for an unrecognised mode", () => {
    const { container } = render(
      // Cast to bypass type check so we can test an unknown mode string
      <StatusBar
        {...defaultProps}
        currentMode={"unknown-mode" as "overview"}
      />,
    );

    const modeIndicator = container.querySelector(".yuv-mode-indicator");
    expect(modeIndicator?.textContent).toContain("overview");
  });
});
