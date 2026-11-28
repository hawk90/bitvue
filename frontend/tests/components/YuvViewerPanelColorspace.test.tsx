/* eslint-disable @typescript-eslint/no-explicit-any */
/**
 * YuvViewerPanel colour-space menu wiring (DEC-008): Options > Color Space events -- dispatched by
 * both the macOS native menu and the Windows/Linux TitleBar -- must reach the renderer as the
 * selected YUV->RGB matrix (COLORSPACE_MATRICES key) passed to VideoCanvas.
 * Mock setup copied from YuvViewerPanel.test.tsx; VideoCanvas is additionally mocked to capture
 * its `colorspace` prop.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { act, fireEvent, render, screen } from "@testing-library/react";
import { YuvViewerPanel } from "../YuvViewerPanel";
import { useMode } from "@/contexts/ModeContext";
import { useStreamData } from "@/contexts/StreamDataContext";
import { useFrameData } from "@/contexts/FrameDataContext";
import { useCanvasInteraction } from "@/hooks/useCanvasInteraction";
import { MENU_EVENTS } from "../../../bitvue-desktop/electron/menuEvents";
import { TitleBar } from "@/components/TitleBar";
import { COLORSPACE_MATRICES, Colorspace } from "@/types/yuv";

vi.mock("@/components/panels/YuvViewerPanel/VideoCanvas", () => ({
  VideoCanvas: (props: { colorspace?: string }) => (
    <div data-testid="video-canvas" data-colorspace={props.colorspace} />
  ),
}));

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

// Mock the Electron bridge's getDecodedFrameYuv — the real decode path (2026-08-08 migration
// off Tauri's get_decoded_frame_yuv). The component actually calls the cancellable variant
// (axis-6 cancellation wiring) -- that's implemented here as a thin wrapper around the same
// `getDecodedFrameYuv` mock so every existing `sharedGetDecodedFrameYuvMock.mockResolvedValue/
// mockRejectedValue(...)` call below keeps controlling the resolved/rejected frame data, just
// via one extra layer of `{promise, cancel}` indirection matching the real bridge shape.
vi.mock("@/services/electronBridgeService", async () => {
  const { bridgeYuvToFrame: actualBridgeYuvToFrame } = await vi.importActual<
    typeof import("@/services/bridge/frameDecode")
  >("@/services/bridge/frameDecode");
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
    // The component also imports these; without them vitest throws on access and the
    // component takes its error path (and logs it) on every render.
    getDebugYuvFrame: vi.fn(() => Promise.reject(new Error("Test mode"))),
    getFrameAnalysis: vi.fn(() => Promise.reject(new Error("Test mode"))),
    bridgeYuvToFrame: actualBridgeYuvToFrame,
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

/**
 * Render and let the panel's mount-time async loads settle inside act(). The panel starts
 * frame/analysis requests from an effect; with a bare render() they resolve right after the
 * (synchronous) test body, outside act(), and React warns on every test.
 */
async function renderPanel(ui: React.ReactElement) {
  let result!: ReturnType<typeof render>;
  await act(async () => {
    result = render(ui);
  });
  return result;
}

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

describe("YuvViewerPanel Options > Color Space", () => {
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

  const canvas = () => screen.getByTestId("video-canvas");
  const fire = (event: string) =>
    act(() => {
      window.dispatchEvent(new CustomEvent(event));
    });

  it("defaults to BT.709", async () => {
    await renderPanel(<YuvViewerPanel {...mockProps} />);
    expect(canvas()).toHaveAttribute("data-colorspace", Colorspace.BT709);
  });

  it.each([
    [MENU_EVENTS.colorBt601, Colorspace.BT601],
    [MENU_EVENTS.colorBt2020, Colorspace.BT2020],
    [MENU_EVENTS.colorBt709, Colorspace.BT709],
  ])("native-menu event %s selects %s", async (event, expected) => {
    await renderPanel(<YuvViewerPanel {...mockProps} />);
    if (expected === Colorspace.BT709) fire(MENU_EVENTS.colorBt2020);
    fire(event);
    expect(canvas()).toHaveAttribute("data-colorspace", expected);
    expect(COLORSPACE_MATRICES[expected]).toBeDefined();
  });

  it.each([
    ["ITU Rec. 601", Colorspace.BT601],
    ["ITU Rec. 2020", Colorspace.BT2020],
  ])("TitleBar item %s selects %s", async (label, expected) => {
    await renderPanel(
      <>
        <TitleBar fileName="x" onOpenFile={vi.fn()} />
        <YuvViewerPanel {...mockProps} />
      </>,
    );
    fireEvent.mouseEnter(screen.getByText("Options"));
    fireEvent.mouseEnter(screen.getByText("Color Space"));
    fireEvent.click(screen.getByText(label));
    expect(canvas()).toHaveAttribute("data-colorspace", expected);
  });
});
