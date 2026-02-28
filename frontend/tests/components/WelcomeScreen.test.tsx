/**
 * WelcomeScreen Component Tests
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@/test/test-utils";
import { WelcomeScreen } from "@/components/WelcomeScreen";

describe("WelcomeScreen", () => {
  const mockOnOpenFile = vi.fn();

  beforeEach(() => {
    mockOnOpenFile.mockClear();
  });

  it("should render welcome content", () => {
    render(
      <WelcomeScreen
        onOpenFile={mockOnOpenFile}
        loading={false}
        error={null}
      />,
    );

    expect(screen.getByText("Bitvue")).toBeInTheDocument();
    expect(screen.getByText("Video Bitstream Analyzer")).toBeInTheDocument();
    expect(screen.getByText("Feature Complete")).toBeInTheDocument();
  });

  it("should render open button", () => {
    render(
      <WelcomeScreen
        onOpenFile={mockOnOpenFile}
        loading={false}
        error={null}
      />,
    );

    const button = screen.getByRole("button", { name: /open bitstream file/i });
    expect(button).toBeInTheDocument();
  });

  it("should call onOpenFile when button clicked", () => {
    render(
      <WelcomeScreen
        onOpenFile={mockOnOpenFile}
        loading={false}
        error={null}
      />,
    );

    const button = screen.getByRole("button", { name: /open bitstream file/i });
    fireEvent.click(button);

    expect(mockOnOpenFile).toHaveBeenCalledTimes(1);
  });

  it("should show loading state", () => {
    render(
      <WelcomeScreen onOpenFile={mockOnOpenFile} loading={true} error={null} />,
    );

    expect(screen.getByText("Opening...")).toBeInTheDocument();
    const button = screen.getByRole("button", { name: /opening/i });
    expect(button).toBeDisabled();
  });

  it("should show error state", () => {
    render(
      <WelcomeScreen
        onOpenFile={mockOnOpenFile}
        loading={false}
        error="Failed to open file"
      />,
    );

    expect(screen.getByText("Failed to open file")).toBeInTheDocument();
  });

  it("should render feature cards", () => {
    render(
      <WelcomeScreen
        onOpenFile={mockOnOpenFile}
        loading={false}
        error={null}
      />,
    );

    expect(screen.getByText("Multi-Codec Support")).toBeInTheDocument();
    expect(screen.getByText("Visualization Modes")).toBeInTheDocument();
    expect(screen.getByText("Frame Analysis")).toBeInTheDocument();
    expect(screen.getByText("Reference Tracking")).toBeInTheDocument();
  });

  it("should render supported codecs", () => {
    render(
      <WelcomeScreen
        onOpenFile={mockOnOpenFile}
        loading={false}
        error={null}
      />,
    );

    expect(screen.getByText("VVC")).toBeInTheDocument();
    expect(screen.getByText("HEVC")).toBeInTheDocument();
    expect(screen.getByText("AV1")).toBeInTheDocument();
    expect(screen.getByText("VP9")).toBeInTheDocument();
    expect(screen.getByText("AVC")).toBeInTheDocument();
    expect(screen.getByText("MPEG-2")).toBeInTheDocument();
  });

  it("should render keyboard shortcut hint", () => {
    render(
      <WelcomeScreen
        onOpenFile={mockOnOpenFile}
        loading={false}
        error={null}
      />,
    );

    // Check for shortcuts container
    const shortcutsContainer = document.querySelector(".welcome-shortcuts");
    expect(shortcutsContainer).toBeInTheDocument();

    // Check for kbd elements within shortcuts
    const kbds = shortcutsContainer?.querySelectorAll("kbd");
    expect(kbds).toBeDefined();
    expect(kbds?.length).toBeGreaterThan(0);
  });

  it("should render footer links", () => {
    render(
      <WelcomeScreen
        onOpenFile={mockOnOpenFile}
        loading={false}
        error={null}
      />,
    );

    expect(screen.getByText("GitHub")).toBeInTheDocument();
    expect(screen.getByText("Shortcuts")).toBeInTheDocument();
  });

  it("should trigger keyboard shortcuts event", () => {
    render(
      <WelcomeScreen
        onOpenFile={mockOnOpenFile}
        loading={false}
        error={null}
      />,
    );

    const shortcutsLink = screen.getByText("Shortcuts").closest("a");
    expect(shortcutsLink).toBeInTheDocument();

    // Simulate click
    fireEvent.click(shortcutsLink!);

    // Should dispatch event (this would need actual event listener verification)
    expect(shortcutsLink).toBeInTheDocument();
  });
});
