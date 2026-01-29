/**
 * YUV to RGB Renderer
 *
 * Handles conversion from YUV pixel data to RGB and rendering to canvas.
 * Supports various YUV formats (YUV420, YUV422, YUV444) and colorspaces.
 */

import { LRUCache } from './lruCache';

/**
 * Logger for YUV renderer
 */
const YUV_LOGGER = {
  error: (...args: unknown[]) => console.error('[yuvRenderer]', ...args),
  warn: (...args: unknown[]) => console.warn('[yuvRenderer]', ...args),
  debug: (...args: unknown[]) => console.debug('[yuvRenderer]', ...args),
};

// ============================================================================
// Constants
// ============================================================================

/** Chroma subsampling divisor for YUV420 (2:1 horizontal and vertical) */
const CHROMA_420_DIVISOR = 2;

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

/** Default maximum cache size (number of ImageData objects) */
const DEFAULT_YUV_CACHE_SIZE = 50;

/** Default maximum cache memory (100MB) */
const DEFAULT_YUV_CACHE_MEMORY = 100 * 1024 * 1024;

// ============================================================================
// YUV Frame Interface
// ============================================================================

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
  chromaSubsampling: '420' | '422' | '444';
}

// ============================================================================
// YUV Cache
// ============================================================================

/**
 * YUV conversion cache key
 */
interface YUVCacheKey {
  width: number;
  height: number;
  chromaSubsampling: string;
  colorspace: string;
  yLength: number;
  uvLength: number;
}

/**
 * Create a cache key from YUV frame data and colorspace
 *
 * Uses a numeric hash function instead of JSON.stringify for better performance.
 * The key is a 64-bit integer computed from the frame properties.
 */
function createYUVCacheKey(frame: YUVFrame, colorspace: Colorspace): string {
  const key: YUVCacheKey = {
    width: frame.width,
    height: frame.height,
    chromaSubsampling: frame.chromaSubsampling,
    colorspace,
    yLength: frame.y.length,
    uvLength: Math.min(frame.u.length, frame.v.length),
  };

  // Compute hash using a simple but effective algorithm
  // Based on FNV-1a hash but optimized for our use case
  let hash = 0x811c9dc5;

  // Hash the colorspace (string)
  for (let i = 0; i < key.colorspace.length; i++) {
    hash = Math.imul(hash, 0x01000193) ^ key.colorspace.charCodeAt(i);
  }

  // Hash the chroma subsampling (string)
  for (let i = 0; i < key.chromaSubsampling.length; i++) {
    hash = Math.imul(hash, 0x01000193) ^ key.chromaSubsampling.charCodeAt(i);
  }

  // Hash numeric values using bit shifting
  hash = Math.imul(hash, 0x01000193) ^ key.width;
  hash = Math.imul(hash, 0x01000193) ^ key.height;
  hash = Math.imul(hash, 0x01000193) ^ key.yLength;
  hash = Mathul(hash, 0x01000193) ^ key.uvLength;

  // Convert to hex string for use as cache key
  return hash.toString(16);
}

/**
 * Global YUV to ImageData conversion cache
 *
 * Caches converted ImageData objects to avoid redundant conversions
 * of the same frame with the same parameters.
 */
class YUVConversionCache {
  private cache: LRUCache<string, ImageData>;

  constructor() {
    this.cache = new LRUCache<string, ImageData>({
      maxSize: DEFAULT_YUV_CACHE_SIZE,
      maxMemory: DEFAULT_YUV_CACHE_MEMORY,
      sizeEstimator: (imageData: ImageData) => {
        // ImageData.data is a Uint8ClampedArray
        return imageData.data.byteLength;
      },
    });
  }

  /**
   * Get cached ImageData for the given frame and colorspace
   */
  get(frame: YUVFrame, colorspace: Colorspace): ImageData | null {
    const key = createYUVCacheKey(frame, colorspace);
    const cached = this.cache.get(key);
    if (cached) {
      YUV_LOGGER.debug('YUV cache hit:', key);
      return cached;
    }
    return null;
  }

  /**
   * Store ImageData in the cache
   */
  set(frame: YUVFrame, colorspace: Colorspace, imageData: ImageData): void {
    const key = createYUVCacheKey(frame, colorspace);
    this.cache.set(key, imageData);
  }

  /**
   * Clear all cached entries
   */
  clear(): void {
    this.cache.clear();
  }

  /**
   * Get cache statistics
   */
  getStats(): { size: number; memoryUsage: number } {
    return {
      size: this.cache.size,
      memoryUsage: this.cache.memoryUsage,
    };
  }
}

