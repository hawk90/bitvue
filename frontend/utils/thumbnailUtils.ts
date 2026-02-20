/**
 * Thumbnail Utilities
 *
 * Shared utility functions for thumbnail processing
 */

import type { ThumbnailResult } from "../types/video";

/**
 * Process thumbnail result and extract data URL
 * Detects format from data content and constructs appropriate data URL
 */
export function processThumbnailResult(result: ThumbnailResult): string | null {
  if (!result.success || !result.thumbnail_data) {
    return null;
  }

  const data = result.thumbnail_data;

  // Already a complete data URL
  if (data.startsWith("data:")) {
    return data;
  }

  // SVG data
  if (data.startsWith("<svg") || data.startsWith("<SVG")) {
    return `data:image/svg+xml;base64,${data}`;
  }

  // Default: treat as PNG base64
  return `data:image/png;base64,${data}`;
}

/**
 * Process multiple thumbnail results and return a map of frame_index to data URL
 */
export function processThumbnailResults(
  results: ThumbnailResult[],
): Map<number, string> {
  const map = new Map<number, string>();

  for (const result of results) {
    const dataUrl = processThumbnailResult(result);
    if (dataUrl !== null) {
      map.set(result.frame_index, dataUrl);
    }
  }

  return map;
}

/**
 * Check if a thumbnail result is successful
 */
export function isThumbnailSuccessful(result: ThumbnailResult): boolean {
  return result.success && result.thumbnail_data.length > 0;
}

/**
 * Get error message from thumbnail result
 */
export function getThumbnailError(result: ThumbnailResult): string | null {
  if (!result.success) {
    return result.error || "Unknown error";
  }
  return null;
}

/**
 * Filter successful thumbnail results
 */
export function filterSuccessfulThumbnails(
  results: ThumbnailResult[],
): ThumbnailResult[] {
  return results.filter(isThumbnailSuccessful);
}

/**
 * Filter failed thumbnail results
 */
export function filterFailedThumbnails(
  results: ThumbnailResult[],
): ThumbnailResult[] {
  return results.filter((r) => !r.success);
}
