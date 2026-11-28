/**
 * Export Utilities for Bitvue
 *
 * Utilities for exporting frame data, analysis results, and reports
 * in various formats (CSV, JSON, PDF).
 *
 * CSV, JSON and the text report are built here and saved through `saveTextFile` (a native save
 * dialog shown by the main process, #605), so the renderer never picks a file path.
 * `exportToPdf`/`generateHtmlReport` are pure client-side `window.open()` + HTML.
 */

import { saveTextFile } from "../services/electronBridgeService";

export type ExportFormat = "csv" | "json" | "txt" | "pdf";

export interface ExportOptions {
  format: ExportFormat;
  includeSyntax?: boolean;
  includeAnalysis?: boolean;
  frameRange?: [number, number];
}

export interface FrameExportData {
  frame_index: number;
  frame_type: string;
  size: number;
  poc?: number;
  pts?: number;
  key_frame?: boolean;
  temporal_id?: number;
  spatial_id?: number;
  ref_frames?: number[];
}

export interface AnalysisReportData {
  codec: string;
  width: number;
  height: number;
  total_frames: number;
  frame_type_distribution: {
    i_frames: number;
    p_frames: number;
    b_frames: number;
  };
  size_statistics: {
    total: number;
    average: number;
    max: number;
    min: number;
  };
  gop_structure: {
    count: number;
    average_size: number;
  };
}

/**
 * Escape HTML special characters to prevent XSS attacks
 *
 * This is critical when generating HTML reports that include user data
 * or file-derived data that could contain malicious content.
 */
