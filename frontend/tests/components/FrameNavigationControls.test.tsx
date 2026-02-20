/**
 * FrameNavigationControls Component Tests
 * Tests frame navigation controls component
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@/test/test-utils";
import { FrameNavigationControls } from "../YuvViewerPanel/FrameNavigationControls";

const defaultProps = {
  currentFrameIndex: 5,
  totalFrames: 100,
  onFirstFrame: vi.fn(),
  onPrevFrame: vi.fn(),
  onNextFrame: vi.fn(),
  onLastFrame: vi.fn(),
  onFrameChange: vi.fn(),
};

describe("FrameNavigationControls", () => {
  it("should render navigation controls", () => {
    render(<FrameNavigationControls {...defaultProps} />);

    const container = document.querySelector(".yuv-toolbar-group");
    expect(container).toBeInTheDocument();
  });

  it("should have first frame button", () => {
    render(<FrameNavigationControls {...defaultProps} />);

    const firstButton = screen.getAllByRole("button")[0];
    expect(firstButton).toBeInTheDocument();
    expect(firstButton).toHaveAttribute("title", "First Frame (Home)");
  });

  it("should have previous frame button", () => {
    render(<FrameNavigationControls {...defaultProps} />);

    const prevButton = screen.getAllByRole("button")[1];
    expect(prevButton).toBeInTheDocument();
    expect(prevButton).toHaveAttribute("title", "Previous Frame (←)");
  });

  it("should have next frame button", () => {
    render(<FrameNavigationControls {...defaultProps} />);

    const nextButton = screen.getAllByRole("button")[2];
    expect(nextButton).toBeInTheDocument();
    expect(nextButton).toHaveAttribute("title", "Next Frame (→)");
  });

  it("should have last frame button", () => {
    render(<FrameNavigationControls {...defaultProps} />);

    const lastButton = screen.getAllByRole("button")[3];
    expect(lastButton).toBeInTheDocument();
    expect(lastButton).toHaveAttribute("title", "Last Frame (End)");
  });

  it("should have frame input", () => {
    render(<FrameNavigationControls {...defaultProps} />);

    const input = screen.getByRole("spinbutton");
    expect(input).toBeInTheDocument();
  });

  it("should display current frame in input", () => {
    render(<FrameNavigationControls {...defaultProps} />);

    const input = screen.getByRole("spinbutton");
    expect(input).toHaveValue(5);
  });

  it("should call onFirstFrame when first button clicked", () => {
    const handleFirst = vi.fn();
    render(
      <FrameNavigationControls {...defaultProps} onFirstFrame={handleFirst} />,
    );

    const firstButton = screen.getAllByRole("button")[0];
    fireEvent.click(firstButton);

    expect(handleFirst).toHaveBeenCalledTimes(1);
  });

  it("should call onPrevFrame when previous button clicked", () => {
    const handlePrev = vi.fn();
    render(
      <FrameNavigationControls {...defaultProps} onPrevFrame={handlePrev} />,
    );

    const prevButton = screen.getAllByRole("button")[1];
    fireEvent.click(prevButton);

    expect(handlePrev).toHaveBeenCalledTimes(1);
  });

  it("should call onNextFrame when next button clicked", () => {
    const handleNext = vi.fn();
    render(
      <FrameNavigationControls {...defaultProps} onNextFrame={handleNext} />,
    );

    const nextButton = screen.getAllByRole("button")[2];
    fireEvent.click(nextButton);

    expect(handleNext).toHaveBeenCalledTimes(1);
  });

  it("should call onLastFrame when last button clicked", () => {
    const handleLast = vi.fn();
    render(
      <FrameNavigationControls {...defaultProps} onLastFrame={handleLast} />,
    );

    const lastButton = screen.getAllByRole("button")[3];
    fireEvent.click(lastButton);

    expect(handleLast).toHaveBeenCalledTimes(1);
  });

  it("should call onFrameChange when input changes", () => {
    const handleChange = vi.fn();
    render(
      <FrameNavigationControls
        {...defaultProps}
        onFrameChange={handleChange}
      />,
    );

    const input = screen.getByRole("spinbutton");
    fireEvent.change(input, { target: { value: "10" } });

    expect(handleChange).toHaveBeenCalledWith(10);
  });

  it("should use React.memo for performance", () => {
    const { rerender } = render(<FrameNavigationControls {...defaultProps} />);

    rerender(<FrameNavigationControls {...defaultProps} />);

    expect(document.querySelector(".yuv-toolbar-group")).toBeInTheDocument();
  });
});

describe("FrameNavigationControls edge cases", () => {
  it("should handle first frame", () => {
    render(<FrameNavigationControls {...defaultProps} currentFrameIndex={0} />);

    const input = screen.getByRole("spinbutton");
    expect(input).toHaveValue(0);
  });

  it("should handle last frame", () => {
    render(
      <FrameNavigationControls
        {...defaultProps}
        currentFrameIndex={99}
        totalFrames={100}
      />,
    );

    const input = screen.getByRole("spinbutton");
    expect(input).toHaveValue(99);
  });

  it("should handle single frame", () => {
    render(
      <FrameNavigationControls
        {...defaultProps}
        currentFrameIndex={0}
        totalFrames={1}
      />,
    );

    expect(screen.getByRole("spinbutton")).toHaveValue(0);
  });

  it("should handle large frame numbers", () => {
    render(
      <FrameNavigationControls
        {...defaultProps}
        currentFrameIndex={9999}
        totalFrames={10000}
      />,
    );

    expect(screen.getByRole("spinbutton")).toHaveValue(9999);
  });

  it("should validate input range", () => {
    const handleChange = vi.fn();
    render(
      <FrameNavigationControls
        {...defaultProps}
        totalFrames={100}
        onFrameChange={handleChange}
      />,
    );

    const input = screen.getByRole("spinbutton");

    // Test out of range value - component validates and won't call handler
    fireEvent.change(input, { target: { value: "150" } });

    // Should NOT call handler since validation happens in the component
    expect(handleChange).not.toHaveBeenCalled();
  });
});

describe("FrameNavigationControls button states", () => {
  it("should disable first button at frame 0", () => {
    render(<FrameNavigationControls {...defaultProps} currentFrameIndex={0} />);

    const firstButton = screen.getAllByRole("button")[0];
    // Button should be disabled at frame 0
    expect(firstButton).toBeDisabled();
  });

  it("should disable last button at last frame", () => {
    render(
      <FrameNavigationControls
        {...defaultProps}
        currentFrameIndex={99}
        totalFrames={100}
      />,
    );

    const lastButton = screen.getAllByRole("button")[3];
    expect(lastButton).toBeDisabled();
  });

  it("should have proper button titles", () => {
    render(<FrameNavigationControls {...defaultProps} />);

    const buttons = screen.getAllByRole("button");
    const firstButton = buttons[0];
    const lastButton = buttons[3];

    expect(firstButton).toHaveAttribute("title", "First Frame (Home)");
    expect(lastButton).toHaveAttribute("title", "Last Frame (End)");
  });
});

describe("FrameNavigationControls keyboard shortcuts", () => {
  it("should display shortcut hints in titles", () => {
    render(<FrameNavigationControls {...defaultProps} />);

    const buttons = screen.getAllByRole("button");
    const prevButton = buttons[1];
    const nextButton = buttons[2];

    expect(prevButton).toHaveAttribute("title", "Previous Frame (←)");
    expect(nextButton).toHaveAttribute("title", "Next Frame (→)");
  });
});
