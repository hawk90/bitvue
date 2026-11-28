/**
 * Tests for exportUtils utility functions
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  exportUtils,
  generateAnalysisReport,
  buildFramesCsv,
  buildFramesJson,
  buildAnalysisReportText,
  type FrameExportData,
} from "@/utils/exportUtils";
import { saveTextFile } from "@/services/electronBridgeService";

vi.mock("@/services/electronBridgeService", () => ({
  saveTextFile: vi.fn(),
}));

describe("exportUtils", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("generateAnalysisReport", () => {
    const mockFrames: FrameExportData[] = [
      {
        frame_index: 0,
        frame_type: "I",
        size: 50000,
        key_frame: true,
      },
      {
        frame_index: 1,
        frame_type: "P",
        size: 25000,
        key_frame: false,
        ref_frames: [0],
      },
      {
        frame_index: 2,
        frame_type: "B",
        size: 15000,
        key_frame: false,
        ref_frames: [0, 1],
      },
    ];

    it("generates report with frame type distribution", () => {
      const report = generateAnalysisReport(mockFrames);

      expect(report.frame_type_distribution.i_frames).toBe(1);
      expect(report.frame_type_distribution.p_frames).toBe(1);
      expect(report.frame_type_distribution.b_frames).toBe(1);
    });

    it("generates size statistics", () => {
      const report = generateAnalysisReport(mockFrames);

      expect(report.size_statistics.total).toBe(90000);
      expect(report.size_statistics.average).toBe(30000);
      expect(report.size_statistics.max).toBe(50000);
      expect(report.size_statistics.min).toBe(15000);
    });

    it("generates GOP structure", () => {
      const report = generateAnalysisReport(mockFrames);

      // With 1 I-frame, there's only 1 GOP start, so no complete GOPs to measure
      // average_size is 0 because we need at least 2 I-frames to measure a GOP
      expect(report.gop_structure.count).toBe(1);
      expect(report.gop_structure.average_size).toBe(0);
    });

    it("includes codec information", () => {
      const report = generateAnalysisReport(mockFrames);

      expect(report.codec).toBe("Unknown");
      expect(report.width).toBe(1920);
      expect(report.height).toBe(1080);
      expect(report.total_frames).toBe(3);
    });
  });

  const REPORT = {
    codec: "hevc",
    width: 1920,
    height: 1080,
    total_frames: 100,
    frame_type_distribution: { i_frames: 10, p_frames: 40, b_frames: 50 },
    size_statistics: { total: 3000000, average: 30000, max: 50000, min: 10000 },
    gop_structure: { count: 10, average_size: 10 },
  };

  const FRAMES: FrameExportData[] = [
    { frame_index: 0, frame_type: "I", size: 50000, key_frame: true },
    {
      frame_index: 1,
      frame_type: "P",
      size: 25000,
      key_frame: false,
      ref_frames: [0, 2],
      pts: 3,
    },
  ];

  describe("buildFramesCsv", () => {
    it("writes a header and one row per frame, refs joined with ;", () => {
      expect(buildFramesCsv(FRAMES).split("\n")).toEqual([
        "Frame,Type,Size,POC,PTS,KeyFrame,TemporalLayer,SpatialLayer,RefFrames",
        "0,I,50000,,,Y,,,",
        "1,P,25000,,3,N,,,0;2",
      ]);
    });

    it("quotes cells with commas/quotes and defuses spreadsheet formulas", () => {
      const csv = buildFramesCsv([
        { frame_index: 0, frame_type: 'a,"b"', size: 1 },
        { frame_index: 1, frame_type: "=SUM(A1)", size: 1 },
        { frame_index: 2, frame_type: "-1", size: 1 },
      ]);
      const rows = csv.split("\n");
      expect(rows[1]).toBe('0,"a,""b""",1,,,N,,,');
      expect(rows[2]).toBe("1,'=SUM(A1),1,,,N,,,");
      expect(rows[3]).toBe("2,'-1,1,,,N,,,");
    });

    it("does not treat a negative number as a formula", () => {
      const csv = buildFramesCsv([
        { frame_index: 0, frame_type: "P", size: 1, poc: -2 },
      ]);
      expect(csv.split("\n")[1]).toBe("0,P,1,-2,,N,,,");
    });
  });

  describe("buildFramesJson / buildAnalysisReportText", () => {
    it("JSON carries the metadata, the count and every frame", () => {
      const parsed = JSON.parse(
        buildFramesJson(FRAMES, { codec: "av1", width: 320, height: 240 }),
      );
      expect(parsed.metadata).toEqual({
        codec: "av1",
        width: 320,
        height: 240,
      });
      expect(parsed.frame_count).toBe(2);
      expect(parsed.frames).toEqual(FRAMES);
    });

    it("the report text states every number it was given", () => {
      const text = buildAnalysisReportText(REPORT);
      for (const needle of [
        "hevc",
        "1920x1080",
        "Total frames: 100",
        "I: 10",
        "P: 40",
        "B: 50",
        "Total:   3000000",
        "Average: 30000",
        "Max:     50000",
        "Min:     10000",
        "Count:        10",
        "Average size: 10 frames",
      ]) {
        expect(text).toContain(needle);
      }
    });
  });

  describe("saving through the dialog (#605)", () => {
    it("CSV: hands the content to saveTextFile with no path and returns the saved path", async () => {
      vi.mocked(saveTextFile).mockResolvedValue("/test/export.csv");
      const result = await exportUtils.exportFramesToCsv(FRAMES);
      expect(saveTextFile).toHaveBeenCalledWith({
        defaultName: "frames.csv",
        filterName: "CSV",
        extensions: ["csv"],
        content: buildFramesCsv(FRAMES),
      });
      expect(result).toBe("/test/export.csv");
    });

    it("JSON and report use their own name, filter and content", async () => {
      vi.mocked(saveTextFile).mockResolvedValue("/test/out");
      const meta = { codec: "hevc", width: 1920, height: 1080 };
      await exportUtils.exportFramesToJson(FRAMES, meta);
      expect(saveTextFile).toHaveBeenLastCalledWith({
        defaultName: "frames.json",
        filterName: "JSON",
        extensions: ["json"],
        content: buildFramesJson(FRAMES, meta),
      });
      await exportUtils.exportAnalysisReport(REPORT, false);
      expect(saveTextFile).toHaveBeenLastCalledWith({
        defaultName: "analysis_report.txt",
        filterName: "Text",
        extensions: ["txt"],
        content: buildAnalysisReportText(REPORT),
      });
    });

    it("a cancelled dialog is reported as 'No file path selected'", async () => {
      vi.mocked(saveTextFile).mockResolvedValue(null);
      await expect(exportUtils.exportFramesToCsv(FRAMES)).rejects.toThrow(
        "No file path selected",
      );
    });

    it("a failed write rejects", async () => {
      vi.mocked(saveTextFile).mockRejectedValue(new Error("disk full"));
      await expect(
        exportUtils.exportFramesToJson(FRAMES, {
          codec: "av1",
          width: 1,
          height: 1,
        }),
      ).rejects.toThrow("disk full");
    });
  });

  describe("exportToPdf", () => {
    it("opens print dialog with HTML report", () => {
      const reportData = {
        codec: "hevc",
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

      expect(openSpy).toHaveBeenCalledWith("", "_blank");
      expect(mockPrintWindow.document.write).toHaveBeenCalled();
      expect(mockPrintWindow.document.close).toHaveBeenCalled();
      expect(mockPrintWindow.print).toHaveBeenCalled();
    });
  });
});