function escapeHtml(unsafe: string | number): string {
  const str = String(unsafe);
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

/** One CSV cell. A value with a comma, quote or line break is quoted; one that starts with a
 *  formula character (`=`, `+`, `-`, `@`) gets a leading `'` so a spreadsheet shows it as text
 *  instead of evaluating it. Numbers are written as they are (a negative number is not a formula). */
function csvCell(value: string | number | undefined): string {
  if (value === undefined) return "";
  if (typeof value === "number") return String(value);
  let text = value;
  if (/^[=+\-@\t\r]/.test(text)) text = `'${text}`;
  return /[",\n\r]/.test(text) ? `"${text.replace(/"/g, '""')}"` : text;
}

/** The frame list as CSV text. */
export function buildFramesCsv(frames: FrameExportData[]): string {
  return [
    "Frame,Type,Size,POC,PTS,KeyFrame,TemporalLayer,SpatialLayer,RefFrames",
    ...frames.map((f) =>
      [
        csvCell(f.frame_index),
        csvCell(f.frame_type),
        csvCell(f.size),
        csvCell(f.poc),
        csvCell(f.pts),
        f.key_frame ? "Y" : "N",
        csvCell(f.temporal_id),
        csvCell(f.spatial_id),
        csvCell(f.ref_frames?.join(";")),
      ].join(","),
    ),
  ].join("\n");
}

/** The frame list and stream metadata as JSON text. */
export function buildFramesJson(
  frames: FrameExportData[],
  metadata: { codec: string; width: number; height: number },
): string {
  return JSON.stringify(
    { metadata, frame_count: frames.length, frames },
    null,
    2,
  );
}

/** The analysis report as plain text. */
export function buildAnalysisReportText(data: AnalysisReportData): string {
  const d = data.frame_type_distribution;
  const s = data.size_statistics;
  return [
    "Bitvue Analysis Report",
    "======================",
    "",
    `Codec:        ${data.codec}`,
    `Resolution:   ${data.width}x${data.height}`,
    `Total frames: ${data.total_frames}`,
    "",
    "Frame types",
    `  I: ${d.i_frames}`,
    `  P: ${d.p_frames}`,
    `  B: ${d.b_frames}`,
    "",
    "Frame sizes (bytes)",
    `  Total:   ${s.total}`,
    `  Average: ${s.average}`,
    `  Max:     ${s.max}`,
    `  Min:     ${s.min}`,
    "",
    "GOPs",
    `  Count:        ${data.gop_structure.count}`,
    `  Average size: ${data.gop_structure.average_size} frames`,
    "",
  ].join("\n");
}

/** Saves `content` through the save dialog; throws "No file path selected" when the user
 *  cancels (the dialog shows that as its error state). */
async function saveThroughDialog(
  defaultName: string,
  filterName: string,
  extension: string,
  content: string,
): Promise<string> {
  const filePath = await saveTextFile({
    defaultName,
    filterName,
    extensions: [extension],
    content,
  });
  if (!filePath) throw new Error("No file path selected");
  return filePath;
}

/**
 * Export frames to CSV format
 */
export async function exportFramesToCsv(
  frames: FrameExportData[],
): Promise<string> {
  return saveThroughDialog("frames.csv", "CSV", "csv", buildFramesCsv(frames));
}

/**
 * Export frames to JSON format
 */
export async function exportFramesToJson(
  frames: FrameExportData[],
  metadata: {
    codec: string;
    width: number;
    height: number;
  },
): Promise<string> {
  return saveThroughDialog(
    "frames.json",
    "JSON",
    "json",
    buildFramesJson(frames, metadata),
  );
}

/**
 * Export analysis report to text format
 */
export async function exportAnalysisReport(
  data: AnalysisReportData,
  // The report is built from `data` alone; there is no syntax data to include, so this flag
  // (still passed by ExportDialog's checkbox) changes nothing.
  _includeSyntax: boolean = false,
): Promise<string> {
  return saveThroughDialog(
    "analysis_report.txt",
    "Text",
    "txt",
    buildAnalysisReportText(data),
  );
}

/**
 * Generate analysis report data from frames
 */
export function generateAnalysisReport(
  frames: FrameExportData[],
  metadata: { codec?: string; width?: number; height?: number } = {},
): AnalysisReportData {
  const defaultReport: AnalysisReportData = {
    codec: metadata.codec ?? "Unknown",
    width: metadata.width ?? 0,
    height: metadata.height ?? 0,
    total_frames: 0,
    frame_type_distribution: { i_frames: 0, p_frames: 0, b_frames: 0 },
    size_statistics: { total: 0, average: 0, max: 0, min: 0 },
    gop_structure: { count: 0, average_size: 0 },
  };

  if (frames.length === 0) return defaultReport;

  const iFrames = frames.filter((f) => f.frame_type === "I").length;
  const pFrames = frames.filter((f) => f.frame_type === "P").length;
  const bFrames = frames.filter((f) => f.frame_type === "B").length;

  const sizes = frames.map((f) => f.size);
  const totalSize = sizes.reduce((a, b) => a + b, 0);
  const avgSize = totalSize / frames.length;
  const maxSize = Math.max(...sizes);
  const minSize = Math.min(...sizes);

  // GOP analysis
  const gopStarts: number[] = [];
  frames.forEach((f, idx) => {
    if (f.frame_type === "I") {
      gopStarts.push(idx);
    }
  });

  let avgGopSize = 0;
  if (gopStarts.length > 1) {
    const gopSizes: number[] = [];
    for (let i = 0; i < gopStarts.length - 1; i++) {
      gopSizes.push(gopStarts[i + 1] - gopStarts[i]);
    }
    avgGopSize = gopSizes.reduce((a, b) => a + b, 0) / gopSizes.length;
  }

  return {
    codec: metadata.codec ?? "Unknown",
    width: metadata.width ?? 1920,
    height: metadata.height ?? 1080,
    total_frames: frames.length,
    frame_type_distribution: {
      i_frames: iFrames,
      p_frames: pFrames,
      b_frames: bFrames,
    },
    size_statistics: {
      total: totalSize,
      average: Math.round(avgSize),
      max: maxSize,
      min: minSize,
    },
    gop_structure: {
      count: gopStarts.length,
      average_size: Math.round(avgGopSize),
    },
  };
}

/**
 * Export to PDF (generates HTML then prints)
 */
export async function exportToPdf(
  reportData: AnalysisReportData,
): Promise<void> {
  const reportContent = generateHtmlReport(reportData);

  // Create a new window with the report
  const printWindow = window.open("", "_blank");
  if (printWindow === null) {
    throw new Error(
      "Unable to open print window. Please allow pop-ups for this application.",
    );
  }
  printWindow.document.write(reportContent);
  printWindow.document.close();
  printWindow.print();
}

/**
 * Generate HTML report for PDF export
 *
 * SECURITY: All user-provided data is HTML-escaped to prevent XSS attacks.
 * This includes codec name, dimensions, and all statistics.
 */
function generateHtmlReport(data: AnalysisReportData): string {
  // Escape all user data to prevent XSS
  const safeCodec = escapeHtml(data.codec);
  const safeWidth = escapeHtml(data.width);
  const safeHeight = escapeHtml(data.height);
  const safeTotalFrames = escapeHtml(data.total_frames);
  const safeIFrames = escapeHtml(data.frame_type_distribution.i_frames);
  const safePFrames = escapeHtml(data.frame_type_distribution.p_frames);
  const safeBFrames = escapeHtml(data.frame_type_distribution.b_frames);
  const safeTotal = escapeHtml(data.size_statistics.total.toLocaleString());
  const safeAverage = escapeHtml(data.size_statistics.average.toLocaleString());
  const safeMax = escapeHtml(data.size_statistics.max.toLocaleString());
  const safeMin = escapeHtml(data.size_statistics.min.toLocaleString());
  const safeGopCount = escapeHtml(data.gop_structure.count);
  const safeAvgGopSize = escapeHtml(data.gop_structure.average_size);

  // Calculate percentages safely
  const iPct =
    data.total_frames > 0
      ? (
          (data.frame_type_distribution.i_frames / data.total_frames) *
          100
        ).toFixed(1)
      : "0.0";
  const pPct =
    data.total_frames > 0
      ? (
          (data.frame_type_distribution.p_frames / data.total_frames) *
          100
        ).toFixed(1)
      : "0.0";
  const bPct =
    data.total_frames > 0
      ? (
          (data.frame_type_distribution.b_frames / data.total_frames) *
          100
        ).toFixed(1)
      : "0.0";
  const safeMb = escapeHtml(
    (data.size_statistics.total / 1024 / 1024).toFixed(2),
  );

  return `
<!DOCTYPE html>
<html>
<head>
  <title>Bitvue Analysis Report</title>
  <style>
    body { font-family: Arial, sans-serif; padding: 40px; }
    h1 { color: #333; }
    .section { margin: 20px 0; }
    .stat { margin: 10px 0; }
    table { border-collapse: collapse; width: 100%; }
    th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }
    th { background-color: #f2f2f2; }
  </style>
</head>
<body>
  <h1>Bitvue Analysis Report</h1>

  <div class="section">
    <h2>Stream Information</h2>
    <div class="stat">Codec: ${safeCodec}</div>
    <div class="stat">Resolution: ${safeWidth}x${safeHeight}</div>
    <div class="stat">Total Frames: ${safeTotalFrames}</div>
  </div>

  <div class="section">
    <h2>Frame Type Distribution</h2>
    <table>
      <tr><th>Type</th><th>Count</th><th>Percentage</th></tr>
      <tr><td>I-Frames</td><td>${safeIFrames}</td><td>${escapeHtml(iPct)}%</td></tr>
      <tr><td>P-Frames</td><td>${safePFrames}</td><td>${escapeHtml(pPct)}%</td></tr>
      <tr><td>B-Frames</td><td>${safeBFrames}</td><td>${escapeHtml(bPct)}%</td></tr>
    </table>
  </div>

  <div class="section">
    <h2>Size Statistics</h2>
    <div class="stat">Total: ${safeTotal} bytes (${safeMb} MB)</div>
    <div class="stat">Average: ${safeAverage} bytes</div>
    <div class="stat">Max: ${safeMax} bytes</div>
    <div class="stat">Min: ${safeMin} bytes</div>
  </div>

  <div class="section">
    <h2>GOP Structure</h2>
    <div class="stat">Number of GOPs: ${safeGopCount}</div>
    <div class="stat">Average GOP size: ${safeAvgGopSize}</div>
  </div>

  <div class="section">
    <p><em>Generated by Bitvue 1.0.0</em></p>
  </div>
</body>
</html>
  `;
}

export const exportUtils = {
  exportFramesToCsv,
  exportFramesToJson,
  exportAnalysisReport,
  generateAnalysisReport,
  exportToPdf,
};
