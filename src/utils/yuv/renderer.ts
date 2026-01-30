/**
 * YUV to ImageData Converter and Canvas Renderer
 *
 * Main YUV to ImageData conversion function and canvas rendering.
 * Refactored from yuvRenderer.ts to use extracted utilities.
 */

import type { YUVFrame } from '../../types/yuv';
import { Colorspace } from '../../types/yuv';
import { YUVCache } from './cache';
import { ChromaStrategyFactory, getYIndex } from './chroma';
import { yuvToRgb, setPixelBlack, yuvToRgbaArray } from './conversion';

/**
 * Logger for YUV renderer
 */
const YUV_LOGGER = {
  error: (...args: unknown[]) => console.error('[yuvRenderer]', ...args),
  warn: (...args: unknown[]) => console.warn('[yuvRenderer]', ...args),
  debug: (...args: unknown[]) => console.debug('[yuvRenderer]', ...args),
};

/**
 * Convert YUV frame to RGB ImageData
 *
 * @param frame - YUV frame data
 * @param colorspace - Colorspace to use for conversion
 * @returns ImageData object ready for canvas
 */
export function yuvToImageData(frame: YUVFrame, colorspace: Colorspace = Colorspace.BT709): ImageData {
  // Check cache first
  const cached = YUVCache.get(frame, colorspace);
  if (cached) {
    return cached;
  }

  const { width, height, y, u, v, yStride, uStride } = frame;

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
        setPixelBlack(data, outIndex);
        outIndex += 4;
        continue;
      }

      const yVal = y[yIndex];
      const uVal = u[chromaIndex];
      const vVal = v[chromaIndex];

      yuvToRgbaArray(yVal, uVal, vVal, colorspace, data, outIndex);

      outIndex += 4;
    }
  }

  // Store in cache for future use
  YUVCache.set(frame, colorspace, imageData);

  return imageData;
}

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
