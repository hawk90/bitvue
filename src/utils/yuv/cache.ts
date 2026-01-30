/**
 * YUV Conversion Cache
 *
 * LRU cache for YUV to ImageData conversions to avoid redundant conversions.
 * Extracted from yuvRenderer.ts for better modularity.
 */

import { LRUCache } from '../lruCache';
import type { YUVFrame } from '../../types/yuv';
import { Colorspace } from '../../types/yuv';

/**
 * Logger for YUV cache
 */
const YUV_CACHE_LOGGER = {
  error: (...args: unknown[]) => console.error('[YUVCache]', ...args),
  warn: (...args: unknown[]) => console.warn('[YUVCache]', ...args),
  debug: (...args: unknown[]) => console.debug('[YUVCache]', ...args),
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
  hash = Math.imul(hash, 0x01000193) ^ key.uvLength;

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
      YUV_CACHE_LOGGER.debug('YUV cache hit:', key);
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
   */
  setLimits(maxSize: number, maxMemory: number): void {
    // Note: This would require extending LRUCache to support resizing
    YUV_CACHE_LOGGER.warn('Dynamic cache resizing not yet implemented');
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
  setLimits: (maxSize: number, maxMemory: number) => yuvCache.setLimits(maxSize, maxMemory),
  /** Get cached ImageData */
  get: (frame: YUVFrame, colorspace: Colorspace) => yuvCache.get(frame, colorspace),
  /** Store ImageData in cache */
  set: (frame: YUVFrame, colorspace: Colorspace, imageData: ImageData) => yuvCache.set(frame, colorspace, imageData),
};
