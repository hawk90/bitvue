/**
 * Pixel value lookup (INT-01: Player hover pixel/block tooltip).
 *
 * Reads the real decoded Y/U/V sample values at a given frame-native pixel coordinate. Pure and
 * DOM-free, same reasoning as `spatialBlockHitTest.ts` -- `VideoCanvas.tsx` handles the client
 * coordinate -> frame coordinate conversion separately and passes plain numbers in here.
 */

import type { YUVFrame, ChromaSubsampling } from "../types/yuv";

export interface PixelValue {
  /** Luma (Y) sample, 0-255 for 8-bit content. */
  y: number;
  /** Chroma-blue (U) sample at this pixel's subsampled chroma cell. */
  u: number;
  /** Chroma-red (V) sample at this pixel's subsampled chroma cell. */
  v: number;
}

/** Same 420/422/444 -> (scaleX, scaleY) convention `VideoCanvas.tsx`'s `applyChannelMode` already
 *  uses for chroma upscaling -- kept in sync deliberately, not reinvented here. */
function chromaScale(subsampling: ChromaSubsampling): {
  scaleX: number;
  scaleY: number;
} {
  const scaleX = subsampling === "420" || subsampling === "422" ? 2 : 1;
  const scaleY = subsampling === "420" ? 2 : 1;
  return { scaleX, scaleY };
}

export function resolvePixelValueAtPoint(
  frameX: number,
  frameY: number,
  yuv: YUVFrame,
): PixelValue | null {
  const px = Math.floor(frameX);
  const py = Math.floor(frameY);
  if (px < 0 || py < 0 || px >= yuv.width || py >= yuv.height) return null;

  const yIndex = py * yuv.yStride + px;
  const yValue = yuv.y[yIndex];
  if (yValue === undefined) return null;

  const { scaleX, scaleY } = chromaScale(yuv.chromaSubsampling);
  const cx = Math.floor(px / scaleX);
  const cy = Math.floor(py / scaleY);
  const uValue = yuv.u[cy * yuv.uStride + cx] ?? 0;
  const vValue = yuv.v[cy * yuv.vStride + cx] ?? 0;

  return { y: yValue, u: uValue, v: vValue };
}
