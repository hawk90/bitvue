/**
 * Chroma Index Strategies
 *
 * Strategy pattern for calculating chroma pixel indices
 * based on chroma subsampling type.
 * Extracted from yuvRenderer.ts for better modularity.
 */

import type { ChromaSubsampling, YUVFrame } from "../../types/yuv";

/**
 * Strategy interface for calculating chroma pixel indices
 */
export interface ChromaIndexStrategy {
  /** Get the index for chroma sample at given pixel coordinates */
  getIndex(x: number, y: number, stride: number): number;
  /** Get the subsampling type */
  readonly subsampling: ChromaSubsampling;
}

/**
 * YUV420 chroma index strategy (2:1 horizontal and vertical subsampling)
 */
class Chroma420Strategy implements ChromaIndexStrategy {
  readonly subsampling: ChromaSubsampling = "420";

  getIndex(x: number, y: number, stride: number): number {
    // Divide both coordinates by 2 for 2:1 subsampling
    return (y >> 1) * stride + (x >> 1);
  }
}

/**
 * YUV422 chroma index strategy (2:1 horizontal subsampling only)
 */
class Chroma422Strategy implements ChromaIndexStrategy {
  readonly subsampling: ChromaSubsampling = "422";

  getIndex(x: number, y: number, stride: number): number {
    // Divide only x coordinate by 2 for horizontal-only subsampling
    return y * stride + (x >> 1);
  }
}

/**
 * YUV444 chroma index strategy (no subsampling)
 */
class Chroma444Strategy implements ChromaIndexStrategy {
  readonly subsampling: ChromaSubsampling = "444";

  getIndex(x: number, y: number, stride: number): number {
    // No subsampling - direct indexing
    return y * stride + x;
  }
}

/**
 * Factory for creating chroma index strategies
 */
export const ChromaStrategyFactory = {
  /** Map of subsampling types to strategy instances */
  strategies: {
    "420": new Chroma420Strategy(),
    "422": new Chroma422Strategy(),
    "444": new Chroma444Strategy(),
  },

  /** Get a strategy for the given subsampling type */
  getStrategy(subsampling: ChromaSubsampling): ChromaIndexStrategy {
    const strategy = this.strategies[subsampling];
    if (!strategy) {
      console.error(
        "[ChromaStrategyFactory] Unknown subsampling type:",
        subsampling,
        ", defaulting to 420",
      );
      return this.strategies["420"];
    }
    console.debug(
      "[ChromaStrategyFactory] Returning strategy for subsampling:",
      subsampling,
    );
    return strategy;
  },

  /** Get a strategy from a YUVFrame */
  getStrategyForFrame(frame: YUVFrame): ChromaIndexStrategy {
    const subsampling = frame.chromaSubsampling || "420";
    console.debug("[ChromaStrategyFactory] getStrategyForFrame:", {
      chromaSubsampling: frame.chromaSubsampling,
      usingSubsampling: subsampling,
    });
    return this.getStrategy(subsampling);
  },
};

/**
 * Get pixel index in Y plane
 */
export function getYIndex(x: number, y: number, stride: number): number {
  return y * stride + x;
}
