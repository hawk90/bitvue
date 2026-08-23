/**
 * VideoCanvas Component Tests
 * Tests video canvas with zoom and pan support
 */

import { describe, it, expect, vi } from "vitest";
import { render, fireEvent } from "@/test/test-utils";
import { VideoCanvas } from "../YuvViewerPanel/VideoCanvas";

// Mock OverlayRenderer (VideoCanvas imports renderModeOverlay from "../OverlayRenderer")
vi.mock("../OverlayRenderer", () => ({
  renderModeOverlay: vi.fn(),
}));

// jsdom's getBoundingClientRect always returns an all-zero rect -- stub it to a chosen logical
// size so client->frame coordinate conversion is a 1:1 mapping at zoom=1. Shared by the spatial
// click and hover-tooltip suites below.
function stubCanvasRect(width: number, height: number) {
  return vi
    .spyOn(HTMLCanvasElement.prototype, "getBoundingClientRect")
    .mockReturnValue({
      x: 0,
      y: 0,
      left: 0,
      top: 0,
      right: width,
      bottom: height,
      width,
      height,
      toJSON: () => {},
    });
}

describe("VideoCanvas", () => {
  const defaultProps = {
    frameImage: null,
    currentFrameIndex: 0,
    currentFrame: null,
    currentMode: "overview" as const,
    zoom: 1,
    pan: { x: 0, y: 0 },
    onWheel: vi.fn(),
    onMouseDown: vi.fn(),
    onMouseMove: vi.fn(),
    onMouseUp: vi.fn(),
    isDragging: false,
  };

  it("should render video canvas", () => {
    render(<VideoCanvas {...defaultProps} />);

    const canvas = document.querySelector(".yuv-canvas");
    expect(canvas).toBeInTheDocument();
  });

  it("should render canvas container", () => {
    render(<VideoCanvas {...defaultProps} />);

    const container = document.querySelector(".yuv-canvas-container");
    expect(container).toBeInTheDocument();
  });

  it("should apply zoom transform", () => {
    const { container } = render(<VideoCanvas {...defaultProps} zoom={1.5} />);

    const canvas = container.querySelector(".yuv-canvas");
    expect(canvas?.style.transform).toContain("scale(1.5)");
  });

  it("should apply pan transform", () => {
    const { container } = render(
      <VideoCanvas {...defaultProps} pan={{ x: 50, y: 25 }} />,
    );

    const canvas = container.querySelector(".yuv-canvas");
    expect(canvas?.style.transform).toContain("translate");
  });

  it("should show grab cursor when not dragging", () => {
    const { container } = render(
      <VideoCanvas {...defaultProps} isDragging={false} />,
    );

    const canvasContainer = container.querySelector(".yuv-canvas-container");
    expect(canvasContainer?.style.cursor).toBe("grab");
  });

  it("should show grabbing cursor when dragging", () => {
    const { container } = render(
      <VideoCanvas {...defaultProps} isDragging={true} />,
    );

    const canvasContainer = container.querySelector(".yuv-canvas-container");
    expect(canvasContainer?.style.cursor).toBe("grabbing");
  });

  it("should call onWheel on wheel event", () => {
    const handleWheel = vi.fn();
    render(<VideoCanvas {...defaultProps} onWheel={handleWheel} />);

    const container = document.querySelector(".yuv-canvas-container");
    if (container) {
      fireEvent.wheel(container, { deltaY: 100 });
      expect(handleWheel).toHaveBeenCalled();
    }
  });

  it("should call onMouseDown on mouse down", () => {
    const handleMouseDown = vi.fn();
    render(<VideoCanvas {...defaultProps} onMouseDown={handleMouseDown} />);

    const container = document.querySelector(".yuv-canvas-container");
    if (container) {
      fireEvent.mouseDown(container);
      expect(handleMouseDown).toHaveBeenCalled();
    }
  });

  it("should call onMouseMove on mouse move", () => {
    const handleMouseMove = vi.fn();
    render(<VideoCanvas {...defaultProps} onMouseMove={handleMouseMove} />);

    const container = document.querySelector(".yuv-canvas-container");
    if (container) {
      fireEvent.mouseMove(container);
      expect(handleMouseMove).toHaveBeenCalled();
    }
  });

  it("should call onMouseUp on mouse up", () => {
    const handleMouseUp = vi.fn();
    render(<VideoCanvas {...defaultProps} onMouseUp={handleMouseUp} />);

    const container = document.querySelector(".yuv-canvas-container");
    if (container) {
      fireEvent.mouseUp(container);
      expect(handleMouseUp).toHaveBeenCalled();
    }
  });

  it("should call onMouseUp on mouse leave", () => {
    const handleMouseUp = vi.fn();
    render(<VideoCanvas {...defaultProps} onMouseUp={handleMouseUp} />);

    const container = document.querySelector(".yuv-canvas-container");
    if (container) {
      fireEvent.mouseLeave(container);
      expect(handleMouseUp).toHaveBeenCalled();
    }
  });

  it("should use React.memo for performance", () => {
    const { rerender } = render(<VideoCanvas {...defaultProps} />);

    rerender(<VideoCanvas {...defaultProps} />);

    expect(document.querySelector(".yuv-canvas")).toBeInTheDocument();
  });
});

