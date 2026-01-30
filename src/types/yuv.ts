/**
 * YUV Types
 *
 * Type definitions for YUV video processing.
 * Extracted from yuvRenderer.ts for better modularity.
 */

/**
 * Chroma subsampling type
 */
export type ChromaSubsampling = '420' | '422' | '444';

/**
 * Colorspace enum for YUV to RGB conversion
 */
export enum Colorspace {
  /** ITU-R BT.601 (SD video) */
  BT601 = 'BT601',
  /** ITU-R BT.709 (HD video) */
  BT709 = 'BT709',
  /** ITU-R BT.2020 (UHD video) */
  BT2020 = 'BT2020',
}

/**
 * Colorspace conversion matrix coefficients
 *
 * Contains the matrix coefficients for YUV to RGB conversion
 * for different colorspaces. Based on ITU-R recommendations.
 *
 * The conversion formula is:
 * ```
 * R = Y + rv * (V - 128)
 * G = Y + gu * (U - 128) + gv * (V - 128)
 * B = Y + bu * (U - 128)
 * ```
 */
export interface ColorspaceMatrix {
  /** Red channel coefficient for V component */
  rv: number;
  /** Green channel coefficient for U component */
  gu: number;
  /** Green channel coefficient for V component */
  gv: number;
  /** Blue channel coefficient for U component */
  bu: number;
}

/**
 * YUV frame data interface
 */
export interface YUVFrame {
  /** Y component data (luminance) */
  y: Uint8Array;
  /** U component data (chrominance blue) */
  u: Uint8Array;
  /** V component data (chrominance red) */
  v: Uint8Array;
  /** Frame width in pixels */
  width: number;
  /** Frame height in pixels */
  height: number;
  /** Y stride (bytes per row in Y plane) */
  yStride: number;
  /** U stride (bytes per row in U plane) */
  uStride: number;
  /** V stride (bytes per row in V plane) */
  vStride: number;
  /** Chroma subsampling: '420', '422', or '444' */
  chromaSubsampling: ChromaSubsampling;
}

/**
 * YUV format for buffer parsing
 */
export type YUVFormat = 'I420' | 'YV12' | 'NV12' | 'YUY2' | 'UYVY';

/**
 * Colorspace conversion matrices
 *
 * Maps each colorspace to its YUV to RGB conversion coefficients.
 */
export const COLORSPACE_MATRICES: Readonly<Record<Colorspace, ColorspaceMatrix>> = {
  [Colorspace.BT601]: {
    // ITU-R BT.601 coefficients (SD video)
    rv: 1.402,
    gu: -0.344136,
    gv: -0.714136,
    bu: 1.772,
  },
  [Colorspace.BT709]: {
    // ITU-R BT.709 coefficients (HD video)
    rv: 1.5748,
    gu: -0.1873,
    gv: -0.4681,
    bu: 1.8556,
  },
  [Colorspace.BT2020]: {
    // ITU-R BT.2020 coefficients (UHD video)
    rv: 1.4746,
    gu: -0.164553,
    gv: -0.571353,
    bu: 1.8814,
  },
} as const;
