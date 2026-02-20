/**
 * YUV Buffer Parsing Utilities
 *
 * Functions for parsing YUV data from ArrayBuffers in various formats.
 * Extracted from yuvRenderer.ts for better modularity.
 */

import type { YUVFrame, YUVFormat, ChromaSubsampling } from "../../types/yuv";

/** Chroma subsampling divisor for YUV420 (2:1 horizontal and vertical) */
const CHROMA_420_DIVISOR = 2;

/**
 * Parse YUV frame from ArrayBuffer
 *
 * Common format: Y plane followed by U plane followed by V plane
 * Supports planar YUV formats (I420, YV12, NV12, etc.)
 *
 * @param buffer - ArrayBuffer containing YUV data
 * @param width - Frame width in pixels
 * @param height - Frame height in pixels
 * @param format - YUV format ('I420', 'YV12', 'NV12', 'YUY2', 'UYVY')
 * @returns YUVFrame object
 */
export function parseYUVFromBuffer(
  buffer: ArrayBuffer,
  width: number,
  height: number,
  format: YUVFormat = "I420",
): YUVFrame {
  const data = new Uint8Array(buffer);

  let y: Uint8Array;
  let u: Uint8Array;
  let v: Uint8Array;
  let yStride: number;
  let uStride: number;
  let vStride: number;
  let chromaSubsampling: ChromaSubsampling;

  const ySize = width * height;
  const chromaSize =
    (width / CHROMA_420_DIVISOR) * (height / CHROMA_420_DIVISOR);

  if (format === "I420" || format === "YV12") {
    // Planar YUV420
    ({ y, u, v, yStride, uStride, vStride, chromaSubsampling } =
      parsePlanarYUV420(data, width, height, ySize, chromaSize, format));
  } else if (format === "NV12") {
    // Semi-planar YUV420 (UV interleaved)
    ({ y, u, v, yStride, uStride, vStride, chromaSubsampling } =
      parseSemiPlanarYUV420(data, width, height, ySize, chromaSize));
  } else if (format === "YUY2" || format === "UYVY") {
    // Packed YUV422
    ({ y, u, v, yStride, uStride, vStride, chromaSubsampling } =
      parsePackedYUV422(data, width, height, ySize, format));
  } else {
    throw new Error(`Unsupported YUV format: ${format}`);
  }

  return {
    y,
    u,
    v,
    width,
    height,
    yStride,
    uStride,
    vStride,
    chromaSubsampling,
  };
}

/**
 * Parse planar YUV420 format (I420 or YV12)
 */
function parsePlanarYUV420(
  data: Uint8Array,
  width: number,
  height: number,
  ySize: number,
  chromaSize: number,
  format: "I420" | "YV12",
): {
  y: Uint8Array;
  u: Uint8Array;
  v: Uint8Array;
  yStride: number;
  uStride: number;
  vStride: number;
  chromaSubsampling: ChromaSubsampling;
} {
  const yStride = width;
  const uStride = width / CHROMA_420_DIVISOR;
  const vStride = width / CHROMA_420_DIVISOR;
  const chromaSubsampling: ChromaSubsampling = "420";

  const y = data.subarray(0, ySize);
  const uStart = format === "I420" ? ySize : ySize + chromaSize;
  const vStart = format === "I420" ? ySize + chromaSize : ySize;
  const u = data.subarray(uStart, uStart + chromaSize);
  const v = data.subarray(vStart, vStart + chromaSize);

  return { y, u, v, yStride, uStride, vStride, chromaSubsampling };
}

/**
 * Parse semi-planar YUV420 format (NV12)
 */
function parseSemiPlanarYUV420(
  data: Uint8Array,
  width: number,
  height: number,
  ySize: number,
  chromaSize: number,
): {
  y: Uint8Array;
  u: Uint8Array;
  v: Uint8Array;
  yStride: number;
  uStride: number;
  vStride: number;
  chromaSubsampling: ChromaSubsampling;
} {
  const yStride = width;
  const uStride = width;
  const vStride = width;
  const chromaSubsampling: ChromaSubsampling = "420";

  const y = data.subarray(0, ySize);
  const uvData = data.subarray(ySize, ySize + chromaSize * 2);

  // De-interleave UV
  const u = new Uint8Array(chromaSize);
  const v = new Uint8Array(chromaSize);
  for (let i = 0; i < chromaSize; i++) {
    u[i] = uvData[i * 2];
    v[i] = uvData[i * 2 + 1];
  }

  return { y, u, v, yStride, uStride, vStride, chromaSubsampling };
}

/**
 * Parse packed YUV422 format (YUY2 or UYVY)
 */
function parsePackedYUV422(
  data: Uint8Array,
  width: number,
  height: number,
  ySize: number,
  format: "YUY2" | "UYVY",
): {
  y: Uint8Array;
  u: Uint8Array;
  v: Uint8Array;
  yStride: number;
  uStride: number;
  vStride: number;
  chromaSubsampling: ChromaSubsampling;
} {
  const chromaSubsampling: ChromaSubsampling = "422";
  const yStride = width;
  const uStride = width;
  const vStride = width;

  const y = new Uint8Array(ySize);
  const u = new Uint8Array(ySize);
  const v = new Uint8Array(ySize);

  const y0Offset = format === "YUY2" ? 0 : 1;
  const y1Offset = format === "YUY2" ? 2 : 3;
  const uOffset = format === "YUY2" ? 1 : 0;
  const vOffset = format === "YUY2" ? 3 : 2;

  for (let i = 0; i < ySize; i++) {
    const pixel = i * 2;
    if (pixel < data.length) {
      y[i] = data[pixel + y0Offset];
      if (i + 1 < ySize) {
        y[i + 1] = data[pixel + y1Offset];
      }
      u[i] = data[pixel + uOffset];
      v[i] = data[pixel + vOffset];
    }
  }

  return { y, u, v, yStride, uStride, vStride, chromaSubsampling };
}

/**
 * Create a blank black YUV frame
 */
export function createBlackYUVFrame(width: number, height: number): YUVFrame {
  const ySize = width * height;
  const chromaSize =
    (width / CHROMA_420_DIVISOR) * (height / CHROMA_420_DIVISOR);

  return {
    y: new Uint8Array(ySize), // All zeros = black
    u: new Uint8Array(chromaSize).fill(128), // Neutral chroma
    v: new Uint8Array(chromaSize).fill(128), // Neutral chroma
    width,
    height,
    yStride: width,
    uStride: width / CHROMA_420_DIVISOR,
    vStride: width / CHROMA_420_DIVISOR,
    chromaSubsampling: "420",
  };
}
