/**
 * YUV to RGB Conversion Utilities
 *
 * Core YUV to RGB pixel conversion functions.
 * Extracted from yuvRenderer.ts for better modularity.
 */

import type { ColorspaceMatrix } from "../../types/yuv";
import { Colorspace, COLORSPACE_MATRICES } from "../../types/yuv";

/** RGB components shift amounts for packing */
const RGB_SHIFT: Readonly<{
  R: number;
  G: number;
  B: number;
  A: number;
}> = {
  R: 16,
  G: 8,
  B: 0,
  A: 24,
} as const;

/** RGBA channel indices in ImageData */
const RGBA_CHANNELS: Readonly<{
  R: number;
  G: number;
  B: number;
  A: number;
}> = {
  R: 0,
  G: 1,
  B: 2,
  A: 3,
} as const;

/** Default alpha value (fully opaque) */
const DEFAULT_ALPHA = 255;

/**
 * Get colorspace conversion matrix
 *
 * @param colorspace - The colorspace to get coefficients for
 * @returns The YUV to RGB conversion matrix
 */
export function getColorspaceMatrix(colorspace: Colorspace): ColorspaceMatrix {
  return COLORSPACE_MATRICES[colorspace];
}

/**
 * Convert YUV to RGB for a single pixel
 *
 * @param y - Y component (0-255)
 * @param u - U component (0-255)
 * @param v - V component (0-255)
 * @param colorspace - Colorspace to use for conversion
 * @returns RGB value as 24-bit integer (0xRRGGBB)
 */
export function yuvToRgb(
  y: number,
  u: number,
  v: number,
  colorspace: Colorspace = Colorspace.BT709,
): number {
  // Convert UV from [0, 255] to [-128, 127]
  const u0 = u - 128;
  const v0 = v - 128;

  // Get colorspace conversion matrix
  const m = getColorspaceMatrix(colorspace);

  // Apply YUV to RGB conversion using matrix coefficients
  const r = y + m.rv * v0;
  const g = y + m.gu * u0 + m.gv * v0;
  const b = y + m.bu * u0;

  // Clamp to [0, 255] and convert to integer
  const rClamped = Math.max(0, Math.min(255, Math.round(r)));
  const gClamped = Math.max(0, Math.min(255, Math.round(g)));
  const bClamped = Math.max(0, Math.min(255, Math.round(b)));

  // Pack as RGB (0xRRGGBB)
  return (
    (rClamped << RGB_SHIFT.R) |
    (gClamped << RGB_SHIFT.G) |
    (bClamped << RGB_SHIFT.B)
  );
}

/**
 * Convert YUV to RGBA array for a single pixel
 *
 * @param y - Y component (0-255)
 * @param u - U component (0-255)
 * @param v - V component (0-255)
 * @param colorspace - Colorspace to use for conversion
 * @param out - Output array to write RGBA values
 * @param outIndex - Starting index in output array
 */
export function yuvToRgbaArray(
  y: number,
  u: number,
  v: number,
  colorspace: Colorspace,
  out: Uint8ClampedArray,
  outIndex: number,
): void {
  const rgb = yuvToRgb(y, u, v, colorspace);

  // Write RGBA (ImageData stores pixels as RGBA)
  out[outIndex + RGBA_CHANNELS.R] = (rgb >> RGB_SHIFT.R) & 0xff;
  out[outIndex + RGBA_CHANNELS.G] = (rgb >> RGB_SHIFT.G) & 0xff;
  out[outIndex + RGBA_CHANNELS.B] = rgb & 0xff;
  out[outIndex + RGBA_CHANNELS.A] = DEFAULT_ALPHA;
}

/**
 * Set pixel to black in RGBA array
 *
 * @param out - Output array to write RGBA values
 * @param outIndex - Starting index in output array
 */
export function setPixelBlack(out: Uint8ClampedArray, outIndex: number): void {
  out[outIndex + RGBA_CHANNELS.R] = 0;
  out[outIndex + RGBA_CHANNELS.G] = 0;
  out[outIndex + RGBA_CHANNELS.B] = 0;
  out[outIndex + RGBA_CHANNELS.A] = DEFAULT_ALPHA;
}

// Re-export constants for use in other modules
export { RGB_SHIFT, RGBA_CHANNELS, DEFAULT_ALPHA };
