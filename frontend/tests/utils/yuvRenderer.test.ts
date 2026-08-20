/**
 * Regression test for a real bug found via screenshot verification (2026-08-08): `YUVRenderer.
 * render()` checked `!this.imageData` and returned before ever calling `resize()` -- the only
 * place `imageData` is created. Since `imageData` starts `null`, `render()` always returned
 * immediately on the very first call, so nothing was ever actually painted to the canvas (it
 * stayed whatever `clear()`/fillRect left it as -- solid black in `VideoCanvas.tsx`'s usage).
 * This went unnoticed because no test previously exercised `YUVRenderer.render()` directly.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { YUVRenderer } from "@/utils/yuv/renderer";
import type { YUVFrame } from "@/types/yuv";

// jsdom doesn't implement ImageData -- YUVRenderer.resize() constructs one directly, so a
// minimal polyfill is needed to exercise the real code path (not just a mocked canvas).
if (typeof globalThis.ImageData === "undefined") {
  class ImageDataPolyfill {
    data: Uint8ClampedArray;
    width: number;
    height: number;
    colorSpace = "srgb" as const;
    constructor(width: number, height: number) {
      this.width = width;
      this.height = height;
      this.data = new Uint8ClampedArray(width * height * 4);
    }
  }
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  globalThis.ImageData = ImageDataPolyfill as any;
}

function createMockCanvas(
  width: number,
  height: number,
): {
  canvas: HTMLCanvasElement;
  putImageData: ReturnType<typeof vi.fn>;
  drawImage: ReturnType<typeof vi.fn>;
} {
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const putImageData = vi.fn();
  const drawImage = vi.fn();
  const mockCtx = {
    putImageData,
    drawImage,
  };
  // Mocked on the *prototype*, not just this one `canvas` instance -- YUVRenderer now paints
  // decoded pixels onto an internal same-resolution offscreen canvas (via `document.
  // createElement("canvas")`, invisible to the test) and `drawImage`-scales that onto the real
  // display canvas, so a real `putImageData` call only becomes observable this way, not via
  // spying on `canvas` alone (see renderer.ts's own doc comment on why: putImageData ignores the
  // canvas transform entirely, so painting straight onto a devicePixelRatio-scaled display canvas
  // would only fill its top-left corner).
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(
    mockCtx as any,
  );
  return { canvas, putImageData, drawImage };
}

function makeFrame(width: number, height: number): YUVFrame {
  return {
    width,
    height,
    y: new Uint8Array(width * height).fill(200),
    u: new Uint8Array((width / 2) * (height / 2)).fill(128),
    v: new Uint8Array((width / 2) * (height / 2)).fill(128),
    yStride: width,
    uStride: width / 2,
    vStride: width / 2,
    chromaSubsampling: "420",
  };
}

describe("YUVRenderer.render", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("paints on the very first call, not just on subsequent ones", () => {
    const { canvas, putImageData, drawImage } = createMockCanvas(4, 4);
    const renderer = new YUVRenderer(canvas);

    renderer.render(makeFrame(4, 4));

    expect(putImageData).toHaveBeenCalledTimes(1);
    // ...and it actually reaches the real display canvas (offscreen -> drawImage), not just the
    // internal offscreen buffer.
    expect(drawImage).toHaveBeenCalledTimes(1);
  });

  it("paints real (non-empty) pixel data, not a blank buffer", () => {
    const { canvas, putImageData } = createMockCanvas(4, 4);
    const renderer = new YUVRenderer(canvas);

    renderer.render(makeFrame(4, 4));

    const painted = putImageData.mock.calls[0][0] as ImageData;
    // Y=200 (bright) should not convert to every RGBA byte being 0.
    expect(Array.from(painted.data).some((byte) => byte !== 0)).toBe(true);
  });

  it("continues to paint on repeated calls at the same size", () => {
    const { canvas, putImageData, drawImage } = createMockCanvas(4, 4);
    const renderer = new YUVRenderer(canvas);

    renderer.render(makeFrame(4, 4));
    renderer.render(makeFrame(4, 4));
    renderer.render(makeFrame(4, 4));

    expect(putImageData).toHaveBeenCalledTimes(3);
    expect(drawImage).toHaveBeenCalledTimes(3);
  });

  it("does nothing (no throw) when no canvas was ever attached", () => {
    const renderer = new YUVRenderer();
    expect(() => renderer.render(makeFrame(4, 4))).not.toThrow();
  });
});
