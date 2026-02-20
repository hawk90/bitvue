/**
 * Platform Utility Tests
 * Tests platform-related utility functions
 */

import { describe, it, expect } from "vitest";
import {
  extractFileName,
  extractFileExtension,
  formatFileSize,
} from "@/types/platform";

describe("extractFileName", () => {
  it("should extract filename from Unix path", () => {
    expect(extractFileName("/path/to/file.txt")).toBe("file.txt");
    expect(extractFileName("/home/user/video.mp4")).toBe("video.mp4");
  });

  it("should extract filename from Windows path", () => {
    expect(extractFileName("C:\\path\\to\\file.txt")).toBe("file.txt");
    expect(extractFileName("D:\\Videos\\test.hevc")).toBe("test.hevc");
  });

  it("should handle paths with mixed separators", () => {
    expect(extractFileName("/path\\to/file.txt")).toBe("file.txt");
  });

  it("should handle filename without path", () => {
    expect(extractFileName("file.txt")).toBe("file.txt");
    expect(extractFileName("video.mp4")).toBe("video.mp4");
  });

  it("should handle empty string", () => {
    expect(extractFileName("")).toBe("");
  });

  it("should handle path ending with separator", () => {
    expect(extractFileName("/path/to/")).toBe("");
    expect(extractFileName("C:\\path\\to\\")).toBe("");
  });

  it("should handle paths with multiple extensions", () => {
    expect(extractFileName("/path/to/file.tar.gz")).toBe("file.tar.gz");
  });

  it("should handle paths with dots in directory names", () => {
    expect(extractFileName("/path.with.dots/file.txt")).toBe("file.txt");
  });

  it("should handle hidden files", () => {
    expect(extractFileName("/path/to/.hidden")).toBe(".hidden");
  });
});

describe("extractFileExtension", () => {
  it("should extract file extension", () => {
    expect(extractFileExtension("file.txt")).toBe("txt");
    expect(extractFileExtension("video.mp4")).toBe("mp4");
    expect(extractFileExtension("archive.tar.gz")).toBe("gz");
  });

  it("should return empty string for files without extension", () => {
    expect(extractFileExtension("file")).toBe("");
    expect(extractFileExtension("Makefile")).toBe("");
  });

  it("should return empty string for empty input", () => {
    expect(extractFileExtension("")).toBe("");
  });

  it("should handle paths", () => {
    expect(extractFileExtension("/path/to/file.txt")).toBe("txt");
    expect(extractFileExtension("C:\\path\\to\\video.mp4")).toBe("mp4");
  });

  it("should convert extension to lowercase", () => {
    expect(extractFileExtension("file.MP4")).toBe("mp4");
    expect(extractFileExtension("file.TxT")).toBe("txt");
    expect(extractFileExtension("file.HEIC")).toBe("heic");
  });

  it("should handle hidden files", () => {
    expect(extractFileExtension(".hidden")).toBe("hidden");
    expect(extractFileExtension(".gitignore")).toBe("gitignore");
  });

  it("should handle files with multiple dots", () => {
    expect(extractFileExtension("file.name.with.dots.txt")).toBe("txt");
  });

  it("should handle files starting with dot and extension", () => {
    expect(extractFileExtension(".config.json")).toBe("json");
  });
});

describe("formatFileSize", () => {
  it("should format bytes", () => {
    expect(formatFileSize(0)).toBe("0 B");
    expect(formatFileSize(100)).toBe("100 B");
    expect(formatFileSize(512)).toBe("512 B");
    expect(formatFileSize(1023)).toBe("1023 B");
  });

  it("should format kilobytes", () => {
    expect(formatFileSize(1024)).toBe("1.00 KB");
    expect(formatFileSize(1536)).toBe("1.50 KB");
    expect(formatFileSize(10240)).toBe("10.00 KB");
    expect(formatFileSize(1048575)).toBe("1024.00 KB");
  });

  it("should format megabytes", () => {
    expect(formatFileSize(1048576)).toBe("1.00 MB");
    expect(formatFileSize(5242880)).toBe("5.00 MB");
    expect(formatFileSize(10485760)).toBe("10.00 MB");
    expect(formatFileSize(1073741823)).toBe("1024.00 MB");
  });

  it("should format gigabytes", () => {
    expect(formatFileSize(1073741824)).toBe("1.00 GB");
    expect(formatFileSize(2147483648)).toBe("2.00 GB");
    expect(formatFileSize(5368709120)).toBe("5.00 GB");
    expect(formatFileSize(10737418240)).toBe("10.00 GB");
  });

  it("should handle large values", () => {
    expect(formatFileSize(123456789012)).toBe("114.98 GB");
    expect(formatFileSize(999999999999)).toBe("931.32 GB");
  });

  it("should format with correct decimal places", () => {
    const result = formatFileSize(1234567);
    expect(result).toMatch(/^\d+\.\d{2} MB$/);
  });
});
