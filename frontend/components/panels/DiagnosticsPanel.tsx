/**
 * Diagnostics Panel
 *
 * Error and warning list with severity indicators
 * Reference: Diagnostics
 */

import { useState, useMemo, memo, useCallback } from "react";
import { useFileState } from "../../contexts/FileStateContext";
import {
  getContextMenuItems,
  type ContextMenuItemWire,
  type BridgeDiagnostic,
} from "../../services/electronBridgeService";
import { useExportEvidenceBundle } from "../../hooks/useExportEvidenceBundle";
import { ContextMenu } from "../ContextMenu";
import "./DiagnosticsPanel.css";

export type DiagnosticSeverity = "error" | "warning" | "info" | "hint";

export interface Diagnostic {
  id: string;
  severity: DiagnosticSeverity;
  code: string;
  message: string;
  file?: string;
  line?: number;
  column?: number;
  frameIndex?: number;
  unitType?: string;
  source: string;
  timestamp: number;
}

const SEVERITY_MAP: Record<BridgeDiagnostic["severity"], DiagnosticSeverity> = {
  Info: "info",
  Warn: "warning",
  Error: "error",
  // No frontend "fatal" tier exists (DiagnosticSeverity has none) -- collapses into "error",
  // the most severe tier that does exist, rather than being silently dropped/miscategorized.
  Fatal: "error",
};

/** Real `DiagnosticAdded` events (`bitvue_engine::event::Diagnostic`, via `indexStream` --
 *  see `FileStateContext.tsx`'s `diagnostics` state) -> this panel's local `Diagnostic` shape. */
export function bridgeDiagnosticToDiagnostic(d: BridgeDiagnostic): Diagnostic {
  return {
    id: `diag-${d.id}`,
    severity: SEVERITY_MAP[d.severity],
    code: d.category,
    message: d.message,
    frameIndex: d.frame_index ?? undefined,
    source: "Indexer",
    timestamp: d.timestamp_ms,
  };
}

interface DiagnosticsPanelProps {
  diagnostics?: Diagnostic[];
}

