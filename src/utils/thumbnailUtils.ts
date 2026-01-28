/**
 * Thumbnail Utilities
 *
 * Shared utility functions for thumbnail processing
 */

import type { ThumbnailResult } from '../types/video';

/**
 * Process thumbnail result and extract data URL
 * Backend already returns complete data URLs, so we just validate and extract
 */
export function processThumbnailResult(result: ThumbnailResult): string | null {
    if (result.success && result.thumbnail_data) {
        // Backend returns complete data URL (e.g., "data:image/png;base64,...")
        // Just return it directly
        return result.thumbnail_data;
    }
    return null;
}

/**
 * Process multiple thumbnail results and return a map of frame_index to data URL
 */
export function processThumbnailResults(results: ThumbnailResult[]): Map<number, string> {
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
        return result.error || 'Unknown error';
    }
    return null;
}

/**
 * Filter successful thumbnail results
 */
export function filterSuccessfulThumbnails(results: ThumbnailResult[]): ThumbnailResult[] {
    return results.filter(isThumbnailSuccessful);
}

/**
 * Filter failed thumbnail results
 */
export function filterFailedThumbnails(results: ThumbnailResult[]): ThumbnailResult[] {
    return results.filter(r => !r.success);
}
