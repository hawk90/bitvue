/**
 * Progressive File Loader - v0.8.x Performance Optimization
 *
 * Loads large video files in chunks to avoid blocking the UI.
 * Implements progressive rendering and adaptive chunk sizing.
 */

import { invoke } from "@tauri-apps/api/core";
import { createLogger } from "./logger";
import { processThumbnailResults } from "./thumbnailUtils";
import type { FrameInfo, StreamInfo, ThumbnailResult } from "../types/video";

const logger = createLogger("progressiveLoader");

export interface LoadProgress {
  loadedFrames: number;
  totalFrames: number;
  loadedBytes: number;
  totalBytes: number;
  percentage: number;
  stage: "parsing" | "indexing" | "thumbnails" | "complete";
}

export type ProgressCallback = (progress: LoadProgress) => void;

export interface ProgressiveLoadOptions {
  /** Chunk size for frame loading (number of frames per chunk) */
  chunkSize?: number;
  /** Delay between chunks (ms) to allow UI updates */
  chunkDelay?: number;
  /** Progress callback */
  onProgress?: ProgressCallback;
  /** Enable adaptive chunk sizing based on file size */
  adaptiveChunking?: boolean;
}

/**
 * Default chunk sizes based on file size
 */
const CHUNK_SIZE_MAP: Record<string, number> = {
  small: 5000, // < 100MB files
  medium: 2000, // 100MB - 1GB files
  large: 500, // 1GB - 10GB files
  xlarge: 100, // > 10GB files
};

/**
 * Get file size category
 */
function getFileSizeCategory(bytes: number): string {
  if (bytes < 100 * 1024 * 1024) return "small";
  if (bytes < 1024 * 1024 * 1024) return "medium";
  if (bytes < 10 * 1024 * 1024 * 1024) return "large";
  return "xlarge";
}

/**
 * Progressive file loader for large video files
 */
export class ProgressiveFileLoader {
  private abortController: AbortController | null = null;
  private currentChunk = 0;
  private chunkSize: number;
  private chunkDelay: number;
  private adaptiveChunking: boolean;

  constructor(options: ProgressiveLoadOptions = {}) {
    this.chunkSize = options.chunkSize ?? 2000;
    this.chunkDelay = options.chunkDelay ?? 10;
    this.adaptiveChunking = options.adaptiveChunking ?? true;
  }

  /**
   * Load frames progressively
   */
  async loadFrames(
    filePath: string,
    options?: ProgressiveLoadOptions,
  ): Promise<FrameInfo[]> {
    this.abortController = new AbortController();
    const signal = this.abortController.signal;
    const onProgress = options?.onProgress;

    try {
      // First, get file info to determine total size
      const fileInfo = await invoke<StreamInfo>("get_stream_info", {
        path: filePath,
      });
      const totalFrames = fileInfo.frameCount || 0;
      const totalBytes = fileInfo.fileSize || 0;

      // Determine optimal chunk size
      let chunkSize = this.chunkSize;
      if (this.adaptiveChunking) {
        const category = getFileSizeCategory(totalBytes);
        chunkSize = CHUNK_SIZE_MAP[category];
        logger.info(
          `File size category: ${category}, chunk size: ${chunkSize}`,
        );
      }

      // Report initial progress
      onProgress?.({
        loadedFrames: 0,
        totalFrames,
        loadedBytes: 0,
        totalBytes,
        percentage: 0,
        stage: "parsing",
      });

      // Load frames in chunks
      const frames: FrameInfo[] = [];
      let loadedFrames = 0;

      while (loadedFrames < totalFrames) {
        if (signal.aborted) {
          throw new Error("Load cancelled");
        }

        const remainingFrames = totalFrames - loadedFrames;
        const framesToLoad = Math.min(chunkSize, remainingFrames);

        // Load chunk
        const chunkStart = loadedFrames;
        const chunkEnd = loadedFrames + framesToLoad;

        const chunkFrames = await invoke<FrameInfo[]>("get_frames", {
          path: filePath,
          startIndex: chunkStart,
          endIndex: chunkEnd,
        });

        // Check for cancellation after async operation
        if (signal.aborted) {
          throw new Error("Load cancelled");
        }

        frames.push(...chunkFrames);
        loadedFrames += chunkFrames.length;

        // Report progress
        onProgress?.({
          loadedFrames,
          totalFrames,
          loadedBytes: Math.round((loadedFrames / totalFrames) * totalBytes),
          totalBytes,
          percentage: (loadedFrames / totalFrames) * 100,
          stage: "parsing",
        });

        // Yield to UI
        if (loadedFrames < totalFrames) {
          await this.yield();
        }
      }

      return frames;
    } catch (error) {
      if (signal.aborted) {
        logger.info("Frame loading cancelled");
        throw new Error("Load cancelled");
      }
      logger.error("Failed to load frames:", error);
      throw error;
    }
  }