export const DiagnosticsPanel = memo(function DiagnosticsPanel({
  diagnostics: propDiagnostics,
}: DiagnosticsPanelProps) {
  const { error } = useFileState();
  const [filterSeverity, setFilterSeverity] = useState<
    DiagnosticSeverity | "all"
  >("all");
  const [selectedDiagnostic, setSelectedDiagnostic] =
    useState<Diagnostic | null>(null);

  // Combine prop diagnostics with stream error
  const diagnostics = useMemo(() => {
    const diags: Diagnostic[] = [...(propDiagnostics || [])];

    // Add stream error if present
    if (error) {
      diags.push({
        id: "stream-error",
        severity: "error",
        code: "STREAM_ERROR",
        message: error,
        source: "Stream",
        timestamp: Date.now(),
      });
    }

    return diags;
  }, [propDiagnostics, error]);

  // Filter by severity
  const filteredDiagnostics = useMemo(() => {
    if (filterSeverity === "all") return diagnostics;
    return diagnostics.filter((d) => d.severity === filterSeverity);
  }, [diagnostics, filterSeverity]);

  // Count by severity
  const severityCounts = useMemo(() => {
    return diagnostics.reduce(
      (acc, d) => {
        acc[d.severity] = (acc[d.severity] || 0) + 1;
        return acc;
      },
      {} as Record<DiagnosticSeverity, number>,
    );
  }, [diagnostics]);

  const handleFilterSeverity = useCallback(
    (severity: DiagnosticSeverity | "all") => {
      setFilterSeverity(severity);
    },
    [],
  );

  const handleSelectDiagnostic = useCallback((diag: Diagnostic | null) => {
    setSelectedDiagnostic(diag);
  }, []);

  // Right-click context menu (Phase 7.6, "DiagnosticsPanel" scope) -- see ContextMenu component doc.
  const exportEvidence = useExportEvidenceBundle();
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    items: ContextMenuItemWire[];
  } | null>(null);

  const handleDiagnosticsContextMenu = useCallback(
    (event: React.MouseEvent) => {
      event.preventDefault();
      const hasSelection = selectedDiagnostic !== null;
      const x = event.clientX;
      const y = event.clientY;
      getContextMenuItems("DiagnosticsPanel", hasSelection, false)
        .then((items) => setContextMenu({ x, y, items }))
        .catch(() => setContextMenu(null));
    },
    [selectedDiagnostic],
  );

  const handleContextMenuSelect = useCallback(
    (command: string) => {
      if (command === "Export.EvidenceBundle") {
        void exportEvidence();
      } else if (command === "Copy.Selection" && selectedDiagnostic) {
        void navigator.clipboard.writeText(
          `${selectedDiagnostic.code}: ${selectedDiagnostic.message}`,
        );
      }
    },
    [exportEvidence, selectedDiagnostic],
  );

  const getSeverityIcon = (severity: DiagnosticSeverity) => {
    const icons = {
      error: "codicon-error",
      warning: "codicon-warning",
      info: "codicon-info",
      hint: "codicon-lightbulb",
    };
    return icons[severity];
  };

  const getSeverityColor = (severity: DiagnosticSeverity) => {
    const colors = {
      error: "var(--error-fg)",
      warning: "var(--warning-fg)",
      info: "var(--status-info)",
      hint: "var(--status-success)",
    };
    return colors[severity];
  };

  return (
    <div className="diagnostics-panel">
      {/* Header with filter */}
      <div className="diagnostics-header">
        <div className="diagnostics-title">
          <i className="codicon codicon-warning" />
          <span>Diagnostics</span>
        </div>

        <div className="diagnostics-filters">
          <button
            className={`diagnostics-filter ${filterSeverity === "all" ? "active" : ""}`}
            onClick={() => handleFilterSeverity("all")}
          >
            All ({diagnostics.length})
          </button>
          <button
            className={`diagnostics-filter diagnostics-filter-error ${filterSeverity === "error" ? "active" : ""}`}
            onClick={() => handleFilterSeverity("error")}
          >
            <i className="codicon codicon-error" />
            {severityCounts.error || 0}
          </button>
          <button
            className={`diagnostics-filter diagnostics-filter-warning ${filterSeverity === "warning" ? "active" : ""}`}
            onClick={() => handleFilterSeverity("warning")}
          >
            <i className="codicon codicon-warning" />
            {severityCounts.warning || 0}
          </button>
          <button
            className={`diagnostics-filter diagnostics-filter-info ${filterSeverity === "info" ? "active" : ""}`}
            onClick={() => handleFilterSeverity("info")}
          >
            <i className="codicon codicon-info" />
            {severityCounts.info || 0}
          </button>
        </div>
      </div>

      {/* Diagnostics table */}
      <div
        className="diagnostics-content"
        onContextMenu={handleDiagnosticsContextMenu}
      >
        {filteredDiagnostics.length === 0 ? (
          <div className="diagnostics-empty">
            <i className="codicon codicon-check" />
            <p>
              No {filterSeverity === "all" ? "" : filterSeverity} diagnostics
            </p>
          </div>
        ) : (
          <table className="diagnostics-table">
            <thead>
              <tr>
                <th className="diagnostics-col-severity">Severity</th>
                <th className="diagnostics-col-code">Code</th>
                <th className="diagnostics-col-message">Message</th>
                <th className="diagnostics-col-source">Source</th>
              </tr>
            </thead>
            <tbody>
              {filteredDiagnostics.map((diag) => (
                <tr
                  key={diag.id}
                  className={`diagnostics-row diagnostics-row-${diag.severity} ${
                    selectedDiagnostic?.id === diag.id ? "selected" : ""
                  }`}
                  onClick={() => handleSelectDiagnostic(diag)}
                >
                  <td className="diagnostics-col-severity">
                    <i
                      className={`codicon ${getSeverityIcon(diag.severity)}`}
                      style={{ color: getSeverityColor(diag.severity) }}
                    />
                  </td>
                  <td className="diagnostics-col-code">
                    <code>{diag.code}</code>
                  </td>
                  <td className="diagnostics-col-message">
                    {diag.message}
                    {diag.frameIndex !== undefined && (
                      <span className="diagnostics-frame-ref">
                        Frame {diag.frameIndex}
                      </span>
                    )}
                  </td>
                  <td className="diagnostics-col-source">{diag.source}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {/* Selected diagnostic details */}
      {selectedDiagnostic && (
        <div className="diagnostics-details">
          <div className="diagnostics-details-header">
            <span className="diagnostics-details-code">
              {selectedDiagnostic.code}
            </span>
            <button
              className="diagnostics-details-close"
              onClick={() => handleSelectDiagnostic(null)}
            >
              <i className="codicon codicon-close" />
            </button>
          </div>
          <div className="diagnostics-details-body">
            <p className="diagnostics-details-message">
              {selectedDiagnostic.message}
            </p>
            <div className="diagnostics-details-info">
              <div className="diagnostics-info-row">
                <span className="diagnostics-info-label">Severity:</span>
                <span
                  className={`diagnostics-info-value diagnostics-${selectedDiagnostic.severity}`}
                >
                  {selectedDiagnostic.severity.toUpperCase()}
                </span>
              </div>
              <div className="diagnostics-info-row">
                <span className="diagnostics-info-label">Source:</span>
                <span className="diagnostics-info-value">
                  {selectedDiagnostic.source}
                </span>
              </div>
              {selectedDiagnostic.frameIndex !== undefined && (
                <div className="diagnostics-info-row">
                  <span className="diagnostics-info-label">Frame:</span>
                  <span className="diagnostics-info-value">
                    {selectedDiagnostic.frameIndex}
                  </span>
                </div>
              )}
              {selectedDiagnostic.unitType && (
                <div className="diagnostics-info-row">
                  <span className="diagnostics-info-label">Unit Type:</span>
                  <span className="diagnostics-info-value">
                    {selectedDiagnostic.unitType}
                  </span>
                </div>
              )}
            </div>
          </div>
        </div>
      )}

      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          items={contextMenu.items}
          onSelect={handleContextMenuSelect}
          onClose={() => setContextMenu(null)}
        />
      )}
    </div>
  );
});
