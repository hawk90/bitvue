/**
 * Tests for exportUtils utility functions
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { exportUtils, generateAnalysisReport, type FrameExportData } from '@/utils/exportUtils';
import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';

// Mock Tauri APIs
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  save: vi.fn(() => Promise.resolve('/mock/path/export.csv')),
}));

describe('exportUtils', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('generateAnalysisReport', () => {
    const mockFrames: FrameExportData[] = [
      {
        frame_index: 0,
        frame_type: 'I',
        size: 50000,
        key_frame: true,
      },
      {
        frame_index: 1,
        frame_type: 'P',
        size: 25000,
        key_frame: false,
        ref_frames: [0],
      },
      {
        frame_index: 2,
        frame_type: 'B',
        size: 15000,
        key_frame: false,
        ref_frames: [0, 1],
      },
      ];

    it('generates report with frame type distribution', () => {
      const report = generateAnalysisReport(mockFrames);

      expect(report.frame_type_distribution.i_frames).toBe(1);
      expect(report.frame_type_distribution.p_frames).toBe(1);
      expect(report.frame_type_distribution.b_frames).toBe(1);
    });

    it('generates size statistics', () => {
      const report = generateAnalysisReport(mockFrames);

      expect(report.size_statistics.total).toBe(90000);
      expect(report.size_statistics.average).toBe(30000);
      expect(report.size_statistics.max).toBe(50000);
      expect(report.size_statistics.min).toBe(15000);
    });

    it('generates GOP structure', () => {
      const report = generateAnalysisReport(mockFrames);

      // With 1 I-frame, there's only 1 GOP start, so no complete GOPs to measure
      // average_size is 0 because we need at least 2 I-frames to measure a GOP
      expect(report.gop_structure.count).toBe(1);
      expect(report.gop_structure.average_size).toBe(0);
    });

    it('includes codec information', () => {
      const report = generateAnalysisReport(mockFrames);

      expect(report.codec).toBe('Unknown');
      expect(report.width).toBe(1920);
      expect(report.height).toBe(1080);
      expect(report.total_frames).toBe(3);
    });
  });

  describe('exportFramesToCsv', () => {
    it('exports frame data to CSV format', async () => {
      vi.mocked(save).mockResolvedValue('/test/export.csv');
      vi.mocked(invoke).mockResolvedValue('Exported 3 frames to /test/export.csv');

      const frames: FrameExportData[] = [
        {
          frame_index: 0,
          frame_type: 'I',
          size: 50000,
          key_frame: true,
        },
        {
          frame_index: 1,
          frame_type: 'P',
          size: 25000,
          key_frame: false,
          ref_frames: [0],
        },
      ];

      const result = await exportUtils.exportFramesToCsv(frames);

      expect(invoke).toHaveBeenCalledWith('export_frames_csv', {
        outputPath: '/test/export.csv',
      });
      expect(result).toBe('/test/export.csv');
    });
  });

  describe('exportFramesToJson', () => {
    it('exports frame data to JSON format', async () => {
      vi.mocked(save).mockResolvedValue('/test/export.json');
      vi.mocked(invoke).mockResolvedValue('Exported 2 frames to /test/export.json');

      const frames: FrameExportData[] = [
        {
          frame_index: 0,
          frame_type: 'I',
          size: 50000,
          key_frame: true,
        },
      ];

      const result = await exportUtils.exportFramesToJson(frames, {
        codec: 'hevc',
        width: 1920,
        height: 1080,
      });

      expect(invoke).toHaveBeenCalledWith('export_frames_json', {
        outputPath: '/test/export.json',
      });
      expect(result).toBe('/test/export.json');
    });
  });

  describe('exportAnalysisReport', () => {
    it('exports analysis report to text format', async () => {
      vi.mocked(save).mockResolvedValue('/test/report.txt');
      vi.mocked(invoke).mockResolvedValue('Exported analysis report to /test/report.txt');

      const reportData = {
        codec: 'hevc',
        width: 1920,
        height: 1080,
        total_frames: 100,
        frame_type_distribution: {
          i_frames: 10,
          p_frames: 40,
          b_frames: 50,
        },
        size_statistics: {
          total: 3000000,
          average: 30000,
          max: 50000,
          min: 10000,
        },
        gop_structure: {
          count: 10,
          average_size: 10,
        },
      };

      const result = await exportUtils.exportAnalysisReport(reportData, false);

      expect(invoke).toHaveBeenCalledWith('export_analysis_report', {
        outputPath: '/test/report.txt',
        includeSyntax: false,
      });
      expect(result).toBe('/test/report.txt');
    });
  });

  describe('exportToPdf', () => {
    it('opens print dialog with HTML report', () => {
      const reportData = {
        codec: 'hevc',
        width: 1920,
        height: 1080,
        total_frames: 100,
        frame_type_distribution: {
          i_frames: 10,
          p_frames: 40,
          b_frames: 50,
        },
        size_statistics: {
          total: 3000000,
          average: 30000,
          max: 50000,
          min: 10000,
        },
        gop_structure: {
          count: 10,
          average_size: 10,
        },
      };

      // Mock window.open and the returned window object
      const mockPrintWindow = {
        document: {
          write: vi.fn(),
          close: vi.fn(),
        },
        print: vi.fn(),
      };

      const openSpy = vi.fn().mockReturnValue(mockPrintWindow);
      global.open = openSpy;

      exportUtils.exportToPdf(reportData);

      expect(openSpy).toHaveBeenCalledWith('', '_blank');
      expect(mockPrintWindow.document.write).toHaveBeenCalled();
      expect(mockPrintWindow.document.close).toHaveBeenCalled();
      expect(mockPrintWindow.print).toHaveBeenCalled();
    });
  });
});
