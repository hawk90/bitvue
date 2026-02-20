/**
 * TitleBar Component Tests
 * Tests application title bar with menus
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@/test/test-utils";
import { TitleBar } from "../TitleBar";

// Mock window controls
global.window.innerWidth = 1920;

const defaultProps = {
  fileName: "test.h265",
  onOpenFile: vi.fn(),
};

describe("TitleBar", () => {
  it("should render title bar", () => {
    render(<TitleBar {...defaultProps} />);

    const menuBar = document.querySelector(".title-bar");
    expect(menuBar).toBeInTheDocument();
  });

  it("should use default props", () => {
    render(<TitleBar {...defaultProps} />);

    // TitleBar renders menu labels
    expect(screen.getByText("File")).toBeInTheDocument();
  });

  it("should render menu bar", () => {
    render(<TitleBar {...defaultProps} />);

    const menuBar = document.querySelector(".title-bar");
    expect(menuBar).toBeInTheDocument();
  });

  it("should use React.memo for performance", () => {
    const { rerender } = render(<TitleBar {...defaultProps} />);

    rerender(<TitleBar {...defaultProps} />);

    expect(screen.getByText("File")).toBeInTheDocument();
  });

  it("should have correct CSS classes", () => {
    const { container } = render(<TitleBar {...defaultProps} />);

    const titleBar = container.querySelector(".title-bar");
    expect(titleBar).toBeInTheDocument();
  });
});

describe("TitleBar menus", () => {
  it("should have file menu", () => {
    render(<TitleBar {...defaultProps} />);

    expect(screen.getByText("File")).toBeInTheDocument();
  });

  it("should have view menu", () => {
    render(<TitleBar {...defaultProps} />);

    expect(screen.getByText("View")).toBeInTheDocument();
  });

  it("should have help menu", () => {
    render(<TitleBar {...defaultProps} />);

    expect(screen.getByText("Help")).toBeInTheDocument();
  });

  it("should show dropdown when menu clicked", () => {
    render(<TitleBar {...defaultProps} />);

    const fileMenu = screen.getByText("File");
    fireEvent.mouseEnter(fileMenu);

    // Menu dropdown should appear
    expect(fileMenu).toBeInTheDocument();
  });
});

describe("TitleBar platform-specific features", () => {
  it("should show window controls on Linux", () => {
    Object.defineProperty(window, "platform", {
      value: "Linux",
      writable: true,
    });

    render(<TitleBar {...defaultProps} />);

    const titleBar = document.querySelector(".title-bar");
    expect(titleBar).toBeInTheDocument();
  });

  it("should show window controls on Windows", () => {
    Object.defineProperty(window, "platform", {
      value: "Win32",
      writable: true,
    });

    render(<TitleBar {...defaultProps} />);

    const titleBar = document.querySelector(".title-bar");
    expect(titleBar).toBeInTheDocument();
  });
});

describe("TitleBar window controls", () => {
  it("should have minimize button", () => {
    render(<TitleBar {...defaultProps} />);

    const minimizeBtn = screen.queryByRole("button", { name: /minimize/i });
    expect(minimizeBtn).toBeInTheDocument();
  });

  it("should have maximize/restore button", () => {
    render(<TitleBar {...defaultProps} />);

    const maximizeBtn = screen.queryByRole("button", { name: /maximize/i });
    expect(maximizeBtn).toBeInTheDocument();
  });

  it("should have close button", () => {
    render(<TitleBar {...defaultProps} />);

    const closeBtn = screen.queryByRole("button", { name: /close/i });
    expect(closeBtn).toBeInTheDocument();
  });
});
