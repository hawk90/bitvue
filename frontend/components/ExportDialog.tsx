/**
 * Export Dialog Component
 *
 * Dialog for exporting frame data, analysis results, and reports.
 * Supports CSV, JSON, and PDF export formats.
 */

import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { exportUtils, type ExportFormat } from "../utils/exportUtils";
import type { FrameInfo } from "../types/video";
import "./ExportDialog.css";

interface ExportDialogProps {
  isOpen: boolean;
  onClose: () => void;
  frames: FrameInfo[];
  codec?: string;
  width?: number;
  height?: number;
}

type ExportType = "frames" | "analysis" | "report";
type ExportStatus = "idle" | "exporting" | "success" | "error";

export function ExportDialog({
  isOpen,
  onClose,
  frames,
  codec = "Unknown",
  width = 1920,
  height = 1080,
}: ExportDialogProps) {
  const [exportType, setExportType] = useState<ExportType>("frames");
  const [format, setFormat] = useState<ExportFormat>("csv");
  const [includeSyntax, setIncludeSyntax] = useState(false);
  const [status, setStatus] = useState<ExportStatus>("idle");
  const [message, setMessage] = useState("");

  const handleExport = useCallback(async () => {
    if (frames.length === 0) {
      setStatus("error");
      setMessage("No frames to export");
      return;
    }

    setStatus("exporting");
    setMessage("Exporting...");

    try {
      let result = "";

      switch (exportType) {
        case "frames":
          if (format === "csv") {
            result = await exportUtils.exportFramesToCsv(
              frames.map((f) => ({
                frame_index: f.frameNumber,
                frame_type: f.frameType,
                size: f.size || 0,
                poc: f.poc,
                pts: f.pts,
                key_frame: f.frameType === "I",
                temporal_id: f.temporalId,
                spatial_id: f.spatialId,
              })),
            );
          } else if (format === "json") {
            result = await exportUtils.exportFramesToJson(
              frames.map((f) => ({
                frame_index: f.frameNumber,
                frame_type: f.frameType,
                size: f.size || 0,
                poc: f.poc,
                pts: f.pts,
                key_frame: f.frameType === "I",
                temporal_id: f.temporalId,
                spatial_id: f.spatialId,
              })),
              { codec, width, height },
            );
          }
          break;

        case "analysis":
        case "report":
          const reportData = exportUtils.generateAnalysisReport(
            frames.map((f) => ({
              frame_index: f.frameNumber,
              frame_type: f.frameType,
              size: f.size || 0,
              poc: f.poc,
              pts: f.pts,
              key_frame: f.frameType === "I",
              temporal_id: f.temporalId,
              spatial_id: f.spatialId,
            })),
          );

          if (format === "txt" || format === "csv") {
            result = await exportUtils.exportAnalysisReport(
              reportData,
              includeSyntax,
            );
          } else if (format === "pdf") {
            await exportUtils.exportToPdf(reportData);
            result = "PDF export initiated";
          }
          break;
      }

      setStatus("success");
      setMessage(`Export successful: ${result}`);
    } catch (error) {
      setStatus("error");
      setMessage(`Export failed: ${error}`);
    }
  }, [exportType, format, includeSyntax, frames, codec, width, height]);

  const handleCancel = useCallback(() => {
    if (status === "exporting") return;
    onClose();
  }, [onClose, status]);

  if (!isOpen) return null;

  return (
    <div className="export-dialog-overlay" onClick={handleCancel}>
      <div className="export-dialog" onClick={(e) => e.stopPropagation()}>
        <div className="export-dialog-header">
          <h2>Export Data</h2>
          <button
            className="export-dialog-close"
            onClick={handleCancel}
            disabled={status === "exporting"}
          >
            ✕
          </button>
        </div>

        <div className="export-dialog-body">
          {/* Export Type Selection */}
          <div className="export-dialog-section">
            <label className="export-dialog-label">Export Type:</label>
            <div className="export-dialog-options">
              <label className="export-dialog-radio">
                <input
                  type="radio"
                  name="exportType"
                  value="frames"
                  checked={exportType === "frames"}
                  onChange={(e) => setExportType(e.target.value as ExportType)}
                  disabled={status === "exporting"}
                />
                <span>Frame Data</span>
              </label>
              <label className="export-dialog-radio">
                <input
                  type="radio"
                  name="exportType"
                  value="analysis"
                  checked={exportType === "analysis"}
                  onChange={(e) => setExportType(e.target.value as ExportType)}
                  disabled={status === "exporting"}
                />
                <span>Analysis Report</span>
              </label>
              <label className="export-dialog-radio">
                <input
                  type="radio"
                  name="exportType"
                  value="report"
                  checked={exportType === "report"}
                  onChange={(e) => setExportType(e.target.value as ExportType)}
                  disabled={status === "exporting"}
                />
                <span>Full Report</span>
              </label>
            </div>
          </div>

          {/* Format Selection */}
          <div className="export-dialog-section">
            <label className="export-dialog-label">Format:</label>
            <div className="export-dialog-options">
              <label className="export-dialog-radio">
                <input
                  type="radio"
                  name="format"
                  value="csv"
                  checked={format === "csv"}
                  onChange={(e) => setFormat(e.target.value as ExportFormat)}
                  disabled={status === "exporting"}
                />
                <span>CSV</span>
              </label>
              <label className="export-dialog-radio">
                <input
                  type="radio"
                  name="format"
                  value="json"
                  checked={format === "json"}
                  onChange={(e) => setFormat(e.target.value as ExportFormat)}
                  disabled={status === "exporting"}
                />
                <span>JSON</span>
              </label>
              {(exportType === "analysis" || exportType === "report") && (
                <label className="export-dialog-radio">
                  <input
                    type="radio"
                    name="format"
                    value="txt"
                    checked={format === "txt"}
                    onChange={(e) => setFormat(e.target.value as ExportFormat)}
                    disabled={status === "exporting"}
                  />
                  <span>Text</span>
                </label>
              )}
              {(exportType === "analysis" || exportType === "report") && (
                <label className="export-dialog-radio">
                  <input
                    type="radio"
                    name="format"
                    value="pdf"
                    checked={format === "pdf"}
                    onChange={(e) => setFormat(e.target.value as ExportFormat)}
                    disabled={status === "exporting"}
                  />
                  <span>PDF</span>
                </label>
              )}
            </div>
          </div>

          {/* Options */}
          {(exportType === "analysis" || exportType === "report") && (
            <div className="export-dialog-section">
              <label className="export-dialog-checkbox">
                <input
                  type="checkbox"
                  checked={includeSyntax}
                  onChange={(e) => setIncludeSyntax(e.target.checked)}
                  disabled={status === "exporting"}
                />
                <span>Include syntax data</span>
              </label>
            </div>
          )}

          {/* Status Message */}
          {message && (
            <div
              className={`export-dialog-message export-dialog-message-${status}`}
            >
              {message}
            </div>
          )}

          {/* Frame Count Info */}
          <div className="export-dialog-info">
            <span>Frames to export: {frames.length.toLocaleString()}</span>
          </div>
        </div>

        <div className="export-dialog-footer">
          <button
            className="export-dialog-button export-dialog-button-secondary"
            onClick={handleCancel}
            disabled={status === "exporting"}
          >
            Cancel
          </button>
          <button
            className="export-dialog-button export-dialog-button-primary"
            onClick={handleExport}
            disabled={status === "exporting" || frames.length === 0}
          >
            {status === "exporting" ? "Exporting..." : "Export"}
          </button>
        </div>
      </div>
    </div>
  );
}
