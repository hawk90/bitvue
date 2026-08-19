/**
 * YUV Conversion Cache
 *
 * LRU cache for YUV to ImageData conversions to avoid redundant conversions.
 * Extracted from yuvRenderer.ts for better modularity.
 */

import { LRUCache } from "../lruCache";
import type { YUVFrame } from "../../types/yuv";
import { Colorspace } from "../../types/yuv";

/**
 * Logger for YUV cache
 */
const YUV_CACHE_LOGGER = {
  error: (...args: unknown[]) => console.error("[YUVCache]", ...args),
  warn: (...args: unknown[]) => console.warn("[YUVCache]", ...args),
  debug: (...args: unknown[]) => console.debug("[YUVCache]", ...args),
};

/** Default maximum cache size (number of ImageData objects) */
const DEFAULT_YUV_CACHE_SIZE = 50;

/** Default maximum cache memory (100MB) */
const DEFAULT_YUV_CACHE_MEMORY = 100 * 1024 * 1024;

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

/** Bytes sampled (evenly strided) from the Y plane to make the cache key content-sensitive --
 *  `hashPlaneSample`'s doc. U/V get a quarter as many samples (they're smaller planes and less
 *  visually significant for collision purposes, luma dominates perceived difference). */
const Y_CONTENT_SAMPLE_COUNT = 256;
const CHROMA_CONTENT_SAMPLE_COUNT = 64;

/**
 * Folds an evenly-strided sample of `data`'s bytes into `hash` (same FNV-1a-style fold as the
 * rest of `createYUVCacheKey`). Sampling instead of hashing every byte keeps a cache HIT cheap
 * (bounded cost, not O(frame size)) while still being sensitive to real content differences --
 * two genuinely different video frames essentially never produce identical strided samples,
 * unlike the two matching-dimension/format frames this is built to distinguish.
 */
function hashPlaneSample(
  data: Uint8Array,
  sampleCount: number,
  hash: number,
): number {
  if (data.length === 0) {
    return hash;
  }
  const stride = Math.max(1, Math.floor(data.length / sampleCount));
  for (let i = 0; i < data.length; i += stride) {
    hash = Math.imul(hash, 0x01000193) ^ data[i];
  }
  return hash;
}

/**
 * Create a cache key from YUV frame data and colorspace
 *
 * Uses a numeric hash function instead of JSON.stringify for better performance.
 * The key is a 64-bit integer computed from the frame properties.
 *
 * **Must include real pixel content, not just dimensions/format** -- a real cross-frame
 * collision bug (found 2026-08-19 while investigating a black-screen report in the new dual-
 * stream compare view) had this key derived ONLY from width/height/chromaSubsampling/colorspace/
 * plane lengths. Since this cache is a single global singleton (`yuvCache` below) shared by
 * every `YUVRenderer` instance in the app, ANY two frames sharing those properties -- which is
 * the *common* case, not an edge case: every frame within one video shares them, and two
 * different streams at the same resolution/format (exactly what dual-stream compare usually is)
 * do too -- silently rendered whichever content got cached *first*, regardless of which frame
 * was actually requested. Confirmed via a direct unit test (two 4x4 frames, same dims/format,
 * different fill values -- the second call returned the first's cached pixels verbatim) before
 * this fix; see `tests/utils/yuv/cache.test.ts`'s regression test for the same scenario.
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
  hash = Math.imul(hash, 0x01000193) ^ key.uvLength;

  // Content sample -- the actual fix, see this function's own doc.
  hash = hashPlaneSample(frame.y, Y_CONTENT_SAMPLE_COUNT, hash);
  hash = hashPlaneSample(frame.u, CHROMA_CONTENT_SAMPLE_COUNT, hash);
  hash = hashPlaneSample(frame.v, CHROMA_CONTENT_SAMPLE_COUNT, hash);

  // Use composite key to prevent hash collisions between frames with different dimensions
  // but an identical hash (e.g. two frames where width/height/yLength happen to produce
  // the same 32-bit FNV output).
  return `${hash.toString(16)}_${key.width}_${key.height}_${key.yLength}_${key.colorspace}`;
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
      YUV_CACHE_LOGGER.debug("YUV cache hit:", key);
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

  /**
   * Set cache size limits
   * @internal
   */
  setLimits(_maxSize: number, _maxMemory: number): void {
    // Note: This would require extending LRUCache to support resizing
    YUV_CACHE_LOGGER.warn("Dynamic cache resizing not yet implemented");
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
  /**
   * Set cache size limits
   * @internal — dynamic resizing not yet implemented
   */
  setLimits: (maxSize: number, maxMemory: number) =>
    yuvCache.setLimits(maxSize, maxMemory),
  /** Get cached ImageData */
  get: (frame: YUVFrame, colorspace: Colorspace) =>
    yuvCache.get(frame, colorspace),
  /** Store ImageData in cache */
  set: (frame: YUVFrame, colorspace: Colorspace, imageData: ImageData) =>
    yuvCache.set(frame, colorspace, imageData),
};