describe("VideoCanvas with frame image", () => {
  const mockImage = new Image();
  mockImage.width = 640;
  mockImage.height = 360;
  // Mock the src to avoid actual loading
  Object.defineProperty(mockImage, "src", {
    value: "data:image/test",
    writable: true,
  });
  Object.defineProperty(mockImage, "complete", { value: true, writable: true });

  const defaultProps = {
    frameImage: mockImage,
    currentFrameIndex: 0,
    currentFrame: { frame_index: 0, frame_type: "I", size: 50000, poc: 0 },
    currentMode: "overview" as const,
    zoom: 1,
    pan: { x: 0, y: 0 },
    onWheel: vi.fn(),
    onMouseDown: vi.fn(),
    onMouseMove: vi.fn(),
    onMouseUp: vi.fn(),
    isDragging: false,
  };

  it("should set canvas dimensions to match image", () => {
    const { container } = render(<VideoCanvas {...defaultProps} />);

    const canvas = container.querySelector("canvas");
    expect(canvas?.width).toBe(640);
    expect(canvas?.height).toBe(360);
  });

  it("should have correct transform origin", () => {
    const { container } = render(<VideoCanvas {...defaultProps} />);

    const canvas = container.querySelector(".yuv-canvas");
    expect(canvas?.style.transformOrigin).toBe("top left");
  });
});

