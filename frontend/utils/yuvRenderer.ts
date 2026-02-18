/**
 * YUV to RGB Renderer
 *
 * Backwards compatibility re-exports from the modularized YUV utilities.
 *
 * The actual implementation has been split into:
 * - types/yuv.ts - Type definitions
 * - utils/yuv/conversion.ts - YUV to RGB conversion functions
 * - utils/yuv/chroma.ts - Chroma index strategies
 * - utils/yuv/parsing.ts - YUV buffer parsing
 * - utils/yuv/cache.ts - YUV conversion cache
 * - utils/yuv/renderer.ts - YUVRenderer class and main conversion functions
 *
 * For new code, prefer importing from utils/yuv:
 * ```ts
 * import { yuvToRgb, YUVRenderer, Colorspace } from './utils/yuv';
 * ```
 *
 * @deprecated Prefer importing from './utils/yuv' instead
 */

// Re-export everything from the new modular structure
export * from "./yuv";
