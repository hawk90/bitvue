/**
 * pixelValueLookup tests (INT-01: Player hover pixel/block tooltip)
 */

import { describe, it, expect } from "vitest";
import { resolvePixelValueAtPoint } from "@/utils/pixelValueLookup";
import type { YUVFrame } from "@/types/yuv";

function makeYuv420(width: number, height: number): YUVFrame {
  const ySize = width * height;
  const chromaW = Math.ceil(width / 2);
  const chromaH = Math.ceil(height / 2);
  const y = new Uint8Array(ySize);
  const u = new Uint8Array(chromaW * chromaH);
  const v = new Uint8Array(chromaW * chromaH);
  // Deterministic, position-dependent values so a wrong index is easy to catch.
  for (let py = 0; py < height; py++) {
    for (let px = 0; px < width; px++) {
      y[py * width + px] = (px + py * 3) % 256;
    }
  }
  for (let cy = 0; cy < chromaH; cy++) {
    for (let cx = 0; cx < chromaW; cx++) {
      u[cy * chromaW + cx] = (cx + 1) % 256;
      v[cy * chromaW + cx] = (cy + 1) % 256;
    }
  }
  return {
    y,
    u,
    v,
    width,
    height,
    yStride: width,
    uStride: chromaW,
    vStride: chromaW,
    chromaSubsampling: "420",
  };
}

describe("resolvePixelValueAtPoint", () => {
  it("reads the real Y sample at the given pixel", () => {
    const yuv = makeYuv420(8, 8);
    const result = resolvePixelValueAtPoint(3, 2, yuv);
    expect(result).not.toBeNull();
    expect(result!.y).toBe(yuv.y[2 * 8 + 3]);
  });

  it("maps to the correct subsampled chroma cell for 4:2:0", () => {
    const yuv = makeYuv420(8, 8);
    // Pixel (3,2) -> chroma cell (1,1) under 4:2:0 (divide both axes by 2, floor).
    const result = resolvePixelValueAtPoint(3, 2, yuv);
    const chromaW = 4;
    expect(result!.u).toBe(yuv.u[1 * chromaW + 1]);
    expect(result!.v).toBe(yuv.v[1 * chromaW + 1]);
  });

  it("floors fractional coordinates to the containing pixel", () => {
    const yuv = makeYuv420(8, 8);
    const exact = resolvePixelValueAtPoint(3, 2, yuv);
    const fractional = resolvePixelValueAtPoint(3.9, 2.4, yuv);
    expect(fractional).toEqual(exact);
  });

  it("returns null for out-of-bounds coordinates", () => {
    const yuv = makeYuv420(8, 8);
    expect(resolvePixelValueAtPoint(-1, 0, yuv)).toBeNull();
    expect(resolvePixelValueAtPoint(0, -1, yuv)).toBeNull();
    expect(resolvePixelValueAtPoint(8, 0, yuv)).toBeNull();
    expect(resolvePixelValueAtPoint(0, 8, yuv)).toBeNull();
  });

  it("handles 4:4:4 (no chroma subsampling) correctly", () => {
    const width = 4;
    const height = 4;
    const y = new Uint8Array(width * height).fill(10);
    const u = new Uint8Array(width * height);
    const v = new Uint8Array(width * height);
    u[2 * width + 2] = 77;
    v[2 * width + 2] = 88;
    const yuv: YUVFrame = {
      y,
      u,
      v,
      width,
      height,
      yStride: width,
      uStride: width,
      vStride: width,
      chromaSubsampling: "444",
    };

    const result = resolvePixelValueAtPoint(2, 2, yuv);
    expect(result).toEqual({ y: 10, u: 77, v: 88 });
  });

  it("handles 4:2:2 (horizontal-only subsampling) correctly", () => {
    const width = 8;
    const height = 4;
    const chromaW = 4;
    const y = new Uint8Array(width * height);
    const u = new Uint8Array(chromaW * height);
    const v = new Uint8Array(chromaW * height);
    // Pixel (5,3) -> chroma cell (2,3) under 4:2:2 (halve X only, Y unchanged).
    u[3 * chromaW + 2] = 42;
    v[3 * chromaW + 2] = 43;
    const yuv: YUVFrame = {
      y,
      u,
      v,
      width,
      height,
      yStride: width,
      uStride: chromaW,
      vStride: chromaW,
      chromaSubsampling: "422",
    };

    const result = resolvePixelValueAtPoint(5, 3, yuv);
    expect(result).toEqual({ y: 0, u: 42, v: 43 });
  });
});
