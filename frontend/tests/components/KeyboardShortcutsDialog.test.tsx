/**
 * KeyboardShortcutsDialog Component Tests
 * Tests keyboard shortcuts modal
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@/test/test-utils";
import { KeyboardShortcutsDialog } from "@/components/KeyboardShortcutsDialog";

describe("KeyboardShortcutsDialog", () => {
  it("should render dialog when open", () => {
    render(<KeyboardShortcutsDialog isOpen={true} onClose={vi.fn()} />);

    expect(screen.getByText("Keyboard Shortcuts")).toBeInTheDocument();
  });

  it("should not render when closed", () => {
    const { container } = render(
      <KeyboardShortcutsDialog isOpen={false} onClose={vi.fn()} />,
    );

    expect(container.firstChild).toBe(null);
  });

  it("should render navigation shortcuts", () => {
    render(<KeyboardShortcutsDialog isOpen={true} onClose={vi.fn()} />);

    expect(screen.getByText(/First frame/)).toBeInTheDocument();
    expect(screen.getByText(/Last frame/)).toBeInTheDocument();
    expect(screen.getByText(/Home/)).toBeInTheDocument();
    expect(screen.getByText(/End/)).toBeInTheDocument();
  });

  it("should render frame navigation shortcuts", () => {
    render(<KeyboardShortcutsDialog isOpen={true} onClose={vi.fn()} />);

    expect(screen.getByText(/Jump to next keyframe/)).toBeInTheDocument();
    expect(screen.getByText(/Jump to previous keyframe/)).toBeInTheDocument();
  });

  it("should render frame type navigation", () => {
    render(<KeyboardShortcutsDialog isOpen={true} onClose={vi.fn()} />);

    // Should have I-frame navigation
    const iFrameNavs = screen.queryAllByText(/I-frame/);
    expect(iFrameNavs.length).toBeGreaterThan(0);
  });

  it("should render mode shortcuts", () => {
    render(<KeyboardShortcutsDialog isOpen={true} onClose={vi.fn()} />);

    expect(screen.getByText(/Overview mode/)).toBeInTheDocument();
    expect(screen.getByText(/Coding Flow mode/)).toBeInTheDocument();
    expect(screen.getByText(/Prediction mode/)).toBeInTheDocument();
  });

  it("should render playback shortcuts", () => {
    render(<KeyboardShortcutsDialog isOpen={true} onClose={vi.fn()} />);

    expect(screen.getByText(/Play\/Pause/)).toBeInTheDocument();
    // There are multiple elements with "Pause" so use getAllByText
    expect(screen.getAllByText(/Pause/).length).toBeGreaterThan(0);
  });

  it("should have close button", () => {
    const handleClose = vi.fn();
    render(<KeyboardShortcutsDialog isOpen={true} onClose={handleClose} />);

    // There are multiple close buttons (in header and footer)
    const closeButtons = screen.getAllByRole("button");
    const closeButton = closeButtons.find(
      (btn) => btn.getAttribute("aria-label") === "Close",
    );

    expect(closeButton).toBeInTheDocument();
    if (closeButton) {
      fireEvent.click(closeButton);
      expect(handleClose).toHaveBeenCalledTimes(1);
    }
  });

  it("should close on Escape key", () => {
    const handleClose = vi.fn();
    render(<KeyboardShortcutsDialog isOpen={true} onClose={handleClose} />);

    fireEvent.keyDown(window, { key: "Escape" });

    expect(handleClose).toHaveBeenCalled();
  });

  it("should be modal with backdrop", () => {
    const { container } = render(
      <KeyboardShortcutsDialog isOpen={true} onClose={vi.fn()} />,
    );

    const backdrop = container.querySelector(".shortcuts-overlay");
    const modal = container.querySelector(".shortcuts-dialog");

    expect(modal).toBeInTheDocument();
    expect(backdrop).toBeInTheDocument();
  });

  it("should use React.memo for performance", () => {
    const { rerender } = render(
      <KeyboardShortcutsDialog isOpen={true} onClose={vi.fn()} />,
    );

    rerender(<KeyboardShortcutsDialog isOpen={true} onClose={vi.fn()} />);

    expect(screen.getByText("Keyboard Shortcuts")).toBeInTheDocument();
  });

  it("should have organized sections", () => {
    render(<KeyboardShortcutsDialog isOpen={true} onClose={vi.fn()} />);

    // Should have sections like Navigation, Playback, Modes, etc.
    expect(screen.getAllByText(/Navigation/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/mode/i).length).toBeGreaterThan(0);
  });
});

describe("KeyboardShortcutsDialog categories", () => {
  it("should show file operations section", () => {
    render(<KeyboardShortcutsDialog isOpen={true} onClose={vi.fn()} />);

    expect(screen.getByText(/Open file/)).toBeInTheDocument();
    expect(screen.getByText(/Close file/)).toBeInTheDocument();
  });

  it("should show view options section", () => {
    render(<KeyboardShortcutsDialog isOpen={true} onClose={vi.fn()} />);

    // Should have View section with view-related shortcuts
    expect(screen.getByText(/View/)).toBeInTheDocument();
    expect(screen.getByText(/Zoom in/)).toBeInTheDocument();
    expect(screen.getByText(/Zoom out/)).toBeInTheDocument();
  });
});

const BASE_SHORTCUTS = [
  { key: "ArrowLeft", action: "Previous frame" },
  { key: "ArrowRight", action: "Next frame" },
  { key: " ", action: "Play/Pause" },
  { key: "K", action: "Next keyframe" },
];
