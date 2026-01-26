/**
 * Export Functionality - v1.0.0
 *
 * Export analysis data to CSV/JSON formats
 * Supports: frames, statistics, QP data, MV data, partitions
 */

import { invoke } from '@tauri-apps/api/core';
import { createLogger } from './logger';

const logger = createLogger('export');

export interface ExportOptions {
  format: 'csv' | 'json';
  includeFrames?: boolean;
  includeStatistics?: boolean;
  includeQP?: boolean;
  includeMV?: boolean;
  includePartitions?: boolean;
  frameRange?: { start: number; end: number };
}

export interface ExportData {
  frames: FrameExportData[];
  statistics: StatisticsExportData;
  qpData: QPExportData[];
  mvData: MVExportData[];
  partitions: PartitionExportData[];
}

export interface FrameExportData {
  frame_index: number;
  frame_type: string;
  poc: number | null;
  pts: number | null;
  size: number;
  key_frame: boolean;
  width: number;
  height: number;
  qp?: number;
  temporal_id?: number;
  ref_frames?: number[];
}

export interface StatisticsExportData {
  totalFrames: number;
  keyFrames: number;
  totalSize: number;
  avgSize: number;
  minSize: number;
  maxSize: number;
  frameTypeCounts: Record<string, number>;
  avgQP: number;
  minQP: number;
  maxQP: number;
}

export interface QPExportData {
  frame_index: number;
  qp_min: number;
  qp_max: number;
  qp_avg: number;
  block_count: number;
}

export interface MVExportData {
  frame_index: number;
  mv_count: number;
  avg_mv_x: number;
  avg_mv_y: number;
  max_mv: number;
  zero_mv_count: number;
}

export interface PartitionExportData {
  frame_index: number;
  block_count: number;
  avg_size: number;
  max_depth: number;
  skip_count: number;
}

/**
 * Export data to CSV format
 */
export function exportToCSV(data: ExportData, options: ExportOptions = {}): string {
  const lines: string[] = [];

  // Frames CSV
  if (options.includeFrames) {
    lines.push('# Frame Data');
    lines.push('frame_index,frame_type,poc,pts,size,key_frame,width,height,qp,temporal_id,ref_frames');

    const frames = data.frames;
    const start = options.frameRange?.start ?? 0;
    const end = options.frameRange?.end ?? frames.length;

    for (let i = start; i < end && i < frames.length; i++) {
      const f = frames[i];
      lines.push([
        f.frame_index,
        f.frame_type,
        f.poc ?? '',
        f.pts ?? '',
        f.size,
        f.key_frame ? 'true' : 'false',
        f.width,
        f.height,
        f.qp ?? '',
        f.temporal_id ?? '',
        f.ref_frames ? `"${f.ref_frames.join(';')}"` : '',
      ].join(','));
    }

    lines.push('');
  }

  // Statistics CSV
  if (options.includeStatistics) {
    lines.push('# Statistics');
    lines.push('total_frames,key_frames,total_size,avg_size,min_size,max_size,avg_qp,min_qp,max_qp');
    lines.push([
      data.statistics.totalFrames,
      data.statistics.keyFrames,
      data.statistics.totalSize,
      data.statistics.avgSize.toFixed(2),
      data.statistics.minSize,
      data.statistics.maxSize,
      data.statistics.avgQP.toFixed(2),
      data.statistics.minQP,
      data.statistics.maxQP,
    ].join(','));

    lines.push('');
  }

  // QP Data CSV
  if (options.includeQP) {
    lines.push('# QP Data');
    lines.push('frame_index,qp_min,qp_max,qp_avg,block_count');

    for (const qp of data.qpData) {
      lines.push([
        qp.frame_index,
        qp.qp_min,
        qp.qp_max,
        qp.qp_avg.toFixed(2),
        qp.block_count,
      ].join(','));
    }

    lines.push('');
  }

  // MV Data CSV
  if (options.includeMV) {
    lines.push('# MV Data');
    lines.push('frame_index,mv_count,avg_mv_x,avg_mv_y,max_mv,zero_mv_count');

    for (const mv of data.mvData) {
      lines.push([
        mv.frame_index,
        mv.mv_count,
        mv.avg_mv_x.toFixed(2),
        mv.avg_mv_y.toFixed(2),
        mv.max_mv.toFixed(2),
        mv.zero_mv_count,
      ].join(','));
    }

    lines.push('');
  }

  // Partition Data CSV
  if (options.includePartitions) {
    lines.push('# Partition Data');
    lines.push('frame_index,block_count,avg_size,max_depth,skip_count');

    for (const part of data.partitions) {
      lines.push([
        part.frame_index,
        part.block_count,
        part.avg_size.toFixed(2),
        part.max_depth,
        part.skip_count,
      ].join(','));
    }
  }

  return lines.join('\n');
}