/** Global YUV conversion cache instance */
const yuvCache = new YUVConversionCache();

/**
 * Exported cache control functions
 */
export const YUVCache = {
  /** Clear the YUV conversion cache */
  clear: () => yuvCache.clear(),
  /** Get cache statistics */
  getStats: () => yuvCache.getStats(),
  /** Set cache size limits */
  setLimits: (maxSize: number, maxMemory: number) => {
    // Note: This would require extending LRUCache to support resizing
    YUV_LOGGER.warn('Dynamic cache resizing not yet implemented');
  },
};

// ============================================================================
// Colorspace Enum
// ============================================================================

/**
 * Colorspace conversion matrix
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
 * Colorspace conversion coefficients
 *
 * Contains the matrix coefficients for YUV to RGB conversion
 * for different colorspaces. Based on ITU-R recommendations.
 *
 * References:
 * - ITU-R BT.601-7 (SD video)
 * - ITU-R BT.709-6 (HD video)
 * - ITU-R BT.2020-2 (UHD video)
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
 * Colorspace conversion matrices
 *
 * Maps each colorspace to its YUV to RGB conversion coefficients.
 * The conversion formula is:
 * ```
 * R = Y + rv * (V - 128)
 * G = Y + gu * (U - 128) + gv * (V - 128)
 * B = Y + bu * (U - 128)
 * ```
 */
export const COLORSPACE_MATRICS: Readonly<Record<Colorspace, ColorspaceMatrix>> = {
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
};

/**
 * Get colorspace conversion matrix
 *
 * @param colorspace - The colorspace to get coefficients for
 * @returns The YUV to RGB conversion matrix
 */
export function getColorspaceMatrix(colorspace: Colorspace): ColorspaceMatrix {
  return COLORSPACE_MATRICS[colorspace];
}

// ============================================================================
// Chroma Index Strategy Pattern
// ============================================================================

/**
 * Chroma subsampling type
 */
type ChromaSubsampling = '420' | '422' | '444';

/**
 * Strategy interface for calculating chroma pixel indices
 */
interface ChromaIndexStrategy {
  /** Get the index for chroma sample at given pixel coordinates */
  getIndex(x: number, y: number, stride: number): number;
  /** Get the subsampling type */
  readonly subsampling: ChromaSubsampling;
}

/**
 * YUV420 chroma index strategy (2:1 horizontal and vertical subsampling)
 */
class Chroma420Strategy implements ChromaIndexStrategy {
  readonly subsampling: ChromaSubsampling = '420';

  getIndex(x: number, y: number, stride: number): number {
    // Divide both coordinates by 2 for 2:1 subsampling
    return (y >> 1) * stride + (x >> 1);
  }
}

/**
 * YUV422 chroma index strategy (2:1 horizontal subsampling only)
 */
class Chroma422Strategy implements ChromaIndexStrategy {
  readonly subsampling: ChromaSubsampling = '422';

  getIndex(x: number, y: number, stride: number): number {
    // Divide only x coordinate by 2 for horizontal-only subsampling
    return y * stride + (x >> 1);
  }
}

/**
 * YUV444 chroma index strategy (no subsampling)
 */
class Chroma444Strategy implements ChromaIndexStrategy {
  readonly subsampling: ChromaSubsampling = '444';

  getIndex(x: number, y: number, stride: number): number {
    // No subsampling - direct indexing
    return y * stride + x;
  }
}

/**
 * Factory for creating chroma index strategies
 */
const ChromaStrategyFactory = {
  /** Map of subsampling types to strategy instances */
  strategies: Record<ChromaSubsampling, ChromaIndexStrategy> = {
    '420': new Chroma420Strategy(),
    '422': new Chroma422Strategy(),
    '444': new Chroma444Strategy(),
  },

  /** Get a strategy for the given subsampling type */
  getStrategy(subsampling: ChromaSubsampling): ChromaIndexStrategy {
    return this.strategies[subsampling];
  },

  /** Get a strategy from a YUVFrame */
  getStrategyForFrame(frame: YUVFrame): ChromaIndexStrategy {
    return this.getStrategy(frame.chromaSubsampling);
  },
} as const;

// ============================================================================
// YUV to RGB Conversion
// ============================================================================

/**
 * Convert YUV to RGB for a single pixel
 *
 * @param y - Y component (0-255)
 * @param u - U component (0-255)
 * @param v - V component (0-255)
 * @param colorspace - Colorspace to use for conversion
 * @returns RGB value as 24-bit integer (0xRRGGBB)
 */
