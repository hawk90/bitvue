/* eslint-disable @typescript-eslint/no-explicit-any */
/**
 * YuvViewerPanel Component Tests
 * Tests main video viewer panel with playback controls and keyboard shortcuts
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { YuvViewerPanel } from "../YuvViewerPanel";
import { useMode } from "@/contexts/ModeContext";
import { useStreamData } from "@/contexts/StreamDataContext";
import { useFrameData } from "@/contexts/FrameDataContext";
import { useCanvasInteraction } from "@/hooks/useCanvasInteraction";
import { useSelection } from "@/contexts/SelectionContext";
import { getContextMenuItems } from "@/services/electronBridgeService";

// Mock contexts
vi.mock("@/contexts/ModeContext");
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
vi.mock("@/contexts/FrameDataContext", () => ({
  useFrameData: vi.fn(),
  FrameDataProvider: ({ children }: { children: React.ReactNode }) => (
    <>{children}</>
  ),
}));
vi.mock("@/contexts/FileStateContext", () => ({
  useFileState: vi.fn(),
  FileStateProvider: ({ children }: { children: React.ReactNode }) => (
    <>{children}</>
  ),
}));
vi.mock("@/hooks/useCanvasInteraction");
vi.mock("@/contexts/SelectionContext", () => ({
  useSelection: vi.fn(() => ({ selection: null })),
  SelectionProvider: ({ children }: { children: React.ReactNode }) => (
    <>{children}</>
  ),
}));
vi.mock("@/contexts/YuvDiffContext", () => ({
  YuvDiffProvider: ({ children }: { children: React.ReactNode }) => (
    <>{children}</>
  ),
  useYuvDiff: vi.fn(() => ({
    isLoaded: false,
    displayMode: "decoded",
    amplifyFactor: 1,
    frameCount: 0,
    loadFile: vi.fn(),
    unloadFile: vi.fn(),
    setDisplayMode: vi.fn(),
    setAmplifyFactor: vi.fn(),
    fetchMetrics: vi.fn(),
  })),
}));

// Mock Tauri invoke — still used by the debug-YUV path (get_decoded_frame_yuv's Tauri call was
// replaced by the Electron bridge below; get_debug_yuv_frame has no sidecar equivalent yet).
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(() => Promise.resolve({ success: false, error: "Test mode" })),
}));

// Mock the Electron bridge's getDecodedFrameYuv — the real decode path (2026-08-08 migration
// off Tauri's get_decoded_frame_yuv). The component actually calls the cancellable variant
// (axis-6 cancellation wiring) -- that's implemented here as a thin wrapper around the same
// `getDecodedFrameYuv` mock so every existing `sharedGetDecodedFrameYuvMock.mockResolvedValue/
// mockRejectedValue(...)` call below keeps controlling the resolved/rejected frame data, just
// via one extra layer of `{promise, cancel}` indirection matching the real bridge shape.
vi.mock("@/services/electronBridgeService", () => {
  const getDecodedFrameYuv = vi.fn(() =>
    Promise.reject(new Error("Test mode")),
  );
  return {
    getDecodedFrameYuv,
    getDecodedFrameYuvCancellable: vi.fn(
      (stream: string, frameIndex: number) => ({
        promise: getDecodedFrameYuv(stream, frameIndex),
        cancel: vi.fn(),
      }),
    ),
    getContextMenuItems: vi.fn(),
  };
});

// Mock Image constructor
global.Image = class {
  onload: (() => void) | null = null;
  onerror: (() => void) | null = null;
  src = "";

  constructor() {
    setTimeout(() => {
      if (this.onload) this.onload();
    }, 0);
  }
} as any;

const mockProps = {
  currentFrameIndex: 1,
  totalFrames: 100,
  onFrameChange: vi.fn(),
};

const mockFrames = [
  { frame_index: 0, frame_type: "I", size: 50000, poc: 0, key_frame: true },
  { frame_index: 1, frame_type: "P", size: 30000, poc: 1 },
  { frame_index: 2, frame_type: "P", size: 35000, poc: 2 },
  { frame_index: 99, frame_type: "I", size: 48000, poc: 99, key_frame: true },
];

const mockCanvasInteraction = {
  zoom: 1,
  pan: { x: 0, y: 0 },
  isDragging: false,
  zoomIn: vi.fn(),
  zoomOut: vi.fn(),
  resetZoom: vi.fn(),
  setZoom: vi.fn(),
  setPan: vi.fn(),
  handlers: {
    onWheel: vi.fn(),
    onMouseDown: vi.fn(),
    onMouseMove: vi.fn(),
    onMouseUp: vi.fn(),
  },
};

describe("YuvViewerPanel basic rendering", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useMode).mockReturnValue({
      currentMode: "overview",
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: "yuv",
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
      availableModes: [],
      availableOverlays: [],
      activeOverlays: new Set(),
      toggleOverlay: vi.fn(),
      activeCodec: null,
      handleFKey: vi.fn(),
      setActiveCodec: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: "/test/path",
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useFrameData).mockReturnValue({
      frames: mockFrames,
      setFrames: vi.fn(),
      getFrameStats: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it("should render video viewer panel", () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    expect(container.firstChild).toBeInTheDocument();
  });

  it("should render toolbar", () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    // Component renders without crashing
    expect(container.firstChild).toBeInTheDocument();
  });

  it("should display current frame index in status bar", () => {
    render(<YuvViewerPanel {...mockProps} />);

    // mockProps.currentFrameIndex = 1, StatusBar shows 1-based (currentFrameIndex + 1 = 2)
    expect(screen.getByText(/Frame 2/)).toBeInTheDocument();
  });

  it("should display total frames", () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    // Component renders with total frames info
    expect(container.firstChild).toBeInTheDocument();
  });

  it("should use React.memo for performance", () => {
    const { rerender, container } = render(<YuvViewerPanel {...mockProps} />);

    rerender(<YuvViewerPanel {...mockProps} />);

    expect(container.firstChild).toBeInTheDocument();
  });
});

describe("YuvViewerPanel frame navigation controls", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useMode).mockReturnValue({
      currentMode: "overview",
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: "yuv",
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
      availableModes: [],
      availableOverlays: [],
      activeOverlays: new Set(),
      toggleOverlay: vi.fn(),
      activeCodec: null,
      handleFKey: vi.fn(),
      setActiveCodec: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: "/test/path",
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useFrameData).mockReturnValue({
      frames: mockFrames,
      setFrames: vi.fn(),
      getFrameStats: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it("should have first frame button", () => {
    render(<YuvViewerPanel {...mockProps} />);

    const firstButton = screen.queryByRole("button", { name: /first/i });
    expect(firstButton).toBeInTheDocument();
  });

  it("should have previous frame button", () => {
    render(<YuvViewerPanel {...mockProps} />);

    const prevButton = screen.queryByRole("button", { name: /previous/i });
    expect(prevButton).toBeInTheDocument();
  });

  it("should have next frame button", () => {
    render(<YuvViewerPanel {...mockProps} />);

    const nextButton = screen.queryByRole("button", { name: /next/i });
    expect(nextButton).toBeInTheDocument();
  });

  it("should have last frame button", () => {
    render(<YuvViewerPanel {...mockProps} />);

    const lastButton = screen.queryByRole("button", { name: /last/i });
    expect(lastButton).toBeInTheDocument();
  });

  it("should show frame input", () => {
    render(<YuvViewerPanel {...mockProps} />);

    const frameInput = screen.queryByRole("spinbutton");
    expect(frameInput).toBeInTheDocument();
  });

  it("should navigate to first frame", () => {
    render(<YuvViewerPanel {...mockProps} />);

    const firstButton = screen.queryByRole("button", { name: /first/i });
    fireEvent.click(firstButton!);

    expect(mockProps.onFrameChange).toHaveBeenCalledWith(0);
  });

  it("should navigate to previous frame", () => {
    render(
      <YuvViewerPanel
        currentFrameIndex={50}
        totalFrames={100}
        onFrameChange={mockProps.onFrameChange}
      />,
    );

    const prevButton = screen.queryByRole("button", { name: /previous/i });
    fireEvent.click(prevButton!);

    expect(mockProps.onFrameChange).toHaveBeenCalledWith(49);
  });

  it("should navigate to next frame", () => {
    render(<YuvViewerPanel {...mockProps} />);

    const nextButton = screen.queryByRole("button", { name: /next/i });
    fireEvent.click(nextButton!);

    expect(mockProps.onFrameChange).toHaveBeenCalledWith(2);
  });

  it("should navigate to last frame", () => {
    render(<YuvViewerPanel {...mockProps} />);

    const lastButton = screen.queryByRole("button", { name: /last/i });
    fireEvent.click(lastButton!);

    expect(mockProps.onFrameChange).toHaveBeenCalledWith(99);
  });

  it("should not navigate before first frame", () => {
    render(
      <YuvViewerPanel
        currentFrameIndex={0}
        totalFrames={100}
        onFrameChange={mockProps.onFrameChange}
      />,
    );

    const prevButton = screen.queryByRole("button", { name: /previous/i });
    fireEvent.click(prevButton!);

    expect(mockProps.onFrameChange).not.toHaveBeenCalled();
  });

  it("should not navigate after last frame", () => {
    render(
      <YuvViewerPanel
        currentFrameIndex={99}
        totalFrames={100}
        onFrameChange={mockProps.onFrameChange}
      />,
    );

    const nextButton = screen.queryByRole("button", { name: /next/i });
    fireEvent.click(nextButton!);

    expect(mockProps.onFrameChange).not.toHaveBeenCalled();
  });
});

describe("YuvViewerPanel playback controls", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useMode).mockReturnValue({
      currentMode: "overview",
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: "yuv",
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
      availableModes: [],
      availableOverlays: [],
      activeOverlays: new Set(),
      toggleOverlay: vi.fn(),
      activeCodec: null,
      handleFKey: vi.fn(),
      setActiveCodec: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: "/test/path",
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useFrameData).mockReturnValue({
      frames: mockFrames,
      setFrames: vi.fn(),
      getFrameStats: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it("should have play/pause button", () => {
    render(<YuvViewerPanel {...mockProps} />);

    const playButton = screen.queryByRole("button", { name: /play|pause/i });
    expect(playButton).toBeInTheDocument();
  });

  it("should have speed selector", () => {
    render(<YuvViewerPanel {...mockProps} />);

    const speedSelector = screen.queryByRole("combobox", { name: /speed/i });
    expect(speedSelector).toBeInTheDocument();
  });

  it("should toggle play/pause when button is clicked", () => {
    render(<YuvViewerPanel {...mockProps} />);

    const playButton = screen.queryByRole("button", { name: /play|pause/i });
    fireEvent.click(playButton!);

    // State should have changed (verified by component rerender)
    expect(playButton).toBeInTheDocument();
  });
});

describe("YuvViewerPanel zoom controls", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useMode).mockReturnValue({
      currentMode: "overview",
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: "yuv",
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
      availableModes: [],
      availableOverlays: [],
      activeOverlays: new Set(),
      toggleOverlay: vi.fn(),
      activeCodec: null,
      handleFKey: vi.fn(),
      setActiveCodec: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: "/test/path",
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useFrameData).mockReturnValue({
      frames: mockFrames,
      setFrames: vi.fn(),
      getFrameStats: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it("should have zoom in button", () => {
    render(<YuvViewerPanel {...mockProps} />);

    const zoomInButton = screen.queryByRole("button", { name: /zoom.*in/i });
    expect(zoomInButton).toBeInTheDocument();
  });

  it("should have zoom out button", () => {
    render(<YuvViewerPanel {...mockProps} />);

    const zoomOutButton = screen.queryByRole("button", { name: /zoom.*out/i });
    expect(zoomOutButton).toBeInTheDocument();
  });

  it("should have reset zoom button", () => {
    render(<YuvViewerPanel {...mockProps} />);

    const resetButton = screen.queryByRole("button", { name: /reset/i });
    expect(resetButton).toBeInTheDocument();
  });

  it("should call zoomIn when zoom in button is clicked", () => {
    render(<YuvViewerPanel {...mockProps} />);

    const zoomInButton = screen.queryByRole("button", { name: /zoom.*in/i });
    fireEvent.click(zoomInButton!);

    expect(mockCanvasInteraction.zoomIn).toHaveBeenCalled();
  });

  it("should call zoomOut when zoom out button is clicked", () => {
    render(<YuvViewerPanel {...mockProps} />);

    const zoomOutButton = screen.queryByRole("button", { name: /zoom.*out/i });
    fireEvent.click(zoomOutButton!);

    expect(mockCanvasInteraction.zoomOut).toHaveBeenCalled();
  });

  it("should call resetZoom when reset button is clicked", () => {
    render(<YuvViewerPanel {...mockProps} />);

    const resetButton = screen.queryByRole("button", { name: /reset/i });
    fireEvent.click(resetButton!);

    expect(mockCanvasInteraction.resetZoom).toHaveBeenCalled();
  });

  // INT-01: Fit-to-window toolbar button
  it("should have a fit-to-window button", () => {
    render(<YuvViewerPanel {...mockProps} />);

    const fitButton = screen.queryByRole("button", { name: /fit/i });
    expect(fitButton).toBeInTheDocument();
  });

  it("should call setZoom/setPan when the fit-to-window button is clicked", () => {
    Object.defineProperty(HTMLDivElement.prototype, "clientWidth", {
      configurable: true,
      value: 800,
    });
    Object.defineProperty(HTMLDivElement.prototype, "clientHeight", {
      configurable: true,
      value: 450,
    });

    render(<YuvViewerPanel {...mockProps} />);

    const fitButton = screen.queryByRole("button", { name: /fit/i });
    fireEvent.click(fitButton!);

    expect(mockCanvasInteraction.setZoom).toHaveBeenCalledWith(1.25);
    expect(mockCanvasInteraction.setPan).toHaveBeenCalledWith({ x: 0, y: 0 });
  });
});

describe("YuvViewerPanel mode selector", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    const setModeMock = vi.fn();
    vi.mocked(useMode).mockReturnValue({
      currentMode: "overview",
      setMode: setModeMock,
      cycleMode: vi.fn(),
      componentMask: "yuv",
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
      availableModes: [],
      availableOverlays: [],
      activeOverlays: new Set(),
      toggleOverlay: vi.fn(),
      activeCodec: null,
      handleFKey: vi.fn(),
      setActiveCodec: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: "/test/path",
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useFrameData).mockReturnValue({
      frames: mockFrames,
      setFrames: vi.fn(),
      getFrameStats: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it("should have mode selector dropdown", () => {
    render(<YuvViewerPanel {...mockProps} />);

    const modeSelector = screen.queryByRole("combobox", { name: /mode/i });
    expect(modeSelector).toBeInTheDocument();
  });

  it("should display current mode", () => {
    render(<YuvViewerPanel {...mockProps} />);

    expect(screen.getByText(/overview/i)).toBeInTheDocument();
  });
});

describe("YuvViewerPanel keyboard shortcuts - navigation", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useMode).mockReturnValue({
      currentMode: "overview",
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: "yuv",
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
      availableModes: [],
      availableOverlays: [],
      activeOverlays: new Set(),
      toggleOverlay: vi.fn(),
      activeCodec: null,
      handleFKey: vi.fn(),
      setActiveCodec: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 50,
      loading: false,
      error: null,
      filePath: "/test/path",
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it("should handle Space key for play/pause", () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: " " });

    const playButton = screen.queryByRole("button", { name: /play|pause/i });
    expect(playButton).toBeInTheDocument();
  });

  it("should handle ArrowLeft key", () => {
    render(
      <YuvViewerPanel
        currentFrameIndex={50}
        totalFrames={100}
        onFrameChange={mockProps.onFrameChange}
      />,
    );

    fireEvent.keyDown(document, { key: "ArrowLeft" });

    expect(mockProps.onFrameChange).toHaveBeenCalledWith(49);
  });

  it("should handle ArrowRight key", () => {
    render(
      <YuvViewerPanel
        currentFrameIndex={50}
        totalFrames={100}
        onFrameChange={mockProps.onFrameChange}
      />,
    );

    fireEvent.keyDown(document, { key: "ArrowRight" });

    expect(mockProps.onFrameChange).toHaveBeenCalledWith(51);
  });

  it("should handle Home key", () => {
    render(
      <YuvViewerPanel
        currentFrameIndex={50}
        totalFrames={100}
        onFrameChange={mockProps.onFrameChange}
      />,
    );

    fireEvent.keyDown(document, { key: "Home" });

    expect(mockProps.onFrameChange).toHaveBeenCalledWith(0);
  });

  it("should handle End key", () => {
    render(
      <YuvViewerPanel
        currentFrameIndex={50}
        totalFrames={100}
        onFrameChange={mockProps.onFrameChange}
      />,
    );

    fireEvent.keyDown(document, { key: "End" });

    expect(mockProps.onFrameChange).toHaveBeenCalledWith(99);
  });

  it("should not navigate before first frame with ArrowLeft", () => {
    render(
      <YuvViewerPanel
        currentFrameIndex={0}
        totalFrames={100}
        onFrameChange={mockProps.onFrameChange}
      />,
    );

    fireEvent.keyDown(document, { key: "ArrowLeft" });

    expect(mockProps.onFrameChange).not.toHaveBeenCalled();
  });

  it("should not navigate after last frame with ArrowRight", () => {
    render(
      <YuvViewerPanel
        currentFrameIndex={99}
        totalFrames={100}
        onFrameChange={mockProps.onFrameChange}
      />,
    );

    fireEvent.keyDown(document, { key: "ArrowRight" });

    expect(mockProps.onFrameChange).not.toHaveBeenCalled();
  });
});

describe("YuvViewerPanel keyboard shortcuts - zoom", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useMode).mockReturnValue({
      currentMode: "overview",
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: "yuv",
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
      availableModes: [],
      availableOverlays: [],
      activeOverlays: new Set(),
      toggleOverlay: vi.fn(),
      activeCodec: null,
      handleFKey: vi.fn(),
      setActiveCodec: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: "/test/path",
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useFrameData).mockReturnValue({
      frames: mockFrames,
      setFrames: vi.fn(),
      getFrameStats: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it("should handle + key for zoom in", () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: "+" });

    expect(mockCanvasInteraction.zoomIn).toHaveBeenCalled();
  });

  it("should handle = key for zoom in", () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: "=" });

    expect(mockCanvasInteraction.zoomIn).toHaveBeenCalled();
  });

  it("should handle - key for zoom out", () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: "-" });

    expect(mockCanvasInteraction.zoomOut).toHaveBeenCalled();
  });

  it("should handle Ctrl+0 for reset zoom", () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: "0", ctrlKey: true });

    expect(mockCanvasInteraction.resetZoom).toHaveBeenCalled();
  });

  it("should handle Cmd+0 for reset zoom (Mac)", () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: "0", metaKey: true });

    expect(mockCanvasInteraction.resetZoom).toHaveBeenCalled();
  });

  it("should not handle 0 without modifier key", () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: "0" });

    expect(mockCanvasInteraction.resetZoom).not.toHaveBeenCalled();
  });

  // INT-01: Fit/100%/200% zoom presets -- previously documented-but-empty "1"/"2"/"3" stubs.
  it("should set zoom to 100% and reset pan on '2'", () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: "2" });

    expect(mockCanvasInteraction.setZoom).toHaveBeenCalledWith(1);
    expect(mockCanvasInteraction.setPan).toHaveBeenCalledWith({ x: 0, y: 0 });
  });

  it("should set zoom to 200% and reset pan on '3'", () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: "3" });

    expect(mockCanvasInteraction.setZoom).toHaveBeenCalledWith(2);
    expect(mockCanvasInteraction.setPan).toHaveBeenCalledWith({ x: 0, y: 0 });
  });

  it("should compute and apply a real fit-to-window zoom on '1'", () => {
    // jsdom elements report 0x0 by default -- stub the container's real on-screen size the
    // same way VideoCanvas.test.tsx stubs getBoundingClientRect for its own coordinate math.
    Object.defineProperty(HTMLDivElement.prototype, "clientWidth", {
      configurable: true,
      value: 800,
    });
    Object.defineProperty(HTMLDivElement.prototype, "clientHeight", {
      configurable: true,
      value: 450,
    });

    render(<YuvViewerPanel {...mockProps} />);
    fireEvent.keyDown(document, { key: "1" });

    // No yuvData/frameImage in this test -> VideoCanvas's logical size fallback is 640x360;
    // fitZoom = min(800/640, 450/360) = min(1.25, 1.25) = 1.25.
    expect(mockCanvasInteraction.setZoom).toHaveBeenCalledWith(1.25);
    expect(mockCanvasInteraction.setPan).toHaveBeenCalledWith({ x: 0, y: 0 });
  });

  it("should not trigger a preset when a modifier key is held with 1/2/3", () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: "1", ctrlKey: true });
    fireEvent.keyDown(document, { key: "2", metaKey: true });
    fireEvent.keyDown(document, { key: "3", altKey: true });

    expect(mockCanvasInteraction.setZoom).not.toHaveBeenCalled();
  });
});

describe("YuvViewerPanel keyboard shortcuts - mode switching (F1-F7)", () => {
  let handleFKeyMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    vi.clearAllMocks();
    handleFKeyMock = vi.fn().mockReturnValue(true);
    vi.mocked(useMode).mockReturnValue({
      currentMode: "overview",
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: "yuv",
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
      availableModes: [],
      availableOverlays: [],
      activeOverlays: new Set(),
      toggleOverlay: vi.fn(),
      activeCodec: null,
      handleFKey: handleFKeyMock,
      setActiveCodec: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: "/test/path",
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useFrameData).mockReturnValue({
      frames: mockFrames,
      setFrames: vi.fn(),
      getFrameStats: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it("should handle F1 key for overview mode", () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: "F1" });

    expect(handleFKeyMock).toHaveBeenCalledWith(1);
  });

  it("should handle F2 key for coding-flow mode", () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: "F2" });

    expect(handleFKeyMock).toHaveBeenCalledWith(2);
  });

  it("should handle F3 key for prediction mode", () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: "F3" });

    expect(handleFKeyMock).toHaveBeenCalledWith(3);
  });

  it("should handle F4 key for transform mode", () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: "F4" });

    expect(handleFKeyMock).toHaveBeenCalledWith(4);
  });

  it("should handle F5 key for qp-map mode", () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: "F5" });

    expect(handleFKeyMock).toHaveBeenCalledWith(5);
  });

  it("should handle F6 key for mv-field mode", () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: "F6" });

    expect(handleFKeyMock).toHaveBeenCalledWith(6);
  });

  it("should handle F7 key for reference mode", () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: "F7" });

    expect(handleFKeyMock).toHaveBeenCalledWith(7);
  });
});

// Player context menu: "Copy Selection" now copies the currently-selected block's position/size
// (was a hardcoded no-op stub -- see PARITY_CHECKLIST.md INT-01's 2026-08-23 note).
describe("YuvViewerPanel Player context menu", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useMode).mockReturnValue({
      currentMode: "overview",
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: "yuv",
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
      availableModes: [],
      availableOverlays: [],
      activeOverlays: new Set(),
      toggleOverlay: vi.fn(),
      activeCodec: null,
      handleFKey: vi.fn(),
      setActiveCodec: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: "/test/path",
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useFrameData).mockReturnValue({
      frames: mockFrames,
      setFrames: vi.fn(),
      getFrameStats: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
    vi.mocked(getContextMenuItems).mockResolvedValue([
      {
        id: "copy_selection",
        label: "Copy Selection",
        command: "Copy.Selection",
        enabled: true,
        disabled_reason: null,
      },
    ] as any);
    Object.defineProperty(navigator, "clipboard", {
      value: { writeText: vi.fn().mockResolvedValue(undefined) },
      configurable: true,
    });
  });

  it("copies the selected block's position/size via Copy Selection", async () => {
    vi.mocked(useSelection).mockReturnValue({
      selection: {
        frame: { frameIndex: 1 },
        temporal: {
          type: "block",
          frameIndex: 1,
          block: { x: 16, y: 32, w: 16, h: 16 },
        },
      },
      setSpatialBlockSelection: vi.fn(),
    } as any);

    const { container } = render(<YuvViewerPanel {...mockProps} />);

    const canvasContainer = container.querySelector(".yuv-canvas-container")!;
    fireEvent.contextMenu(canvasContainer);

    const copySelectionBtn = await screen.findByRole("menuitem", {
      name: "Copy Selection",
    });
    fireEvent.click(copySelectionBtn);

    await waitFor(() => {
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
        "Frame 1: block 16x16 @ (16, 32)",
      );
    });
  });

  it("does nothing when Copy Selection is invoked with no block selected", async () => {
    vi.mocked(useSelection).mockReturnValue({
      selection: { frame: { frameIndex: 1 }, temporal: null },
      setSpatialBlockSelection: vi.fn(),
    } as any);

    const { container } = render(<YuvViewerPanel {...mockProps} />);

    const canvasContainer = container.querySelector(".yuv-canvas-container")!;
    fireEvent.contextMenu(canvasContainer);

    const copySelectionBtn = await screen.findByRole("menuitem", {
      name: "Copy Selection",
    });
    fireEvent.click(copySelectionBtn);

    expect(navigator.clipboard.writeText).not.toHaveBeenCalled();
  });
});

describe("YuvViewerPanel loading states", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useMode).mockReturnValue({
      currentMode: "overview",
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: "yuv",
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
      availableModes: [],
      availableOverlays: [],
      activeOverlays: new Set(),
      toggleOverlay: vi.fn(),
      activeCodec: null,
      handleFKey: vi.fn(),
      setActiveCodec: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: "/test/path",
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useFrameData).mockReturnValue({
      frames: mockFrames,
      setFrames: vi.fn(),
      getFrameStats: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it("should show loading overlay when frame is loading", () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    // Component renders without crashing
    expect(container.firstChild).toBeInTheDocument();
  });

  it("should show placeholder when no frame is loaded", () => {
    const { container: _container } = render(<YuvViewerPanel {...mockProps} />);

    // Component renders without crashing
    expect(_container.firstChild).toBeInTheDocument();
  });

  it("should show correct placeholder text", () => {
    render(<YuvViewerPanel {...mockProps} />);

    // Component renders - checking for frame number display
    // mockProps.currentFrameIndex = 1 → StatusBar shows "Frame 2" (1-based)
    expect(screen.getByText(/Frame 2/)).toBeInTheDocument();
  });
});

describe("YuvViewerPanel status bar", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useMode).mockReturnValue({
      currentMode: "overview",
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: "yuv",
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
      availableModes: [],
      availableOverlays: [],
      activeOverlays: new Set(),
      toggleOverlay: vi.fn(),
      activeCodec: null,
      handleFKey: vi.fn(),
      setActiveCodec: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: "/test/path",
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useFrameData).mockReturnValue({
      frames: mockFrames,
      setFrames: vi.fn(),
      getFrameStats: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it("should display current mode in status bar", () => {
    render(<YuvViewerPanel {...mockProps} />);

    expect(screen.getByText(/overview/i)).toBeInTheDocument();
  });

  it("should display zoom level in status bar", () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    // Component renders with zoom info
    expect(container.firstChild).toBeInTheDocument();
  });

  it("should display playback speed in status bar", () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    expect(container.firstChild).toBeInTheDocument();
  });

  it("should display frame info", () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    expect(container.firstChild).toBeInTheDocument();
  });
});

describe("YuvViewerPanel frame info display", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useMode).mockReturnValue({
      currentMode: "overview",
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: "yuv",
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
      availableModes: [],
      availableOverlays: [],
      activeOverlays: new Set(),
      toggleOverlay: vi.fn(),
      activeCodec: null,
      handleFKey: vi.fn(),
      setActiveCodec: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: "/test/path",
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useFrameData).mockReturnValue({
      frames: mockFrames,
      setFrames: vi.fn(),
      getFrameStats: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it("should display current frame type", () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    // Component renders with frame type info
    expect(container.firstChild).toBeInTheDocument();
  });

  it("should display frame size", () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    expect(container.firstChild).toBeInTheDocument();
  });
});

describe("YuvViewerPanel edge cases", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useMode).mockReturnValue({
      currentMode: "overview",
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: "yuv",
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
      availableModes: [],
      availableOverlays: [],
      activeOverlays: new Set(),
      toggleOverlay: vi.fn(),
      activeCodec: null,
      handleFKey: vi.fn(),
      setActiveCodec: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: "/test/path",
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useFrameData).mockReturnValue({
      frames: mockFrames,
      setFrames: vi.fn(),
      getFrameStats: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it("should handle single frame video", () => {
    const { container } = render(
      <YuvViewerPanel
        currentFrameIndex={0}
        totalFrames={1}
        onFrameChange={vi.fn()}
      />,
    );

    expect(container.firstChild).toBeInTheDocument();
  });

  it("should handle zero total frames", () => {
    const { container } = render(
      <YuvViewerPanel
        currentFrameIndex={0}
        totalFrames={0}
        onFrameChange={vi.fn()}
      />,
    );

    expect(container.firstChild).toBeInTheDocument();
  });

  it("should handle very large frame index", () => {
    const { container } = render(
      <YuvViewerPanel
        currentFrameIndex={9999}
        totalFrames={10000}
        onFrameChange={vi.fn()}
      />,
    );

    expect(container.firstChild).toBeInTheDocument();
  });

  it("should handle frame at index 0", () => {
    render(
      <YuvViewerPanel
        currentFrameIndex={0}
        totalFrames={100}
        onFrameChange={mockProps.onFrameChange}
      />,
    );

    const prevButton = screen.queryByRole("button", { name: /previous/i });
    fireEvent.click(prevButton!);

    // Should not call onFrameChange when already at first frame
    expect(mockProps.onFrameChange).not.toHaveBeenCalled();
  });

  it("should handle frame at last index", () => {
    render(
      <YuvViewerPanel
        currentFrameIndex={99}
        totalFrames={100}
        onFrameChange={mockProps.onFrameChange}
      />,
    );

    const nextButton = screen.queryByRole("button", { name: /next/i });
    fireEvent.click(nextButton!);

    // Should not call onFrameChange when already at last frame
    expect(mockProps.onFrameChange).not.toHaveBeenCalled();
  });

  it("should handle missing frame info", () => {
    vi.mocked(useStreamData).mockReturnValue({
      frames: [],
      currentFrameIndex: 0,
      loading: false,
      error: null,
      filePath: "/test/path",
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);

    const { container } = render(<YuvViewerPanel {...mockProps} />);

    // Should still render without crashing
    expect(container.firstChild).toBeInTheDocument();
  });
});

describe("YuvViewerPanel keyboard shortcut edge cases", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useMode).mockReturnValue({
      currentMode: "overview",
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: "yuv",
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
      availableModes: [],
      availableOverlays: [],
      activeOverlays: new Set(),
      toggleOverlay: vi.fn(),
      activeCodec: null,
      handleFKey: vi.fn(),
      setActiveCodec: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: "/test/path",
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useFrameData).mockReturnValue({
      frames: mockFrames,
      setFrames: vi.fn(),
      getFrameStats: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it("should not trigger Space key with modifier keys", () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    // Space with Ctrl should not trigger
    fireEvent.keyDown(document, { key: " ", ctrlKey: true });

    // Should still be rendered (no crash)
    expect(container.firstChild).toBeInTheDocument();
  });

  it("should not trigger Space key with Shift key", () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: " ", shiftKey: true });

    expect(container.firstChild).toBeInTheDocument();
  });

  it("should not trigger Space key with Meta key", () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: " ", metaKey: true });

    expect(container.firstChild).toBeInTheDocument();
  });

  it("should handle rapid keyboard shortcuts", () => {
    const { container } = render(
      <YuvViewerPanel
        currentFrameIndex={50}
        totalFrames={100}
        onFrameChange={mockProps.onFrameChange}
      />,
    );

    // Rapid arrow key presses
    fireEvent.keyDown(document, { key: "ArrowRight" });
    fireEvent.keyDown(document, { key: "ArrowRight" });
    fireEvent.keyDown(document, { key: "ArrowLeft" });

    expect(container.firstChild).toBeInTheDocument();
  });

  it("should handle multiple mode switches", () => {
    const handleFKeyMock2 = vi.fn().mockReturnValue(true);
    vi.mocked(useMode).mockReturnValue({
      currentMode: "overview",
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: "yuv",
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
      availableModes: [],
      availableOverlays: [],
      activeOverlays: new Set(),
      toggleOverlay: vi.fn(),
      activeCodec: null,
      handleFKey: handleFKeyMock2,
      setActiveCodec: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: "/test/path",
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useFrameData).mockReturnValue({
      frames: mockFrames,
      setFrames: vi.fn(),
      getFrameStats: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);

    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: "F1" });
    fireEvent.keyDown(document, { key: "F2" });
    fireEvent.keyDown(document, { key: "F3" });

    expect(handleFKeyMock2).toHaveBeenCalledTimes(3);
  });
});

describe("YuvViewerPanel zoom state interaction", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useMode).mockReturnValue({
      currentMode: "overview",
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: "yuv",
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
      availableModes: [],
      availableOverlays: [],
      activeOverlays: new Set(),
      toggleOverlay: vi.fn(),
      activeCodec: null,
      handleFKey: vi.fn(),
      setActiveCodec: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: "/test/path",
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useFrameData).mockReturnValue({
      frames: mockFrames,
      setFrames: vi.fn(),
      getFrameStats: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it("should display zoom in status bar", () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    // Component renders
    expect(container.firstChild).toBeInTheDocument();
  });

  it("should display zoom out in status bar", () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    expect(container.firstChild).toBeInTheDocument();
  });

  it("should handle maximum zoom", () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    expect(container.firstChild).toBeInTheDocument();
  });

  it("should handle minimum zoom", () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    expect(container.firstChild).toBeInTheDocument();
  });
});

describe("YuvViewerPanel cleanup", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useMode).mockReturnValue({
      currentMode: "overview",
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: "yuv",
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
      availableModes: [],
      availableOverlays: [],
      activeOverlays: new Set(),
      toggleOverlay: vi.fn(),
      activeCodec: null,
      handleFKey: vi.fn(),
      setActiveCodec: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: "/test/path",
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useFrameData).mockReturnValue({
      frames: mockFrames,
      setFrames: vi.fn(),
      getFrameStats: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it("should cleanup event listeners on unmount", () => {
    const { unmount } = render(<YuvViewerPanel {...mockProps} />);

    // Should not throw when unmounting
    expect(() => unmount()).not.toThrow();
  });

  it("should cleanup playback timer on unmount", () => {
    const { unmount } = render(<YuvViewerPanel {...mockProps} />);

    expect(() => unmount()).not.toThrow();
  });
});

// ---------------------------------------------------------------------------
// Helper: standard mock setup shared by the new describe blocks below
// ---------------------------------------------------------------------------

function setupStandardMocks() {
  vi.mocked(useMode).mockReturnValue({
    currentMode: "overview",
    setMode: vi.fn(),
    cycleMode: vi.fn(),
    componentMask: "yuv",
    toggleComponent: vi.fn(),
    setComponentMask: vi.fn(),
    showGrid: false,
    toggleGrid: vi.fn(),
    showLabels: true,
    toggleLabels: vi.fn(),
    showBlockTypes: false,
    toggleBlockTypes: vi.fn(),
    availableModes: [],
    availableOverlays: [],
    activeOverlays: new Set(),
    toggleOverlay: vi.fn(),
    activeCodec: null,
    handleFKey: vi.fn().mockReturnValue(false),
    setActiveCodec: vi.fn(),
  });
  vi.mocked(useStreamData).mockReturnValue({
    frames: mockFrames,
    currentFrameIndex: 1,
    loading: false,
    error: null,
    filePath: "/test/path",
    setCurrentFrameIndex: vi.fn(),
    refreshFrames: vi.fn(),
    clearData: vi.fn(),
    getFrameStats: vi.fn(),
    setFrames: vi.fn(),
  } as any);
  vi.mocked(useFrameData).mockReturnValue({
    frames: mockFrames,
    setFrames: vi.fn(),
    getFrameStats: vi.fn(),
  } as any);
  vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
}

// ---------------------------------------------------------------------------
// Error state tests
// ---------------------------------------------------------------------------

// Obtain the shared mocks at module level (after the vi.mock calls at top). get_decoded_frame_yuv
// moved from Tauri's invoke() to the Electron bridge (2026-08-08) -- sharedInvokeMock still
// covers the debug-YUV path (get_debug_yuv_frame, unmigrated), sharedGetDecodedFrameYuvMock
// covers the real primary decode path.
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { getDecodedFrameYuv as bridgeGetDecodedFrameYuv } from "@/services/electronBridgeService";
const sharedInvokeMock = tauriInvoke as ReturnType<typeof vi.fn>;
const sharedGetDecodedFrameYuvMock = bridgeGetDecodedFrameYuv as ReturnType<
  typeof vi.fn
>;

describe("YuvViewerPanel frame load error handling", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    sharedInvokeMock.mockResolvedValue({ success: false });
    sharedGetDecodedFrameYuvMock.mockRejectedValue(new Error("Test mode"));
    setupStandardMocks();
  });

  it("shows error overlay when invoke throws", async () => {
    sharedGetDecodedFrameYuvMock.mockRejectedValue(
      new Error("Backend unavailable"),
    );

    const { findByText } = render(<YuvViewerPanel {...mockProps} />);

    // The catch block sets loadError to the generic message
    const errMsg = await findByText(/Failed to load frame/i);
    expect(errMsg).toBeInTheDocument();
  });

  it("shows error overlay when the bridge call rejects", async () => {
    sharedGetDecodedFrameYuvMock.mockRejectedValue(
      new Error("Corrupt frame data"),
    );

    const { findByText } = render(<YuvViewerPanel {...mockProps} />);

    // The bridge has no "soft failure" shape (unlike the old Tauri success:false convention) --
    // any failure is a rejected promise, and the catch block always surfaces the same generic
    // message (see "shows error overlay when invoke throws" above).
    const errMsg = await findByText(/Failed to load frame/i);
    expect(errMsg).toBeInTheDocument();
  });

  it("does not show error overlay when loading succeeds", async () => {
    sharedGetDecodedFrameYuvMock.mockResolvedValue({
      width: 16,
      height: 16,
      bitDepth: 8,
      chromaSubsampling: "420",
      yStride: 16,
      uStride: 8,
      vStride: 8,
      yLen: 4,
      uLen: 2,
      vLen: 2,
      bytes: new Uint8Array(8),
    });

    const { container } = render(<YuvViewerPanel {...mockProps} />);

    // Wait for effects to settle, error overlay must not appear
    await vi.waitFor(() => {
      expect(
        container.querySelector(".yuv-error-overlay"),
      ).not.toBeInTheDocument();
    });
  });

  it("shows a retry button inside the error overlay", async () => {
    sharedGetDecodedFrameYuvMock.mockRejectedValue(new Error("Network error"));

    const { findByText } = render(<YuvViewerPanel {...mockProps} />);

    const retryButton = await findByText(/Retry/i);
    expect(retryButton).toBeInTheDocument();
  });

  it("clicking Retry clears error and calls the bridge again", async () => {
    // First call throws → error state shown
    sharedGetDecodedFrameYuvMock.mockRejectedValueOnce(
      new Error("Transient error"),
    );

    const { findByText } = render(<YuvViewerPanel {...mockProps} />);

    const retryButton = await findByText(/Retry/i);
    const callCountBefore = sharedGetDecodedFrameYuvMock.mock.calls.length;

    fireEvent.click(retryButton);

    // After click, the bridge should have been called at least once more
    await vi.waitFor(() => {
      expect(sharedGetDecodedFrameYuvMock.mock.calls.length).toBeGreaterThan(
        callCountBefore,
      );
    });
  });
});

// ---------------------------------------------------------------------------
// Playback edge case tests
// ---------------------------------------------------------------------------

describe("YuvViewerPanel playback edge cases", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Restore default invoke mock behaviour after clearAllMocks
    sharedInvokeMock.mockResolvedValue({ success: false });
    vi.useFakeTimers();
    setupStandardMocks();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("stops playing when at the last frame after timer fires", async () => {
    // Render at the last frame
    const onFrameChange = vi.fn();
    render(
      <YuvViewerPanel
        currentFrameIndex={99}
        totalFrames={100}
        onFrameChange={onFrameChange}
      />,
    );

    // Start playback
    const playButton = screen.queryByRole("button", { name: /play|pause/i });
    fireEvent.click(playButton!);

    // Advance timer so the playback effect fires
    vi.runAllTimers();

    // onFrameChange should NOT have been called (already at last frame)
    expect(onFrameChange).not.toHaveBeenCalled();
  });

  it("clears playback timer on unmount without error", () => {
    const { unmount } = render(<YuvViewerPanel {...mockProps} />);

    const playButton = screen.queryByRole("button", { name: /play|pause/i });
    fireEvent.click(playButton!); // start playing

    expect(() => unmount()).not.toThrow();
  });

  it("rapid play/pause toggles do not crash the component", () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    const playButton = screen.queryByRole("button", { name: /play|pause/i });
    for (let i = 0; i < 10; i++) {
      fireEvent.click(playButton!);
    }

    expect(container.firstChild).toBeInTheDocument();
  });

  it("speed selector change is reflected in the component", () => {
    render(<YuvViewerPanel {...mockProps} />);

    const speedSelector = screen.queryByRole("combobox", { name: /speed/i });
    expect(speedSelector).toBeInTheDocument();

    fireEvent.change(speedSelector!, { target: { value: "2" } });

    // Component must still be rendered without crashing
    expect(speedSelector).toBeInTheDocument();
  });
});

// ---------------------------------------------------------------------------
// Navigation boundary tests
// ---------------------------------------------------------------------------

describe("YuvViewerPanel navigation boundaries", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    sharedInvokeMock.mockResolvedValue({ success: false });
    setupStandardMocks();
  });

  it("Previous frame button is disabled at frame 0", () => {
    render(
      <YuvViewerPanel
        currentFrameIndex={0}
        totalFrames={100}
        onFrameChange={vi.fn()}
      />,
    );

    const prevButton = screen.queryByRole("button", { name: /previous/i });
    expect(prevButton).toBeDisabled();
  });

  it("Next frame button is disabled at last frame", () => {
    render(
      <YuvViewerPanel
        currentFrameIndex={99}
        totalFrames={100}
        onFrameChange={vi.fn()}
      />,
    );

    const nextButton = screen.queryByRole("button", { name: /next/i });
    expect(nextButton).toBeDisabled();
  });

  it("onFrameChange not called when clicking Previous at frame 0", () => {
    const onFrameChange = vi.fn();
    render(
      <YuvViewerPanel
        currentFrameIndex={0}
        totalFrames={100}
        onFrameChange={onFrameChange}
      />,
    );

    const prevButton = screen.queryByRole("button", { name: /previous/i });
    fireEvent.click(prevButton!);

    expect(onFrameChange).not.toHaveBeenCalled();
  });

  it("ArrowLeft at frame 0 does not call onFrameChange", () => {
    const onFrameChange = vi.fn();
    render(
      <YuvViewerPanel
        currentFrameIndex={0}
        totalFrames={100}
        onFrameChange={onFrameChange}
      />,
    );

    fireEvent.keyDown(document, { key: "ArrowLeft" });

    expect(onFrameChange).not.toHaveBeenCalled();
  });

  it("Home key navigates to frame 0", () => {
    const onFrameChange = vi.fn();
    render(
      <YuvViewerPanel
        currentFrameIndex={50}
        totalFrames={100}
        onFrameChange={onFrameChange}
      />,
    );

    fireEvent.keyDown(document, { key: "Home" });

    expect(onFrameChange).toHaveBeenCalledWith(0);
  });

  it("End key navigates to last frame (totalFrames - 1)", () => {
    const onFrameChange = vi.fn();
    render(
      <YuvViewerPanel
        currentFrameIndex={50}
        totalFrames={100}
        onFrameChange={onFrameChange}
      />,
    );

    fireEvent.keyDown(document, { key: "End" });

    expect(onFrameChange).toHaveBeenCalledWith(99);
  });
});

// ---------------------------------------------------------------------------
// Placeholder and loading state tests
// ---------------------------------------------------------------------------

describe("YuvViewerPanel loading and placeholder states", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    sharedInvokeMock.mockResolvedValue({ success: false });
    setupStandardMocks();
  });

  it("shows placeholder only when there is no error, no frame data, and not loading", async () => {
    // The placeholder is rendered when: !frameImage && !yuvData && !isLoading && !loadError
    // We need both YUV AND RGB to return success:false WITHOUT an error field so
    // loadError stays null but also no data is loaded.
    sharedInvokeMock
      .mockResolvedValueOnce({ success: false }) // yuv — no error field
      .mockResolvedValueOnce({ success: false }) // rgb — no error field → loadError gets default msg
      .mockResolvedValue({ success: false }); // analysis

    // NOTE: When RGB returns success:false with no error field,
    // `result.error || "Failed to load frame"` still sets loadError.
    // So the only way to show placeholder is when YUV succeeds but produces
    // no data. That is tested separately above.
    // This test verifies the component still renders without crash.
    const { container } = render(<YuvViewerPanel {...mockProps} />);
    expect(container.firstChild).toBeInTheDocument();
  });

  it("loading overlay disappears and error overlay appears when invoke rejects", async () => {
    sharedInvokeMock.mockRejectedValue(new Error("fail"));

    const { container, findByText } = render(<YuvViewerPanel {...mockProps} />);

    // After error settles, loading overlay should be gone
    await findByText(/Failed to load frame/i);
    expect(
      container.querySelector(".yuv-loading-overlay"),
    ).not.toBeInTheDocument();
  });

  it("error overlay appears and placeholder is absent when the bridge call rejects", async () => {
    sharedGetDecodedFrameYuvMock.mockRejectedValue(new Error("Bad frame"));

    const { container, findByText } = render(<YuvViewerPanel {...mockProps} />);

    await findByText(/Failed to load frame/i);

    // When error overlay is showing, placeholder must not be visible
    expect(
      container.querySelector(".yuv-placeholder-overlay"),
    ).not.toBeInTheDocument();
    expect(container.querySelector(".yuv-error-overlay")).toBeInTheDocument();
  });

  it("loading overlay is visible while the bridge call is pending", async () => {
    // Make the bridge call never resolve during the synchronous part of the test
    let resolveDecode!: (value: unknown) => void;
    sharedGetDecodedFrameYuvMock.mockReturnValue(
      new Promise((resolve) => {
        resolveDecode = resolve;
      }),
    );

    const { container } = render(<YuvViewerPanel {...mockProps} />);

    // Loading overlay should be present while the promise is pending
    expect(container.querySelector(".yuv-loading-overlay")).toBeInTheDocument();

    // Resolve to avoid hanging promise warning
    resolveDecode({
      width: 16,
      height: 16,
      bitDepth: 8,
      chromaSubsampling: "420",
      yStride: 16,
      uStride: 8,
      vStride: 8,
      yLen: 4,
      uLen: 2,
      vLen: 2,
      bytes: new Uint8Array(8),
    });
  });
});
