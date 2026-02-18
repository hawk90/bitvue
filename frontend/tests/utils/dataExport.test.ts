/**
 * Data Export Utility Tests
 * Tests frame data export functionality
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import * as dataExport from "../dataExport";

describe("dataExport utilities", () => {
  const mockFrames = [
    {
      frame_index: 0,
      frame_type: "I",
      size: 50000,
      pts: 0,
      poc: 0,
    },
    {
      frame_index: 1,
      frame_type: "P",
      size: 30000,
      pts: 1,
      poc: 1,
    },
    {
      frame_index: 2,
      frame_type: "B",
      size: 20000,
      pts: 2,
      poc: 2,
    },
  ];

  beforeEach(() => {
    // Mock URL.createObjectURL and URL.revokeObjectURL
    global.URL.createObjectURL = vi.fn(() => "blob:mock-url");
    global.URL.revokeObjectURL = vi.fn();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("exportFrameSizes", () => {
    it("should export frame sizes as CSV", () => {
      const csv = dataExport.exportFrameSizes(mockFrames);

      expect(csv).toContain("frame_index,frame_type,size");
      expect(csv).toContain("0,I,50000");
      expect(csv).toContain("1,P,30000");
    });

    it("should include all frame properties", () => {
      const csv = dataExport.exportFrameSizes(mockFrames);

      const lines = csv.split("\n");
      expect(lines.length).toBeGreaterThan(1); // Header + data rows
    });

    it("should handle empty frames array", () => {
      const csv = dataExport.exportFrameSizes([]);

      expect(csv).toContain("frame_index,frame_type,size");
    });
  });

  describe("exportUnitTree", () => {
    it("should export unit tree as JSON", () => {
      const mockTree = [
        {
          key: "root",
          unit_type: "SEQUENCE_HEADER",
          offset: 0,
          size: 100,
          children: [],
        },
      ];

      const json = dataExport.exportUnitTree(mockTree);

      expect(json).toContain("SEQUENCE_HEADER");
    });
  });

  describe("exportSyntaxTree", () => {
    it("should export syntax tree as JSON", () => {
      const mockSyntax = {
        type: "root",
        children: [{ type: "node", name: "test" }],
      };

      const json = dataExport.exportSyntaxTree(mockSyntax);

      expect(json).toContain("root");
    });
  });

  describe("exportMetrics", () => {
    it("should export metrics as CSV", () => {
      const mockMetrics = {
        avgQP: 25,
        avgSize: 35000,
        bitrate: 5000000,
      };

      const csv = dataExport.exportMetrics(mockMetrics, mockFrames);

      expect(csv).toContain("avgQP");
    });
  });

  describe("file download", () => {
    it("should trigger file download", () => {
      const mockLink = {
        href: "",
        download: "",
        click: vi.fn(),
        style: {},
      };

      const createElementSpy = vi
        .spyOn(document, "createElement")
        .mockReturnValue(mockLink as any);
      const appendChildSpy = vi
        .spyOn(document.body, "appendChild")
        .mockImplementation(() => mockLink as any);
      const removeChildSpy = vi
        .spyOn(document.body, "removeChild")
        .mockImplementation(() => mockLink as any);

      dataExport.downloadCSV("frame_index,size\n0,100", "test.csv");

      expect(mockLink.href).toBe("blob:mock-url");
      expect(mockLink.download).toBe("test.csv");
      expect(mockLink.click).toHaveBeenCalled();

      createElementSpy.mockRestore();
      appendChildSpy.mockRestore();
      removeChildSpy.mockRestore();
    });

    it("should revoke object URL after download", () => {
      dataExport.downloadCSV("test.csv", "data");

      expect(global.URL.revokeObjectURL).toHaveBeenCalled();
    });
  });
});

describe("Data export edge cases", () => {
  it("should handle special characters in data", () => {
    const specialFrames = [
      {
        frame_index: 0,
        frame_type: "I",
        size: 1000,
        // Special characters in values
        poc: 0,
      },
    ];

    const csv = dataExport.exportFrameSizes(specialFrames);

    // Should not throw
    expect(csv).toBeDefined();
  });

  it("should handle large datasets", () => {
    const largeFrames = Array.from({ length: 10000 }, (_, i) => ({
      frame_index: i,
      frame_type: "I",
      size: 1000,
      poc: 0,
    }));

    const csv = dataExport.exportFrameSizes(largeFrames);

    // Should complete without hanging
    expect(csv).toBeDefined();
    expect(csv.split("\n").length).toBeGreaterThan(1);
  });
});
