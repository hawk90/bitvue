/**
 * GoToFrameDialog Component Tests
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@/test/test-utils";
import { GoToFrameDialog } from "@/components/GoToFrameDialog";

const defaultProps = {
  isOpen: true,
  onClose: vi.fn(),
  currentIndex: 4, // 0-based → shows "5" in input
  totalFrames: 100,
  onGoTo: vi.fn(),
};

describe("GoToFrameDialog", () => {
  it("renders when open", () => {
    render(<GoToFrameDialog {...defaultProps} />);
    expect(screen.getByText("Go to Frame")).toBeInTheDocument();
  });

  it("does not render when closed", () => {
    const { container } = render(
      <GoToFrameDialog {...defaultProps} isOpen={false} />,
    );
    expect(container.firstChild).toBeNull();
  });

  it("shows 1-based frame number in input", () => {
    render(<GoToFrameDialog {...defaultProps} currentIndex={0} />);
    const input = screen.getByRole("spinbutton") as HTMLInputElement;
    expect(input.value).toBe("1");
  });

  it("converts to 0-based when calling onGoTo", () => {
    const onGoTo = vi.fn();
    render(<GoToFrameDialog {...defaultProps} onGoTo={onGoTo} />);

    const input = screen.getByRole("spinbutton");
    fireEvent.change(input, { target: { value: "10" } });
    fireEvent.click(screen.getByText("Go"));

    expect(onGoTo).toHaveBeenCalledWith(9); // 10 - 1 = 9 (0-based)
  });

  it("shows error for out-of-range frame number", () => {
    render(<GoToFrameDialog {...defaultProps} totalFrames={50} />);

    const input = screen.getByRole("spinbutton");
    fireEvent.change(input, { target: { value: "999" } });
    fireEvent.click(screen.getByText("Go"));

    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByRole("alert").textContent).toContain("50");
  });

  it("shows error for frame number 0", () => {
    render(<GoToFrameDialog {...defaultProps} />);

    const input = screen.getByRole("spinbutton");
    fireEvent.change(input, { target: { value: "0" } });
    fireEvent.click(screen.getByText("Go"));

    expect(screen.getByRole("alert")).toBeInTheDocument();
  });

  it("calls onClose when Cancel is clicked", () => {
    const onClose = vi.fn();
    render(<GoToFrameDialog {...defaultProps} onClose={onClose} />);
    fireEvent.click(screen.getByText("Cancel"));
    expect(onClose).toHaveBeenCalled();
  });

  it("calls onClose when Escape key is pressed", () => {
    const onClose = vi.fn();
    render(<GoToFrameDialog {...defaultProps} onClose={onClose} />);
    fireEvent.keyDown(screen.getByRole("spinbutton"), { key: "Escape" });
    expect(onClose).toHaveBeenCalled();
  });

  it("submits on Enter key", () => {
    const onGoTo = vi.fn();
    render(<GoToFrameDialog {...defaultProps} onGoTo={onGoTo} />);

    const input = screen.getByRole("spinbutton");
    fireEvent.change(input, { target: { value: "3" } });
    fireEvent.keyDown(input, { key: "Enter" });

    expect(onGoTo).toHaveBeenCalledWith(2); // 3 - 1 = 2
  });

  it("shows max frame number in label", () => {
    render(<GoToFrameDialog {...defaultProps} totalFrames={200} />);
    expect(screen.getByText(/1–200/)).toBeInTheDocument();
  });
});
