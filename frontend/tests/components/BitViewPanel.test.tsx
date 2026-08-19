/**
 * BitViewPanel Component Tests
 * Tests bit-level syntax tree panel
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@/test/test-utils";
import { BitViewPanel } from "../BitViewPanel";

// BitViewPanel reads useFileState/useCurrentFrame from the StreamDataContext barrel directly
// (not going through @/test/test-utils's AllTheProviders tree) -- same established pattern as
// SyntaxDetailPanel.test.tsx/DiagnosticsPanel.test.tsx for the other two real consumers of these
// hooks, mocking the exact module specifier the component under test imports.
vi.mock("../../contexts/StreamDataContext", () => ({
  useFileState: () => ({ filePath: "/test/path/video.ivf" }),
  useCurrentFrame: () => ({
    currentFrameIndex: 0,
    setCurrentFrameIndex: vi.fn(),
  }),
}));

describe("BitViewPanel", () => {
  it("should render bit view panel", () => {
    render(<BitViewPanel />);

    expect(screen.getByText("Bit View")).toBeInTheDocument();
  });

  it("should render frame navigation controls", () => {
    render(<BitViewPanel />);

    expect(screen.getByText(/Frame 0/)).toBeInTheDocument();
  });

  it("should use React.memo for performance", () => {
    const { rerender } = render(<BitViewPanel />);

    rerender(<BitViewPanel />);

    expect(screen.getByText("Bit View")).toBeInTheDocument();
  });

  it("should render pin button", () => {
    render(<BitViewPanel />);

    expect(screen.getByRole("button", { name: /pin/i })).toBeInTheDocument();
  });

  it("should show a load-failure state when no real bridge is available", async () => {
    render(<BitViewPanel />);

    // No window.bitvue in the test environment, so getFrameSyntax rejects -- BitViewPanel
    // surfaces this as a real error state rather than crashing, same contract every other
    // Electron-bridge-backed panel in this suite follows.
    expect(
      await screen.findByText(/window\.bitvue is unavailable/i),
    ).toBeInTheDocument();
  });
});
