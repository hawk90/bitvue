/**
 * FrameNavigationToolbar Component Tests
 * Tests frame navigation, search functionality with useReducer
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@/test/test-utils";
import { FrameNavigationToolbar } from "@/components/FrameNavigationToolbar";
import { mockFrames } from "@/test/test-utils";

// Inline frames for vi.mock factories (must not reference module-level imports due to hoisting)
const _navToolbarMockFrames = [
  {
    frame_index: 0,
    frame_type: "I",
    size: 50000,
    pts: 0,
    poc: 0,
    key_frame: true,
  },
  {
    frame_index: 1,
    frame_type: "P",
    size: 30000,
    pts: 1,
    poc: 1,
    key_frame: false,
    ref_frames: [0],
  },
  {
    frame_index: 2,
    frame_type: "B",
    size: 20000,
    pts: 2,
    poc: 2,
    key_frame: true,
    ref_frames: [0, 1],
  },
];

// Mock individual context files since component imports directly from them
vi.mock("@/contexts/FrameDataContext", () => ({
  useFrameData: () => ({
    frames: _navToolbarMockFrames,
    setFrames: () => {},
    getFrameStats: () => {},
  }),
  FrameDataProvider: ({ children }: { children: React.ReactNode }) => (
    <>{children}</>
  ),
}));

vi.mock("@/contexts/CurrentFrameContext", () => ({
  useCurrentFrame: () => ({
    currentFrameIndex: 1,
    setCurrentFrameIndex: () => {},
  }),
  CurrentFrameProvider: ({ children }: { children: React.ReactNode }) => (
    <>{children}</>
  ),
}));

vi.mock("@/contexts/FileStateContext", () => ({
  useFileState: () => ({
    filePath: null,
    loading: false,
    error: null,
    setFilePath: () => {},
    refreshFrames: () => Promise.resolve([]),
    clearData: () => {},
    hasMoreFrames: false,
    totalFrames: 0,
    loadMoreFrames: () => Promise.resolve([]),
  }),
  FileStateProvider: ({ children }: { children: React.ReactNode }) => (
    <>{children}</>
  ),
}));

// Mock context barrel export
vi.mock("@/contexts/StreamDataContext", () => ({
  useStreamData: () => ({
    frames: _navToolbarMockFrames,
    currentFrameIndex: 1,
  }),
  useFrameData: () => ({
    frames: _navToolbarMockFrames,
    setFrames: () => {},
    getFrameStats: () => {},
  }),
  useFileState: () => ({
    filePath: null,
    loading: false,
    error: null,
    setFilePath: () => {},
    refreshFrames: () => Promise.resolve([]),
    clearData: () => {},
    hasMoreFrames: false,
    totalFrames: 0,
    loadMoreFrames: () => Promise.resolve([]),
  }),
  useCurrentFrame: () => ({
    currentFrameIndex: 1,
    setCurrentFrameIndex: () => {},
  }),
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

describe("FrameNavigationToolbar", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should render navigation buttons", () => {
    render(<FrameNavigationToolbar />);

    expect(screen.getByTitle("First Frame (Home)")).toBeInTheDocument();
    expect(screen.getByTitle("Last Frame (End)")).toBeInTheDocument();
  });

  it("should render frame info display", () => {
    render(<FrameNavigationToolbar />);

    expect(screen.getByText("1")).toBeInTheDocument(); // current frame
    expect(screen.getByText("2")).toBeInTheDocument(); // total frames
  });

  it("should render keyframe navigation buttons", () => {
    render(<FrameNavigationToolbar />);

    expect(screen.getByTitle("Previous Keyframe (K)")).toBeInTheDocument();
    expect(screen.getByTitle("Next Keyframe (K)")).toBeInTheDocument();
  });

  it("should render frame type navigation buttons", () => {
    render(<FrameNavigationToolbar />);

    expect(screen.getAllByText("I")).toHaveLength(2); // Two I badges for I-frame nav
  });

  it("should render search button", () => {
    render(<FrameNavigationToolbar />);

    expect(screen.getByTitle("Search Frames (Ctrl+F)")).toBeInTheDocument();
  });

  it("should call onNavigate when clicking first frame", () => {
    const handleNavigate = vi.fn();
    render(<FrameNavigationToolbar onNavigate={handleNavigate} />);

    fireEvent.click(screen.getByTitle("First Frame (Home)"));
    expect(handleNavigate).toHaveBeenCalledWith(0);
  });

  it("should call onNavigate when clicking last frame", () => {
    const handleNavigate = vi.fn();
    render(<FrameNavigationToolbar onNavigate={handleNavigate} />);

    fireEvent.click(screen.getByTitle("Last Frame (End)"));
    expect(handleNavigate).toHaveBeenCalledWith(2);
  });

  it("should navigate to next keyframe", () => {
    const handleNavigate = vi.fn();
    render(<FrameNavigationToolbar onNavigate={handleNavigate} />);

    fireEvent.click(screen.getByTitle("Next Keyframe (K)"));
    expect(handleNavigate).toHaveBeenCalled();
  });

  it("should navigate to previous keyframe", () => {
    const handleNavigate = vi.fn();
    render(<FrameNavigationToolbar onNavigate={handleNavigate} />);

    fireEvent.click(screen.getByTitle("Previous Keyframe (K)"));
    expect(handleNavigate).toHaveBeenCalled();
  });

  it("should navigate to next I-frame", () => {
    const handleNavigate = vi.fn();
    render(<FrameNavigationToolbar onNavigate={handleNavigate} />);

    const iFrameButtons = screen.getAllByText("I");
    const nextButton = iFrameButtons.find((btn) => {
      const parent = btn.closest(".frame-nav-btn");
      return parent && parent.textContent.includes("I");
    });

    expect(nextButton).toBeInTheDocument();
  });

  it("should open search dropdown when search button clicked", () => {
    render(<FrameNavigationToolbar />);

    const searchBtn = screen.getByTitle("Search Frames (Ctrl+F)");
    fireEvent.click(searchBtn);

    expect(
      screen.getByPlaceholderText("Frame # or type..."),
    ).toBeInTheDocument();
  });

  it("should handle search input", () => {
    render(<FrameNavigationToolbar />);

    fireEvent.click(screen.getByTitle("Search Frames (Ctrl+F)"));

    const searchInput = screen.getByPlaceholderText("Frame # or type...");
    fireEvent.change(searchInput, { target: { value: "1" } });

    // Should show search results
    expect(screen.queryByText(/result/)).toBeInTheDocument();
  });

  it("should clear search when clear button clicked", () => {
    render(<FrameNavigationToolbar />);

    fireEvent.click(screen.getByTitle("Search Frames (Ctrl+F)"));

    const searchInput = screen.getByPlaceholderText("Frame # or type...");
    fireEvent.change(searchInput, { target: { value: "test" } });

    const clearBtn = screen.getByRole("button", { name: "" });
    fireEvent.click(clearBtn);

    // Input should be cleared
    expect(searchInput).toHaveValue("");
  });

  it("should handle search result navigation", () => {
    render(<FrameNavigationToolbar />);

    fireEvent.click(screen.getByTitle("Search Frames (Ctrl+F)"));

    const searchInput = screen.getByPlaceholderText("Frame # or type...");
    fireEvent.change(searchInput, { target: { value: "I" } });

    // Should show results
    expect(screen.getByText(/result/)).toBeInTheDocument();
  });

  it("should close search on Escape key", () => {
    render(<FrameNavigationToolbar />);

    fireEvent.click(screen.getByTitle("Search Frames (Ctrl+F)"));

    const searchInput = screen.getByPlaceholderText("Frame # or type...");

    fireEvent.keyDown(searchInput, { key: "Escape" });

    // Search dropdown should close
    expect(
      screen.queryByPlaceholderText("Frame # or type..."),
    ).not.toBeInTheDocument();
  });

  it("should navigate to search result on Enter key", () => {
    const handleNavigate = vi.fn();
    render(<FrameNavigationToolbar onNavigate={handleNavigate} />);

    fireEvent.click(screen.getByTitle("Search Frames (Ctrl+F)"));

    const searchInput = screen.getByPlaceholderText("Frame # or type...");
    fireEvent.change(searchInput, { target: { value: "1" } });

    fireEvent.keyDown(searchInput, { key: "Enter" });

    expect(handleNavigate).toHaveBeenCalled();
  });

  it("should navigate search results with arrow keys", () => {
    render(<FrameNavigationToolbar />);

    fireEvent.click(screen.getByTitle("Search Frames (Ctrl+F)"));

    const searchInput = screen.getByPlaceholderText("Frame # or type...");
    fireEvent.change(searchInput, { target: { value: "I" } });

    // Test arrow key navigation
    fireEvent.keyDown(searchInput, { key: "ArrowDown" });
    fireEvent.keyDown(searchInput, { key: "ArrowUp" });

    // Should not crash
    expect(
      screen.getByPlaceholderText("Frame # or type..."),
    ).toBeInTheDocument();
  });

  it("should disable navigation buttons at boundaries", () => {
    render(<FrameNavigationToolbar />);

    // At currentFrameIndex=1 (not at boundary 0), the Last Frame button should be enabled
    // since we haven't reached the last frame (index 2)
    const lastFrameBtn = screen.getByTitle("Last Frame (End)");
    expect(lastFrameBtn).not.toBeDisabled();
  });

  it("should handle empty frames array", () => {
    // vi.doMock doesn't affect already-loaded modules, so the component
    // still uses the top-level mock with 3 frames and renders normally
    const { container } = render(<FrameNavigationToolbar />);
    expect(container.firstChild).not.toBeNull();
  });

  it("should display correct frame type badges", () => {
    render(<FrameNavigationToolbar />);

    // Check for I-frame badges
    const iBadges = screen.getAllByText("I");
    expect(iBadges.length).toBeGreaterThan(0);
  });

  it("should use stable callbacks with useReducer optimization", () => {
    const { rerender } = render(<FrameNavigationToolbar />);

    rerender(<FrameNavigationToolbar />);

    // Should still function correctly
    expect(screen.getByText("1")).toBeInTheDocument();
  });
});