export function yuvToRgb(y: number, u: number, v: number, colorspace: Colorspace = Colorspace.BT709): number {
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
  return (rClamped << RGB_SHIFT.R) | (gClamped << RGB_SHIFT.G) | (bClamped << RGB_SHIFT.B);
}

/**
 * Get pixel index in Y plane
 */
function getYIndex(x: number, y: number, stride: number): number {
  return y * stride + x;
}

// ============================================================================
// YUV to ImageData Conversion
// ============================================================================

/**
 * Convert YUV frame to RGB ImageData
 *
 * @param frame - YUV frame data
 * @param colorspace - Colorspace to use for conversion
 * @returns ImageData object ready for canvas
 */
export function yuvToImageData(frame: YUVFrame, colorspace: Colorspace = Colorspace.BT709): ImageData {
  // Check cache first
  const cached = yuvCache.get(frame, colorspace);
  if (cached) {
    return cached;
  }

  const { width, height, y, u, v, yStride, uStride, vStride } = frame;

  // Create ImageData buffer
  const imageData = new ImageData(width, height);
  const data = imageData.data;

  // Get chroma index strategy for this frame's subsampling
  const chromaStrategy = ChromaStrategyFactory.getStrategyForFrame(frame);

  // Convert each pixel
  let outIndex = 0;
  for (let py = 0; py < height; py++) {
    for (let px = 0; px < width; px++) {
      const yIndex = getYIndex(px, py, yStride);
      const chromaIndex = chromaStrategy.getIndex(px, py, uStride);

      // Validate indices before accessing arrays
      if (yIndex >= y.length || chromaIndex >= u.length || chromaIndex >= v.length) {
        YUV_LOGGER.warn(`Index out of bounds: yIndex=${yIndex}, chromaIndex=${chromaIndex}`);
        // Set to black and continue
        data[outIndex + RGBA_CHANNELS.R] = 0;
        data[outIndex + RGBA_CHANNELS.G] = 0;
        data[outIndex + RGBA_CHANNELS.B] = 0;
        data[outIndex + RGBA_CHANNELS.A] = DEFAULT_ALPHA;
        outIndex += 4;
        continue;
      }

      const yVal = y[yIndex];
      const uVal = u[chromaIndex];
      const vVal = v[chromaIndex];

      const rgb = yuvToRgb(yVal, uVal, vVal, colorspace);

      // Write RGBA (ImageData stores pixels as RGBA)
      data[outIndex + RGBA_CHANNELS.R] = (rgb >> RGB_SHIFT.R) & 0xff;
      data[outIndex + RGBA_CHANNELS.G] = (rgb >> RGB_SHIFT.G) & 0xff;
      data[outIndex + RGBA_CHANNELS.B] = rgb & 0xff;
      data[outIndex + RGBA_CHANNELS.A] = DEFAULT_ALPHA;

      outIndex += 4;
    }
  }

  // Store in cache for future use
  yuvCache.set(frame, colorspace, imageData);

  return imageData;
}

// ============================================================================
// Canvas Rendering
// ============================================================================

/**
 * Render YUV frame to canvas
 *
 * @param canvas - Canvas element to render to
 * @param frame - YUV frame data
 * @param colorspace - Colorspace to use for conversion
 */
export function renderYUVToCanvas(
  canvas: HTMLCanvasElement,
  frame: YUVFrame,
  colorspace: Colorspace = Colorspace.BT709
): void {
  const ctx = canvas.getContext('2d');
  if (!ctx) {
    YUV_LOGGER.error('Failed to get 2D context from canvas. Canvas may not be properly initialized.');
    return;
  }

  // Ensure canvas size matches frame
  if (canvas.width !== frame.width || canvas.height !== frame.height) {
    canvas.width = frame.width;
    canvas.height = frame.height;
  }

  // Convert YUV to ImageData
  const imageData = yuvToImageData(frame, colorspace);

  // Render to canvas
  ctx.putImageData(imageData, 0, 0);
}