// INT-01: Player click (not drag-pan) -> spatialBlock select
describe("VideoCanvas spatial block click", () => {
  const frameWithQpGrid = {
    frame_index: 0,
    frame_type: "I",
    size: 50000,
    qp_grid: {
      grid_w: 4,
      grid_h: 4,
      block_w: 16,
      block_h: 16,
      qp: new Array(16).fill(20),
      qp_min: 20,
      qp_max: 20,
    },
  };

  const defaultProps = {
    frameImage: null,
    currentFrameIndex: 0,
    currentFrame: frameWithQpGrid,
    currentMode: "overview" as const,
    zoom: 1,
    pan: { x: 0, y: 0 },
    onWheel: vi.fn(),
    onMouseDown: vi.fn(),
    onMouseMove: vi.fn(),
    onMouseUp: vi.fn(),
    isDragging: false,
  };

  it("fires onSpatialBlockClick with the resolved block on a real click (no movement)", () => {
    stubCanvasRect(640, 360);
    const handleClick = vi.fn();
    const { container } = render(
      <VideoCanvas {...defaultProps} onSpatialBlockClick={handleClick} />,
    );
    const canvasContainer = container.querySelector(".yuv-canvas-container")!;

    fireEvent.mouseDown(canvasContainer, { clientX: 20, clientY: 5 });
    fireEvent.mouseUp(canvasContainer, { clientX: 20, clientY: 5 });

    // (20, 5) falls in qp_grid cell col=1,row=0 -> block at (16, 0), 16x16.
    expect(handleClick).toHaveBeenCalledWith({ x: 16, y: 0, w: 16, h: 16 });
  });

  it("does not fire onSpatialBlockClick when the pointer moved beyond the click threshold (a drag-pan)", () => {
    stubCanvasRect(640, 360);
    const handleClick = vi.fn();
    const { container } = render(
      <VideoCanvas {...defaultProps} onSpatialBlockClick={handleClick} />,
    );
    const canvasContainer = container.querySelector(".yuv-canvas-container")!;

    fireEvent.mouseDown(canvasContainer, { clientX: 10, clientY: 10 });
    fireEvent.mouseUp(canvasContainer, { clientX: 40, clientY: 40 });

    expect(handleClick).not.toHaveBeenCalled();
  });

  it("does not fire onSpatialBlockClick when the click lands outside every resolvable block", () => {
    stubCanvasRect(640, 360);
    const handleClick = vi.fn();
    const { container } = render(
      <VideoCanvas {...defaultProps} onSpatialBlockClick={handleClick} />,
    );
    const canvasContainer = container.querySelector(".yuv-canvas-container")!;

    fireEvent.mouseDown(canvasContainer, { clientX: 500, clientY: 500 });
    fireEvent.mouseUp(canvasContainer, { clientX: 500, clientY: 500 });

    expect(handleClick).not.toHaveBeenCalled();
  });

  it("still calls the underlying onMouseDown/onMouseUp props for pan even when onSpatialBlockClick is provided", () => {
    stubCanvasRect(640, 360);
    const handleMouseDown = vi.fn();
    const handleMouseUp = vi.fn();
    const { container } = render(
      <VideoCanvas
        {...defaultProps}
        onMouseDown={handleMouseDown}
        onMouseUp={handleMouseUp}
        onSpatialBlockClick={vi.fn()}
      />,
    );
    const canvasContainer = container.querySelector(".yuv-canvas-container")!;

    fireEvent.mouseDown(canvasContainer, { clientX: 20, clientY: 5 });
    fireEvent.mouseUp(canvasContainer, { clientX: 20, clientY: 5 });

    expect(handleMouseDown).toHaveBeenCalled();
    expect(handleMouseUp).toHaveBeenCalled();
  });

  it("does not throw when onSpatialBlockClick is omitted (optional prop)", () => {
    stubCanvasRect(640, 360);
    const { container } = render(<VideoCanvas {...defaultProps} />);
    const canvasContainer = container.querySelector(".yuv-canvas-container")!;

    expect(() => {
      fireEvent.mouseDown(canvasContainer, { clientX: 20, clientY: 5 });
      fireEvent.mouseUp(canvasContainer, { clientX: 20, clientY: 5 });
    }).not.toThrow();
  });
});

