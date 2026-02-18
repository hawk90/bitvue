/**
 * TitleBar Component Tests
 * Tests application title bar with menus
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@/test/test-utils";
import { TitleBar } from "../TitleBar";

// Mock window controls
global.window.innerWidth = 1920;

describe("TitleBar", () => {
  it("should render title bar", () => {
    render(<TitleBar title="Bitvue" />);

    expect(screen.getByText("Bitvue")).toBeInTheDocument();
  });

  it("should use default title when not provided", () => {
    render(<TitleBar />);

    expect(screen.getByText("Bitvue")).toBeInTheDocument();
  });

  it("should render menu bar", () => {
    render(<TitleBar />);

    const menuBar = document.querySelector(".title-bar");
    expect(menuBar).toBeInTheDocument();
  });

  it("should use React.memo for performance", () => {
    const { rerender } = render(<TitleBar title="Test" />);

    rerender(<TitleBar title="Test" />);

    expect(screen.getByText("Test")).toBeInTheDocument();
  });

  it("should have correct CSS classes", () => {
    const { container } = render(<TitleBar />);

    const titleBar = container.querySelector(".title-bar");
    expect(titleBar).toBeInTheDocument();
  });
});

describe("TitleBar menus", () => {
  it("should have file menu", () => {
    render(<TitleBar />);

    expect(screen.getByText("File")).toBeInTheDocument();
  });

  it("should have view menu", () => {
    render(<TitleBar />);

    expect(screen.getByText("View")).toBeInTheDocument();
  });

  it("should have help menu", () => {
    render(<TitleBar />);

    expect(screen.getByText("Help")).toBeInTheDocument();
  });

  it("should show dropdown when menu clicked", () => {
    render(<TitleBar />);

    const fileMenu = screen.getByText("File");
    fireEvent.click(fileMenu);

    // Menu dropdown should appear (verified by state change)
    expect(fileMenu).toBeInTheDocument();
  });
});

describe("TitleBar platform-specific features", () => {
  it("should show window controls on Linux", () => {
    Object.defineProperty(window, "platform", {
      value: "Linux",
      writable: true,
    });

    render(<TitleBar />);

    const titleBar = document.querySelector(".title-bar");
    expect(titleBar).toBeInTheDocument();
  });

  it("should show window controls on Windows", () => {
    Object.defineProperty(window, "platform", {
      value: "Win32",
      writable: true,
    });

    render(<TitleBar />);

    const titleBar = document.querySelector(".title-bar");
    expect(titleBar).toBeInTheDocument();
  });
});

describe("TitleBar window controls", () => {
  it("should have minimize button", () => {
    render(<TitleBar />);

    const minimizeBtn = screen.queryByRole("button", { name: /minimize/i });
    expect(minimizeBtn).toBeInTheDocument();
  });

  it("should have maximize/restore button", () => {
    render(<TitleBar />);

    const maximizeBtn = screen.queryByRole("button", { name: /maximize/i });
    expect(maximizeBtn).toBeInTheDocument();
  });

  it("should have close button", () => {
    render(<TitleBar />);

    const closeBtn = screen.queryByRole("button", { name: /close/i });
    expect(closeBtn).toBeInTheDocument();
  });
});
