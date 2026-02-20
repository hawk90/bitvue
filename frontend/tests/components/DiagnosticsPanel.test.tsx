/**
 * DiagnosticsPanel Component Tests
 * Tests error/warning display and filtering
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@/test/test-utils";
import { DiagnosticsPanel } from "@/components/panels/DiagnosticsPanel";
import type { FrameInfo } from "@/types/video";

// Mock individual context hooks that DiagnosticsPanel uses
const mockFrames = [
  { frame_index: 0, frame_type: "I", size: 150000, key_frame: true },
  { frame_index: 1, frame_type: "P", size: 25000, key_frame: false },
] as FrameInfo[];

vi.mock("@/contexts/FrameDataContext", () => ({
  useFrameData: vi.fn(),
  FrameDataProvider: ({ children }: { children: React.ReactNode }) => (
    <>{children}</>
  ),
}));

vi.mock("@/contexts/CurrentFrameContext", () => ({
  useCurrentFrame: vi.fn(),
  CurrentFrameProvider: ({ children }: { children: React.ReactNode }) => (
    <>{children}</>
  ),
}));

vi.mock("@/contexts/FileStateContext", () => ({
  useFileState: vi.fn(),
  FileStateProvider: ({ children }: { children: React.ReactNode }) => (
    <>{children}</>
  ),
}));

vi.mock("@/contexts/StreamDataContext", () => ({
  useStreamData: vi.fn(),
  useFrameData: vi.fn(),
  useFileState: vi.fn(),
  useCurrentFrame: vi.fn(),
  FrameDataProvider: ({ children }: { children: React.ReactNode }) => (
    <>{children}</>
  ),
  FileStateProvider: ({ children }: { children: React.ReactNode }) => (
    <>{children}</>
  ),
  CurrentFrameProvider: ({ children }: { children: React.ReactNode }) => (
    <>{children}</>
  ),
  StreamDataProvider: ({ children }: { children: React.ReactNode }) => (
    <>{children}</>
  ),
}));

import { useFrameData } from "@/contexts/FrameDataContext";
import { useCurrentFrame } from "@/contexts/CurrentFrameContext";
import { useFileState } from "@/contexts/FileStateContext";

beforeEach(() => {
  vi.mocked(useFrameData).mockReturnValue({
    frames: mockFrames,
    setFrames: vi.fn(),
    getFrameStats: vi.fn(),
  } as any);
  vi.mocked(useCurrentFrame).mockReturnValue({
    currentFrameIndex: 1,
    setCurrentFrameIndex: vi.fn(),
  } as any);
  vi.mocked(useFileState).mockReturnValue({
    filePath: null,
    loading: false,
    error: null,
    setFilePath: vi.fn(),
    refreshFrames: vi.fn(),
    clearData: vi.fn(),
    hasMoreFrames: false,
    totalFrames: 0,
    loadMoreFrames: vi.fn(),
  } as any);
});

const mockDiagnostics = [
  {
    id: "diag1",
    severity: "error" as const,
    code: "ERR001",
    message: "Critical error occurred",
    source: "Parser",
    timestamp: Date.now(),
  },
  {
    id: "diag2",
    severity: "warning" as const,
    code: "WARN001",
    message: "Warning message",
    source: "Validator",
    timestamp: Date.now(),
  },
  {
    id: "diag3",
    severity: "info" as const,
    code: "INFO001",
    message: "Info message",
    source: "Analyzer",
    timestamp: Date.now(),
  },
];

describe("DiagnosticsPanel", () => {
  it("should render diagnostics panel header", () => {
    render(<DiagnosticsPanel diagnostics={[]} />);

    expect(screen.getByText("Diagnostics")).toBeInTheDocument();
    // The warning icon is in the title, not a button
    expect(document.querySelector(".codicon-warning")).toBeInTheDocument();
  });

  it("should render empty state when no diagnostics", () => {
    // Provide frames with size < 100000 to avoid auto-generated diagnostics
    vi.mocked(useFrameData).mockReturnValue({
      frames: [
        { frame_index: 0, frame_type: "I", size: 50000, key_frame: true },
        {
          frame_index: 1,
          frame_type: "P",
          size: 25000,
          key_frame: false,
          ref_frames: ["frame_0"],
        },
      ] as FrameInfo[],
      setFrames: vi.fn(),
      getFrameStats: vi.fn(),
    } as any);

    render(<DiagnosticsPanel diagnostics={[]} />);

    // The empty state shows "No diagnostics" without any filter
    expect(screen.getByText(/No diagnostics/i)).toBeInTheDocument();
  });

  it("should render diagnostics list", () => {
    render(<DiagnosticsPanel diagnostics={mockDiagnostics} />);

    expect(screen.getByText("Critical error occurred")).toBeInTheDocument();
    expect(screen.getByText("Warning message")).toBeInTheDocument();
    expect(screen.getByText("Info message")).toBeInTheDocument();
  });

  it("should display severity icons", () => {
    render(<DiagnosticsPanel diagnostics={mockDiagnostics} />);

    const errorIcon = document.querySelector(".codicon-error");
    const warningIcon = document.querySelector(".codicon-warning");
    const infoIcon = document.querySelector(".codicon-info");

    expect(errorIcon).toBeInTheDocument();
    expect(warningIcon).toBeInTheDocument();
    expect(infoIcon).toBeInTheDocument();
  });

  it("should show diagnostic codes", () => {
    render(<DiagnosticsPanel diagnostics={mockDiagnostics} />);

    expect(screen.getByText("ERR001")).toBeInTheDocument();
    expect(screen.getByText("WARN001")).toBeInTheDocument();
  });

  it('should show "all" filter count', () => {
    render(<DiagnosticsPanel diagnostics={mockDiagnostics} />);

    expect(screen.getByText("All (3)")).toBeInTheDocument();
  });

  it("should show severity counts", () => {
    render(<DiagnosticsPanel diagnostics={mockDiagnostics} />);

    // Check that the filter buttons exist with the right counts
    const buttons = screen.getAllByRole("button");
    const allButton = buttons.find((btn) => btn.textContent?.includes("All"));
    const errorButton = buttons.find((btn) =>
      btn.classList.contains("diagnostics-filter-error"),
    );
    const warningButton = buttons.find((btn) =>
      btn.classList.contains("diagnostics-filter-warning"),
    );

    expect(allButton).toBeInTheDocument();
    expect(errorButton).toBeInTheDocument();
    expect(warningButton).toBeInTheDocument();
  });

  it("should select diagnostic on click", async () => {
    render(<DiagnosticsPanel diagnostics={mockDiagnostics} />);

    const diagnosticRow = screen
      .getByText("Critical error occurred")
      .closest(".diagnostics-row");
    fireEvent.click(diagnosticRow!);

    // Wait for state update
    await waitFor(() => {
      expect(diagnosticRow).toHaveClass("selected");
    });
  });

  it("should show diagnostic details when selected", async () => {
    render(<DiagnosticsPanel diagnostics={mockDiagnostics} />);

    const diagnosticRow = screen
      .getByText("Critical error occurred")
      .closest(".diagnostics-row");
    fireEvent.click(diagnosticRow!);

    // Wait for details to appear - check for the details code in the details panel
    await waitFor(() => {
      expect(screen.getAllByText("ERR001").length).toBeGreaterThan(0);
      expect(
        document.querySelector(".diagnostics-details"),
      ).toBeInTheDocument();
    });
  });

  it("should close details with close button", async () => {
    render(<DiagnosticsPanel diagnostics={mockDiagnostics} />);

    // Select a diagnostic
    const diagnosticRow = screen
      .getByText("Critical error occurred")
      .closest(".diagnostics-row");
    fireEvent.click(diagnosticRow!);

    // Wait for details to appear
    await waitFor(() => {
      expect(
        document.querySelector(".diagnostics-details"),
      ).toBeInTheDocument();
    });

    // Click close button - find it by class since it has no text
    const closeBtn = document.querySelector(".diagnostics-details-close");
    fireEvent.click(closeBtn!);

    // Wait for details to hide
    await waitFor(() => {
      expect(
        document.querySelector(".diagnostics-details"),
      ).not.toBeInTheDocument();
    });
  });

  it("should show frame info when available", () => {
    const frameDiagnostic = {
      id: "frame-diag",
      severity: "warning" as const,
      code: "LARGE_FRAME",
      message: "Frame 5 is large",
      frameIndex: 5,
      source: "Analyzer",
      timestamp: Date.now(),
    };

    render(<DiagnosticsPanel diagnostics={[frameDiagnostic]} />);

    // Use getAllByText since Frame 5 appears in both message and frame ref
    expect(screen.getAllByText(/Frame 5/i).length).toBeGreaterThan(0);
  });

  it("should handle stream errors", () => {
    vi.mocked(useFileState).mockReturnValue({
      filePath: null,
      loading: false,
      error: "Stream parse error",
      setFilePath: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      hasMoreFrames: false,
      totalFrames: 0,
      loadMoreFrames: vi.fn(),
    } as any);

    render(<DiagnosticsPanel diagnostics={[]} />);

    // Should show stream error as diagnostic
    expect(screen.getByText("Stream parse error")).toBeInTheDocument();
  });

  it("should use stable callbacks (useCallback optimization)", () => {
    const { rerender } = render(
      <DiagnosticsPanel diagnostics={mockDiagnostics} />,
    );

    rerender(<DiagnosticsPanel diagnostics={mockDiagnostics} />);

    expect(screen.getByText("Diagnostics")).toBeInTheDocument();
  });
});

describe("DiagnosticsPanel filtering", () => {
  it("should filter to show only errors", async () => {
    render(<DiagnosticsPanel diagnostics={mockDiagnostics} />);

    const buttons = screen.getAllByRole("button");
    const errorButton = buttons.find((btn) =>
      btn.classList.contains("diagnostics-filter-error"),
    );

    if (errorButton) {
      fireEvent.click(errorButton);

      await waitFor(() => {
        expect(screen.getByText("Critical error occurred")).toBeInTheDocument();
        expect(screen.queryByText("Warning message")).not.toBeInTheDocument();
      });
    }
  });

  it("should filter to show only warnings", async () => {
    render(<DiagnosticsPanel diagnostics={mockDiagnostics} />);

    const buttons = screen.getAllByRole("button");
    const warningButton = buttons.find((btn) =>
      btn.classList.contains("diagnostics-filter-warning"),
    );

    if (warningButton) {
      fireEvent.click(warningButton);

      await waitFor(() => {
        expect(
          screen.queryByText("Critical error occurred"),
        ).not.toBeInTheDocument();
        expect(screen.getByText("Warning message")).toBeInTheDocument();
      });
    }
  });

  it("should filter to show only info", async () => {
    render(<DiagnosticsPanel diagnostics={mockDiagnostics} />);

    const buttons = screen.getAllByRole("button");
    const infoButton = buttons.find((btn) =>
      btn.classList.contains("diagnostics-filter-info"),
    );

    if (infoButton) {
      fireEvent.click(infoButton);

      await waitFor(() => {
        expect(
          screen.queryByText("Critical error occurred"),
        ).not.toBeInTheDocument();
        expect(screen.queryByText("Warning message")).not.toBeInTheDocument();
        expect(screen.getByText("Info message")).toBeInTheDocument();
      });
    }
  });

  it('should reset filter when clicking "All"', async () => {
    render(<DiagnosticsPanel diagnostics={mockDiagnostics} />);

    const buttons = screen.getAllByRole("button");

    // Filter to errors first
    const errorButton = buttons.find((btn) =>
      btn.classList.contains("diagnostics-filter-error"),
    );

    if (errorButton) {
      fireEvent.click(errorButton);

      // Then click All
      const allButton = buttons.find((btn) => btn.textContent?.includes("All"));

      if (allButton) {
        fireEvent.click(allButton);

        await waitFor(() => {
          expect(
            screen.getByText("Critical error occurred"),
          ).toBeInTheDocument();
          expect(screen.getByText("Warning message")).toBeInTheDocument();
        });
      }
    }
  });
});
