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

  const rows = frames.map(frame => [
    frame.frame_index,
    frame.frame_type,
    frame.size,
    frame.pts ?? '',
    frame.poc ?? '',
    frame.temporal_id ?? '',
    frame.spatial_id ?? '',
    frame.display_order ?? '',
    frame.coding_order ?? '',
    frame.key_frame ? 'true' : 'false',
    frame.ref_frames?.join(';') ?? '',
  ]);

  const headerRow = headers.join(',');
  const dataRows = rows.map(row => row.join(','));

  return [headerRow, ...dataRows].join('\n');
}

/**
 * Download CSV file
 */
export function downloadCSV(csv: string, filename: string): void {
  const blob = new Blob([csv], { type: 'text/csv' });
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
  const blob = new Blob([json], { type: 'application/json' });
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
 * Export frames to JSON file
 */
export function exportFramesToJSON(frames: FrameInfo[], filename?: string): void {
  const json = framesToJSON(frames);
  const defaultFilename = `bitvue-frames-${frames.length}-${new Date().toISOString().split('T')[0]}.json`;
  downloadJSON(json, filename || defaultFilename);
}

/**
 * Generate frame statistics summary
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
  const totalSize = frames.reduce((sum, f) => sum + f.size, 0);
  const avgSize = totalFrames > 0 ? totalSize / totalFrames : 0;
  const sizes = frames.map(f => f.size);
  const minSize = Math.min(...sizes);
  const maxSize = Math.max(...sizes);

  const frameTypes: Record<string, number> = {};
  let keyframes = 0;

  frames.forEach(frame => {
    frameTypes[frame.frame_type] = (frameTypes[frame.frame_type] || 0) + 1;
    if (frame.key_frame || frame.frame_type === 'I' || frame.frame_type === 'KEY') {
      keyframes++;
    }
  });

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
