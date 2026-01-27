/**
 * Data Export Utilities
 *
 * Export frame data to CSV or JSON format
 */

import type { FrameInfo } from '../types/video';

/**
 * Convert frames to CSV format
 */
export function framesToCSV(frames: FrameInfo[]): string {
  const headers = [
    'frame_index',
    'frame_type',
    'size',
    'pts',
    'poc',
    'temporal_id',
    'spatial_id',
    'display_order',
    'coding_order',
    'key_frame',
    'ref_frames',
  ];

  const rows = frames.map(frame => {
    // Validate frame structure to prevent runtime errors
    if (!frame || typeof frame !== 'object') {
      return Array(headers.length).fill('');
    }

    return [
      frame?.frame_index ?? '',
      frame?.frame_type ?? '',
      frame?.size ?? '',
      frame?.pts ?? '',
      frame?.poc ?? '',
      frame?.temporal_id ?? '',
      frame?.spatial_id ?? '',
      frame?.display_order ?? '',
      frame?.coding_order ?? '',
      frame?.key_frame ? 'true' : 'false',
      frame?.ref_frames?.join(';') ?? '',
    ];
  });

  const headerRow = headers.join(',');
  const dataRows = rows.map(row => row.join(','));

  return [headerRow, ...dataRows].join('\n');
}

/**
 * Generic file download utility
 * Creates a blob, generates a download link, and triggers the download
 */
function downloadFile(content: string, filename: string, mimeType: string): void {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}

/**
 * Download CSV file
 */
export function downloadCSV(csv: string, filename: string): void {
  downloadFile(csv, filename, 'text/csv');
}

/**
 * Export frames to CSV file
 */
export function exportFramesToCSV(frames: FrameInfo[], filename?: string): void {
  const csv = framesToCSV(frames);
  const defaultFilename = `bitvue-frames-${frames.length}-${new Date().toISOString().split('T')[0]}.csv`;
  downloadCSV(csv, filename || defaultFilename);
}

/**
 * Convert frames to JSON format
 */
export function framesToJSON(frames: FrameInfo[]): string {
  return JSON.stringify(frames, null, 2);
}

/**
 * Download JSON file
 */
export function downloadJSON(json: string, filename: string): void {
  downloadFile(json, filename, 'application/json');
}

/**
 * Export frames to JSON file
 */
export function exportFramesToJSON(frames: FrameInfo[], filename?: string): void {
  const json = framesToJSON(frames);
  const defaultFilename = `bitvue-frames-${frames.length}-${new Date().toISOString().split('T')[0]}.json`;
  downloadJSON(json, filename || defaultFilename);
}

/**
 * Generate frame statistics summary
 * Optimized to use a single iteration through the frames array
 */
export function generateFrameStatsSummary(frames: FrameInfo[]): {
  totalFrames: number;
  totalSize: number;
  avgSize: number;
  minSize: number;
  maxSize: number;
  frameTypes: Record<string, number>;
  keyframes: number;
} {
  const totalFrames = frames.length;
  const frameTypes: Record<string, number> = {};
  let keyframes = 0;
  let totalSize = 0;
  let minSize = Infinity;
  let maxSize = -Infinity;

  // Single iteration to calculate all statistics
  for (const frame of frames) {
    // Accumulate total size
    totalSize += frame.size;

    // Track min/max sizes
    if (frame.size < minSize) minSize = frame.size;
    if (frame.size > maxSize) maxSize = frame.size;

    // Count frame types
    frameTypes[frame.frame_type] = (frameTypes[frame.frame_type] || 0) + 1;

    // Count keyframes
    if (frame.key_frame || frame.frame_type === 'I' || frame.frame_type === 'KEY') {
      keyframes++;
    }
  }

  // Handle edge case for empty arrays
  if (minSize === Infinity) minSize = 0;
  if (maxSize === -Infinity) maxSize = 0;

  const avgSize = totalFrames > 0 ? totalSize / totalFrames : 0;

  return {
    totalFrames,
    totalSize,
    avgSize,
    minSize,
    maxSize,
    frameTypes,
    keyframes,
  };
}

/**
 * Export frame statistics to CSV
 */
export function exportFrameStatsToCSV(frames: FrameInfo[], filename?: string): void {
  const stats = generateFrameStatsSummary(frames);

  const csv = [
    'Metric,Value',
    `Total Frames,${stats.totalFrames}`,
    `Total Size,${stats.totalSize}`,
    `Average Size,${stats.avgSize.toFixed(2)}`,
    `Min Size,${stats.minSize}`,
    `Max Size,${stats.maxSize}`,
    `Keyframes,${stats.keyframes}`,
    '',
    'Frame Type,Count',
    ...Object.entries(stats.frameTypes).map(([type, count]) => `${type},${count}`),
  ].join('\n');

  const defaultFilename = `bitvue-stats-${new Date().toISOString().split('T')[0]}.csv`;
  downloadCSV(csv, filename || defaultFilename);
}

/**
 * Export frame sizes as CSV (alias for framesToCSV for backward compatibility)
 */
export function exportFrameSizes(frames: FrameInfo[]): string {
  return framesToCSV(frames);
}

/**
 * Export unit tree as JSON
 */
export function exportUnitTree(tree: any[]): string {
  return JSON.stringify(tree, null, 2);
}

/**
 * Export syntax tree as JSON
 */
export function exportSyntaxTree(tree: any): string {
  return JSON.stringify(tree, null, 2);
}

/**
 * Export metrics as CSV
 */
export function exportMetrics(metrics: Record<string, number>, frames: FrameInfo[]): string {
  const frameStats = generateFrameStatsSummary(frames);

  const csv = [
    'Metric,Value',
    ...Object.entries(metrics).map(([key, value]) => `${key},${value}`),
    '',
    'Frame Statistics',
    `Total Frames,${frameStats.totalFrames}`,
    `Total Size,${frameStats.totalSize}`,
    `Average Size,${frameStats.avgSize.toFixed(2)}`,
    `Min Size,${frameStats.minSize}`,
    `Max Size,${frameStats.maxSize}`,
    `Keyframes,${frameStats.keyframes}`,
  ].join('\n');

  return csv;
}
