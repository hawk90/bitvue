/**
 * YUV to RGB Renderer
 *
 * Handles conversion from YUV pixel data to RGB and rendering to canvas.
 * Supports various YUV formats (YUV420, YUV422, YUV444) and colorspaces.
 */

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

  let r: number, g: number, b: number;

  if (colorspace === Colorspace.BT601) {
    // ITU-R BT.601 coefficients (SD video)
    r = y + 1.402 * v0;
    g = y - 0.344136 * u0 - 0.714136 * v0;
    b = y + 1.772 * u0;
  } else if (colorspace === Colorspace.BT2020) {
    // ITU-R BT.2020 coefficients (UHD video)
    r = y + 1.4746 * v0;
    g = y - 0.164553 * u0 - 0.571353 * v0;
    b = y + 1.8814 * u0;
  } else {
    // ITU-R BT.709 coefficients (HD video, default)
    r = y + 1.5748 * v0;
    g = y - 0.1873 * u0 - 0.4681 * v0;
    b = y + 1.8556 * u0;
  }

  // Clamp to [0, 255] and convert to integer
  const rClamped = Math.max(0, Math.min(255, Math.round(r)));
  const gClamped = Math.max(0, Math.min(255, Math.round(g)));
  const bClamped = Math.max(0, Math.min(255, Math.round(b)));

  // Pack as RGB (0xRRGGBB)
  return (rClamped << 16) | (gClamped << 8) | bClamped;
}

/**
 * Get pixel index in Y plane
 */
function getYIndex(x: number, y: number, stride: number): number {
  return y * stride + x;
}

/**
 * Get pixel index in U/V planes for 420 subsampling
 */
function getChromaIndex420(x: number, y: number, stride: number): number {
  return (y >> 1) * stride + (x >> 1);
}

/**
 * Get pixel index in U/V planes for 422 subsampling
 */
function getChromaIndex422(x: number, y: number, stride: number): number {
  return y * stride + (x >> 1);
}

/**
 * Get pixel index in U/V planes for 444 (no subsampling)
 */
function getChromaIndex444(x: number, y: number, stride: number): number {
  return y * stride + x;
}

/**
 * Convert YUV frame to RGB ImageData
 *
 * @param frame - YUV frame data
 * @param colorspace - Colorspace to use for conversion
 * @returns ImageData object ready for canvas
 */
export function yuvToImageData(frame: YUVFrame, colorspace: Colorspace = Colorspace.BT709): ImageData {
  const { width, height, y, u, v, yStride, uStride, vStride, chromaSubsampling } = frame;

  // Create ImageData buffer
  const imageData = new ImageData(width, height);
  const data = imageData.data;

  // Select chroma index function based on subsampling
  const getChromaIndex = chromaSubsampling === '420'
    ? getChromaIndex420
    : chromaSubsampling === '422'
      ? getChromaIndex422
      : getChromaIndex444;

  // Convert each pixel
  let outIndex = 0;
  for (let py = 0; py < height; py++) {
    for (let px = 0; px < width; px++) {
      const yIndex = getYIndex(px, py, yStride);
      const chromaIndex = getChromaIndex(px, py, uStride);

      const yVal = y[yIndex];
      const uVal = u[chromaIndex];
      const vVal = v[chromaIndex];

      const rgb = yuvToRgb(yVal, uVal, vVal, colorspace);

      // Write RGBA (ImageData stores pixels as RGBA)
      data[outIndex] = (rgb >> 16) & 0xff;     // R
      data[outIndex + 1] = (rgb >> 8) & 0xff;  // G
      data[outIndex + 2] = rgb & 0xff;          // B
      data[outIndex + 3] = 255;                // A (opaque)

      outIndex += 4;
    }
  }

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
  if (!ctx) return;

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
  let chromaSubsampling: '420' | '422' | '444';

  const ySize = width * height;
  const chromaSize = (width / 2) * (height / 2); // For 420

  if (format === 'I420' || format === 'YV12') {
    // Planar YUV420
    yStride = width;
    uStride = width / 2;
    vStride = width / 2;
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
    const frameSize = width * height * 2; // 16 bits per pixel

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
        // Next pixel shares UV
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

/**
 * Create a blank black YUV frame
 */
export function createBlackYUVFrame(width: number, height: number): YUVFrame {
  const ySize = width * height;
  const chromaSize = (width / 2) * (height / 2);

  return {
    y: new Uint8Array(ySize),       // All zeros = black
    u: new Uint8Array(chromaSize).fill(128), // Neutral chroma
    v: new Uint8Array(chromaSize).fill(128), // Neutral chroma
    width,
    height,
    yStride: width,
    uStride: width / 2,
    vStride: width / 2,
    chromaSubsampling: '420',
  };
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
      return; // Already sized correctly
    }

    this.width = width;
    this.height = height;

    if (this.canvas) {
      this.canvas.width = width;
      this.canvas.height = height;
    }

    // Create ImageData buffer
    this.imageData = new ImageData(width, height);
  }

  /**
   * Render YUV frame efficiently
   */
  render(frame: YUVFrame, colorspace: Colorspace = Colorspace.BT709): void {
    if (!this.ctx || !this.imageData) {
      return;
    }

    // Resize if needed
    if (frame.width !== this.width || frame.height !== this.height) {
      this.resize(frame.width, frame.height);
    }

    // Convert and render
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

export default {
  yuvToRgb,
  yuvToImageData,
  renderYUVToCanvas,
  parseYUVFromBuffer,
  createBlackYUVFrame,
  YUVRenderer,
  Colorspace,
};
