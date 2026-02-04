/**
 * Screenshot Capture Utilities
 *
 * Capture canvas or DOM elements as images
 */

import { createLogger } from './logger';

const logger = createLogger('screenshotCapture');

/**
 * Capture a canvas element as PNG data URL
 */
export function captureCanvas(canvas: HTMLCanvasElement): string {
  return canvas.toDataURL('image/png');
}

/**
 * Download a data URL as a file
 */
export function downloadDataUrl(dataUrl: string, filename: string): void {
  const link = document.createElement('a');
  link.href = dataUrl;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
}

/**
 * Capture and download current frame from canvas
 */
export function captureFrame(canvas: HTMLCanvasElement, frameIndex: number): void {
  const dataUrl = captureCanvas(canvas);
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
  const filename = `bitvue-frame-${frameIndex}-${timestamp}.png`;
  downloadDataUrl(dataUrl, filename);
}

/**
 * Copy image to clipboard
 */
export async function copyCanvasToClipboard(canvas: HTMLCanvasElement): Promise<boolean> {
  try {
    const blob = await new Promise<Blob>((resolve) => {
      canvas.toBlob((blob) => resolve(blob!), 'image/png');
    });

    await navigator.clipboard.write([
      new ClipboardItem({ 'image/png': blob })
    ]);

    return true;
  } catch (error) {
    logger.error('Failed to copy to clipboard:', error);
    return false;
  }
}

/**
 * Get formatted timestamp for filename
 */
export function getTimestampFilename(prefix: string, frameIndex: number): string {
  const now = new Date();
  const date = now.toISOString().split('T')[0]; // YYYY-MM-DD
  const time = now.toTimeString().split(' ')[0].replace(/:/g, '-'); // HH-MM-SS
  return `${prefix}-frame-${frameIndex}-${date}_${time}.png`;
}
