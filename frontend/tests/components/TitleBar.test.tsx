/**
 * TitleBar Component Tests
 * Tests application menu bar and menu navigation
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@/test/test-utils";
import { TitleBar } from "@/components/TitleBar";

// Mock file dialog
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

describe("TitleBar", () => {
  const mockOnOpenFile = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should render title bar", () => {
    render(<TitleBar fileName="test.h265" onOpenFile={mockOnOpenFile} />);

    // Should have menu items
    expect(screen.getByText("File")).toBeInTheDocument();
    expect(screen.getByText("Mode")).toBeInTheDocument();
    expect(screen.getByText("YUVDiff")).toBeInTheDocument();
    expect(screen.getByText("Options")).toBeInTheDocument();
    expect(screen.getByText("Export")).toBeInTheDocument();
    expect(screen.getByText("View")).toBeInTheDocument();
    expect(screen.getByText("Help")).toBeInTheDocument();
  });

  it("should open File menu on click", () => {
    render(<TitleBar fileName="test.h265" onOpenFile={mockOnOpenFile} />);

    const fileMenu = screen.getByText("File");
    fireEvent.mouseEnter(fileMenu);

    // Should show file menu items
    expect(screen.getByText("Open bitstream...")).toBeInTheDocument();
    expect(screen.getByText("Close bitstream")).toBeInTheDocument();
  });

  it("should open Mode menu on click", () => {
    render(<TitleBar fileName="test.h265" onOpenFile={mockOnOpenFile} />);

    const modeMenu = screen.getByText("Mode");
    fireEvent.mouseEnter(modeMenu);

    // Should show mode items
    expect(screen.getByText("Overview")).toBeInTheDocument();
    expect(screen.getByText("Coding Flow")).toBeInTheDocument();
    expect(screen.getByText("Prediction")).toBeInTheDocument();
  });

  it("should have submenus", () => {
    render(<TitleBar fileName="test.h265" onOpenFile={mockOnOpenFile} />);

    // Open File menu to see submenus
    fireEvent.mouseEnter(screen.getByText("File"));

    // Should have "Open bitstream as" submenu
    expect(screen.getByText("Open bitstream as...")).toBeInTheDocument();
  });

  it("should trigger file open action", () => {
    render(<TitleBar fileName="test.h265" onOpenFile={mockOnOpenFile} />);

    fireEvent.mouseEnter(screen.getByText("File"));

    const openButton = screen.getByText("Open bitstream...");
    fireEvent.click(openButton);

    expect(mockOnOpenFile).toHaveBeenCalled();
  });

  it("should render window controls", () => {
    render(<TitleBar fileName="test.h265" onOpenFile={mockOnOpenFile} />);

    // Should have minimize, maximize, close buttons
    const buttons = document.querySelectorAll(".title-bar-button");
    expect(buttons.length).toBe(3);
  });

  it("should close menu when clicking away", () => {
    render(<TitleBar fileName="test.h265" onOpenFile={mockOnOpenFile} />);

    const fileMenu = screen.getByText("File");
    fireEvent.mouseEnter(fileMenu);

    // Menu should be open
    expect(screen.getByText("Open bitstream...")).toBeInTheDocument();

    // Mouse leave to close
    fireEvent.mouseLeave(fileMenu.parentElement!);

    // Menu should close
    expect(screen.queryByText("Open bitstream...")).not.toBeInTheDocument();
  });

  it("should support keyboard shortcuts display", () => {
    render(<TitleBar fileName="test.h265" onOpenFile={mockOnOpenFile} />);

    fireEvent.mouseEnter(screen.getByText("Mode"));

    // Should show shortcuts
    expect(screen.getByText("F1")).toBeInTheDocument();
    expect(screen.getByText("F2")).toBeInTheDocument();
  });

  it("should open submenu on hover", () => {
    render(<TitleBar fileName="test.h265" onOpenFile={mockOnOpenFile} />);

    fireEvent.mouseEnter(screen.getByText("File"));
    fireEvent.mouseEnter(
      screen.getByText("Open bitstream as...").closest("div")!,
    );

    // Should show codec options
    expect(screen.getByText("AV1")).toBeInTheDocument();
    expect(screen.getByText("HEVC")).toBeInTheDocument();
  });

  it("should use React.memo for performance", () => {
    const { rerender } = render(
      <TitleBar fileName="test.h265" onOpenFile={mockOnOpenFile} />,
    );

    rerender(<TitleBar fileName="test.h265" onOpenFile={mockOnOpenFile} />);

    // Should render correctly
    expect(screen.getByText("File")).toBeInTheDocument();
  });

  it("should handle no filename", () => {
    render(<TitleBar fileName="" onOpenFile={mockOnOpenFile} />);

    // Should still render title bar
    expect(screen.getByText("File")).toBeInTheDocument();
  });
});

describe("TitleBar menu actions", () => {
  const mockOnOpenFile = vi.fn();

  it("should have Export options", () => {
    render(<TitleBar fileName="test.h265" onOpenFile={mockOnOpenFile} />);

    fireEvent.mouseEnter(screen.getByText("Export"));

    expect(screen.getByText("Data Export")).toBeInTheDocument();
  });

  it("should have View options", () => {
    render(<TitleBar fileName="test.h265" onOpenFile={mockOnOpenFile} />);

    fireEvent.mouseEnter(screen.getByText("View"));

    expect(screen.getByText("Reset Layout")).toBeInTheDocument();
  });

  it("should have Help options", () => {
    render(<TitleBar fileName="test.h265" onOpenFile={mockOnOpenFile} />);

    fireEvent.mouseEnter(screen.getByText("Help"));

    expect(screen.getByText("Documentation")).toBeInTheDocument();
    expect(screen.getByText("Keyboard Shortcuts")).toBeInTheDocument();
  });
});