/**
 * Export data to JSON format
 */
export function exportToJSON(data: ExportData, options: ExportOptions = {}): string {
  const exportData: Record<string, unknown> = {};

  if (options.includeFrames) {
    const frames = data.frames;
    const start = options.frameRange?.start ?? 0;
    const end = options.frameRange?.end ?? frames.length;

    exportData.frames = frames.slice(start, end);
  }

  if (options.includeStatistics) {
    exportData.statistics = data.statistics;
  }

  if (options.includeQP) {
    exportData.qp_data = data.qpData;
  }

  if (options.includeMV) {
    exportData.mv_data = data.mvData;
  }

  if (options.includePartitions) {
    exportData.partitions = data.partitions;
  }

  exportData.export_timestamp = new Date().toISOString();
  exportData.bitvue_version = '1.0.0';

  return JSON.stringify(exportData, null, 2);
}

/**
 * Trigger file download
 */
export function downloadFile(content: string, filename: string, mimeType: string): void {
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
 * Export frames and trigger download
 */
export async function exportData(
  filePath: string,
  options: ExportOptions = {
    format: 'json',
    includeFrames: true,
    includeStatistics: true,
  }
): Promise<void> {
  try {
    logger.info('Exporting data for:', filePath, 'options:', options);

    // Get frames from backend
    const frames = await invoke<any[]>('get_frames', { path: filePath });

    // Collect frame data for export
    const exportData: ExportData = {
      frames: frames.map((f, i) => ({
        frame_index: i,
        frame_type: f.frame_type,
        poc: f.poc,
        pts: f.pts,
        size: f.size,
        key_frame: f.key_frame ?? false,
        width: 0, // Would be populated from actual data
        height: 0,
      })),
      statistics: {
        totalFrames: frames.length,
        keyFrames: frames.filter((f: any) => f.key_frame || f.frame_type === 'I').length,
        totalSize: frames.reduce((sum: number, f: any) => sum + f.size, 0),
        avgSize: 0,
        minSize: 0,
        maxSize: 0,
        frameTypeCounts: {},
        avgQP: 0,
        minQP: 0,
        maxQP: 0,
      },
      qpData: [],
      mvData: [],
      partitions: [],
    };

    // Calculate statistics
    const sizes = frames.map((f: any) => f.size);
    exportData.statistics.avgSize = sizes.reduce((a, b) => a + b, 0) / sizes.length;
    exportData.statistics.minSize = Math.min(...sizes);
    exportData.statistics.maxSize = Math.max(...sizes);

    // Count frame types
    frames.forEach((f: any) => {
      const type = f.frame_type;
      exportData.statistics.frameTypeCounts[type] = (exportData.statistics.frameTypeCounts[type] || 0) + 1;
    });

    // Convert to requested format
    const content = options.format === 'csv'
      ? exportToCSV(exportData, options)
      : exportToJSON(exportData, options);

    // Generate filename
    const baseName = filePath.split(/[/\\]/).pop() ?? 'export';
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
    const extension = options.format === 'csv' ? 'csv' : 'json';
    const filename = `${baseName}_export_${timestamp}.${extension}`;

    // Trigger download
    const mimeType = options.format === 'csv' ? 'text/csv' : 'application/json';
    downloadFile(content, filename, mimeType);

    logger.info('Export completed:', filename);
  } catch (error) {
    logger.error('Export failed:', error);
    throw error;
  }
}

/**
 * Export per-frame analysis data
 */
export async function exportFrameAnalysis(
  filePath: string,
  frameIndex: number,
  format: 'csv' | 'json' = 'json'
): Promise<void> {
  try {
    const analysis = await invoke('get_frame_analysis', {
      path: filePath,
      frame_index: frameIndex,
    });

    const filename = `frame_${frameIndex}_analysis_${format === 'csv' ? 'csv' : 'json'}`;
    const content = format === 'csv'
      ? exportAnalysisToCSV(analysis, frameIndex)
      : JSON.stringify(analysis, null, 2);

    const mimeType = format === 'csv' ? 'text/csv' : 'application/json';
    downloadFile(content, filename, mimeType);

    logger.info('Frame analysis exported:', filename);
  } catch (error) {
    logger.error('Frame analysis export failed:', error);
    throw error;
  }
}

/**
 * Export analysis data to CSV
 */
function exportAnalysisToCSV(analysis: any, frameIndex: number): string {
  const lines: string[] = [];
  lines.push('# Frame Analysis');
  lines.push(`# Frame: ${frameIndex}`);

  if (analysis.qp_grid) {
    const { qp } = analysis.qp_grid;
    lines.push('# QP Grid');
    lines.push('block_index,qp_value');

    qp.forEach((value: number, i: number) => {
      lines.push(`${i},${value}`);
    });
    lines.push('');
  }

  if (analysis.mv_grid) {
    const { mv_l0, mv_l1 } = analysis.mv_grid;
    lines.push('# MV Grid L0');
    lines.push('block_index,mv_x,mv_y');

    mv_l0.forEach((mv: any, i: number) => {
      lines.push(`${i},${mv.dx_qpel},${mv.dy_qpel}`);
    });
    lines.push('');
  }

  if (analysis.partition_grid) {
    const { blocks } = analysis.partition_grid;
    lines.push('# Partition Grid');
    lines.push('block_index,x,y,width,height,partition,depth');

    blocks.forEach((block: any, i: number) => {
      lines.push(`${i},${block.x},${block.y},${block.width},${block.height},${block.partition},${block.depth}`);
    });
    lines.push('');
  }

  return lines.join('\n');
}

/**
 * Export batch analysis for multiple frames
 */
export async function exportBatchAnalysis(
  filePath: string,
  frameIndices: number[],
  format: 'csv' | 'json' = 'json'
): Promise<void> {
  try {
    logger.info('Batch export:', filePath, 'frames:', frameIndices.length);

    const allAnalysis: any[] = [];

    for (const frameIndex of frameIndices) {
      const analysis = await invoke('get_frame_analysis', {
        path: filePath,
        frame_index: frameIndex,
      });
      allAnalysis.push({ frame_index: frameIndex, ...analysis });
    }

    const content = JSON.stringify(allAnalysis, null, 2);
    const filename = `batch_analysis_${new Date().toISOString().slice(0, 19)}.json`;
    downloadFile(content, filename, 'application/json');

    logger.info('Batch export completed:', filename);
  } catch (error) {
    logger.error('Batch export failed:', error);
    throw error;
  }
}

/**
 * Create export configuration from UI state
 */
export function createExportConfig(
  state: {
    format: 'csv' | 'json';
    includeFrames: boolean;
    includeStats: boolean;
    includeQP: boolean;
    includeMV: boolean;
    includePartitions: boolean;
    frameRange?: { start: number; end: number };
  }
): ExportOptions {
  return {
    format: state.format,
    includeFrames: state.includeFrames,
    includeStatistics: state.includeStats,
    includeQP: state.includeQP,
    includeMV: state.includeMV,
    includePartitions: state.includePartitions,
    frameRange: state.frameRange,
  };
}