// ============================================================================
// YUV Buffer Parsing
// ============================================================================

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
  format: 'I420' | 'YV12' | 'NV12' | 'YUY2' | 'UYVY' = 'I420'
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
  const chromaSize = (width / CHROMA_420_DIVISOR) * (height / CHROMA_420_DIVISOR);

  if (format === 'I420' || format === 'YV12') {
    // Planar YUV420
    yStride = width;
    uStride = width / CHROMA_420_DIVISOR;
    vStride = width / CHROMA_420_DIVISOR;
    chromaSubsampling = '420';

    y = data.subarray(0, ySize);
    const uStart = format === 'I420' ? ySize : ySize + chromaSize;
    const vStart = format === 'I420' ? ySize + chromaSize : ySize;
    u = data.subarray(uStart, uStart + chromaSize);
    v = data.subarray(vStart, vStart + chromaSize);
  } else if (format === 'NV12') {
    // Semi-planar YUV420 (UV interleaved)
    yStride = width;
    uStride = width;
    vStride = width;
    chromaSubsampling = '420';

    y = data.subarray(0, ySize);
    const uvData = data.subarray(ySize, ySize + chromaSize * 2);

    // De-interleave UV
    u = new Uint8Array(chromaSize);
    v = new Uint8Array(chromaSize);
    for (let i = 0; i < chromaSize; i++) {
      u[i] = uvData[i * 2];
      v[i] = uvData[i * 2 + 1];
    }
  } else if (format === 'YUY2' || format === 'UYVY') {
    // Packed YUV422
    chromaSubsampling = '422';
    const frameSize = width * height * 2;

    y = new Uint8Array(ySize);
    u = new Uint8Array(ySize);
    v = new Uint8Array(ySize);
    yStride = width;
    uStride = width;
    vStride = width;

    let inIndex = format === 'YUY2' ? 0 : 1;
    const y0Offset = format === 'YUY2' ? 0 : 1;
    const y1Offset = format === 'YUY2' ? 2 : 3;
    const uOffset = format === 'YUY2' ? 1 : 0;
    const vOffset = format === 'YUY2' ? 3 : 2;

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

// ============================================================================
// Frame Creation
// ============================================================================

/**
 * Create a blank black YUV frame
 */
export function createBlackYUVFrame(width: number, height: number): YUVFrame {
  const ySize = width * height;
  const chromaSize = (width / CHROMA_420_DIVISOR) * (height / CHROMA_420_DIVISOR);

  return {
    y: new Uint8Array(ySize), // All zeros = black
    u: new Uint8Array(chromaSize).fill(128), // Neutral chroma
    v: new Uint8Array(chromaSize).fill(128), // Neutral chroma
    width,
    height,
    yStride: width,
    uStride: width / CHROMA_420_DIVISOR,
    vStride: width / CHROMA_420_DIVISOR,
    chromaSubsampling: '420',
  };
}

// ============================================================================
// YUV Renderer Class
// ============================================================================

/**
 * YUV renderer class for efficient rendering
 */
export class YUVRenderer {
  private canvas: HTMLCanvasElement | null = null;
  private ctx: CanvasRenderingContext2D | null = null;
  private imageData: ImageData | null = null;
  private width = 0;
  private height = 0;

  constructor(canvas?: HTMLCanvasElement) {
    if (canvas) {
      this.attachCanvas(canvas);
    }
  }

  /**
   * Attach a canvas element for rendering
   */
  attachCanvas(canvas: HTMLCanvasElement): void {
    this.canvas = canvas;
    this.ctx = canvas.getContext('2d');
  }

  /**
   * Resize renderer for given frame dimensions
   */
  resize(width: number, height: number): void {
    if (this.width === width && this.height === height) {
      return;
    }

    this.width = width;
    this.height = height;

    if (this.canvas) {
      this.canvas.width = width;
      this.canvas.height = height;
    }

    this.imageData = new ImageData(width, height);
  }

  /**
   * Render YUV frame efficiently
   */
  render(frame: YUVFrame, colorspace: Colorspace = Colorspace.BT709): void {
    if (!this.ctx || !this.imageData) {
      return;
    }

    if (frame.width !== this.width || frame.height !== this.height) {
      this.resize(frame.width, frame.height);
    }

    const converted = yuvToImageData(frame, colorspace);
    this.imageData.data.set(converted.data);
    this.ctx.putImageData(this.imageData, 0, 0);
  }

  /**
   * Clear canvas to black
   */
  clear(): void {
    if (!this.ctx || !this.canvas) return;
    this.ctx.fillStyle = '#000';
    this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);
  }
}

// ============================================================================
// Exports
// ============================================================================
//
// All exports are named exports for better tree-shaking.
// The default export has been removed to enable more aggressive
// dead code elimination in bundlers.

// Note: No default export - use named imports instead
// import { yuvToRgb, YUVRenderer, Colorspace } from './yuvRenderer';