// INT-01: Player hover pixel/block tooltip
describe("VideoCanvas hover pixel tooltip", () => {
  const frameWithQpGrid = {
    frame_index: 0,
    frame_type: "I",
    size: 50000,
    qp_grid: {
      grid_w: 4,
      grid_h: 4,
      block_w: 16,
      block_h: 16,
      qp: new Array(16).fill(20),
      qp_min: 20,
      qp_max: 20,
    },
  };

  function makeYuv420(width: number, height: number) {
    const chromaW = width / 2;
    const chromaH = height / 2;
    return {
      y: new Uint8Array(width * height).fill(100),
      u: new Uint8Array(chromaW * chromaH).fill(50),
      v: new Uint8Array(chromaW * chromaH).fill(150),
      width,
      height,
      yStride: width,
      uStride: chromaW,
      vStride: chromaW,
      chromaSubsampling: "420" as const,
    };
  }

  const defaultProps = {
    frameImage: null,
    currentFrameIndex: 3,
    currentFrame: frameWithQpGrid,
    currentMode: "overview" as const,
    zoom: 1,
    pan: { x: 0, y: 0 },
    onWheel: vi.fn(),
    onMouseDown: vi.fn(),
    onMouseMove: vi.fn(),
    onMouseUp: vi.fn(),
    isDragging: false,
    yuvData: makeYuv420(640, 360),
  };

  it("shows a tooltip with frame index, pixel value, and block info on hover", () => {
    stubCanvasRect(640, 360);
    const { container } = render(<VideoCanvas {...defaultProps} />);
    const canvasContainer = container.querySelector(".yuv-canvas-container")!;

    fireEvent.mouseMove(canvasContainer, { clientX: 20, clientY: 5 });

    const tooltip = container.querySelector(".player-pixel-tooltip");
    expect(tooltip).toBeInTheDocument();
    expect(tooltip?.textContent).toContain("Frame #3");
    expect(tooltip?.textContent).toContain("Y 100");
    expect(tooltip?.textContent).toContain("U 50");
    expect(tooltip?.textContent).toContain("V 150");
    expect(tooltip?.textContent).toContain("Block 16×16");
  });

  it("still calls the underlying onMouseMove prop for panning", () => {
    stubCanvasRect(640, 360);
    const handleMouseMove = vi.fn();
    const { container } = render(
      <VideoCanvas {...defaultProps} onMouseMove={handleMouseMove} />,
    );
    const canvasContainer = container.querySelector(".yuv-canvas-container")!;

    fireEvent.mouseMove(canvasContainer, { clientX: 20, clientY: 5 });

    expect(handleMouseMove).toHaveBeenCalled();
  });

  it("hides the tooltip once the pointer leaves the canvas", () => {
    stubCanvasRect(640, 360);
    const { container } = render(<VideoCanvas {...defaultProps} />);
    const canvasContainer = container.querySelector(".yuv-canvas-container")!;

    fireEvent.mouseMove(canvasContainer, { clientX: 20, clientY: 5 });
    expect(
      container.querySelector(".player-pixel-tooltip"),
    ).toBeInTheDocument();

    fireEvent.mouseLeave(canvasContainer);
    expect(container.querySelector(".player-pixel-tooltip")).toBeNull();
  });

  it("suppresses the tooltip while dragging (drag-pan, not hover)", () => {
    stubCanvasRect(640, 360);
    const { container } = render(
      <VideoCanvas {...defaultProps} isDragging={true} />,
    );
    const canvasContainer = container.querySelector(".yuv-canvas-container")!;

    fireEvent.mouseMove(canvasContainer, { clientX: 20, clientY: 5 });

    expect(container.querySelector(".player-pixel-tooltip")).toBeNull();
  });

  it("does not show a tooltip when the pointer is outside the frame bounds", () => {
    stubCanvasRect(640, 360);
    const { container } = render(<VideoCanvas {...defaultProps} />);
    const canvasContainer = container.querySelector(".yuv-canvas-container")!;

    fireEvent.mouseMove(canvasContainer, { clientX: 900, clientY: 900 });

    expect(container.querySelector(".player-pixel-tooltip")).toBeNull();
  });

  it("omits pixel/block rows gracefully when there is no yuvData or currentFrame", () => {
    stubCanvasRect(640, 360);
    const { container } = render(
      <VideoCanvas {...defaultProps} yuvData={undefined} currentFrame={null} />,
    );
    const canvasContainer = container.querySelector(".yuv-canvas-container")!;

    fireEvent.mouseMove(canvasContainer, { clientX: 20, clientY: 5 });

    const tooltip = container.querySelector(".player-pixel-tooltip");
    expect(tooltip).toBeInTheDocument();
    expect(tooltip?.textContent).toContain("Frame #3");
    expect(tooltip?.textContent).not.toContain("Y ");
    expect(tooltip?.textContent).not.toContain("Block");
  });
});