  /**
   * Load thumbnails progressively
   */
  async loadThumbnails(
    filePath: string,
    frameIndices: number[],
    options?: ProgressiveLoadOptions,
  ): Promise<Map<number, string>> {
    this.abortController = new AbortController();
    const signal = this.abortController.signal;
    const onProgress = options?.onProgress;

    const thumbnails = new Map<number, string>();
    const totalFrames = frameIndices.length;
    let loadedFrames = 0;

    // Determine chunk size
    let chunkSize = this.chunkSize;
    if (this.adaptiveChunking) {
      // Smaller chunks for thumbnails (more expensive)
      chunkSize = Math.min(chunkSize, 100);
    }

    for (let i = 0; i < frameIndices.length; i += chunkSize) {
      if (signal.aborted) {
        throw new Error("Load cancelled");
      }

      const chunk = frameIndices.slice(i, i + chunkSize);

      const results = await invoke<ThumbnailResult[]>("get_thumbnails", {
        frameIndices: chunk,
      });

      // Process thumbnail results using shared utility
      const processed = processThumbnailResults(results);
      processed.forEach((dataUrl, frameIndex) => {
        thumbnails.set(frameIndex, dataUrl);
      });

      // Count successful loads
      loadedFrames += results.filter((r) => r.success).length;

      // Report progress
      onProgress?.({
        loadedFrames,
        totalFrames,
        loadedBytes: loadedFrames * 10_000, // Estimate
        totalBytes: totalFrames * 10_000,
        percentage: (loadedFrames / totalFrames) * 100,
        stage: "thumbnails",
      });

      // Yield to UI
      if (i + chunkSize < frameIndices.length) {
        await this.yield();
      }
    }

    return thumbnails;
  }

  /**
   * Cancel ongoing load
   */
  cancel(): void {
    if (this.abortController) {
      this.abortController.abort();
      this.abortController = null;
    }
  }

  /**
   * Yield to UI thread
   */
  private async yield(): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, this.chunkDelay));
  }

  /**
   * Check if currently loading
   */
  isLoading(): boolean {
    return this.abortController !== null;
  }
}

/**
 * Create a progressive loader instance
 */
export function createProgressiveLoader(
  options?: ProgressiveLoadOptions,
): ProgressiveFileLoader {
  return new ProgressiveFileLoader(options);
}

/**
 * Load frames with progress tracking
 */
export async function loadFramesProgressive(
  filePath: string,
  onProgress?: ProgressCallback,
): Promise<FrameInfo[]> {
  const loader = new ProgressiveFileLoader({ onProgress });
  return loader.loadFrames(filePath);
}

/**
 * Load thumbnails with progress tracking
 */
export async function loadThumbnailsProgressive(
  filePath: string,
  frameIndices: number[],
  onProgress?: ProgressCallback,
): Promise<Map<number, string>> {
  const loader = new ProgressiveFileLoader({ onProgress });
  return loader.loadThumbnails(filePath, frameIndices);
}
