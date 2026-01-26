/**
 * Diagnostics Panel
 *
 * Error and warning list with severity indicators
 * Reference: VQAnalyzer Diagnostics
 */

import { useState, useMemo, memo, useCallback } from 'react';
import { useStreamData } from '../../contexts/StreamDataContext';
import './DiagnosticsPanel.css';

export type DiagnosticSeverity = 'error' | 'warning' | 'info' | 'hint';

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

interface DiagnosticsPanelProps {
  diagnostics?: Diagnostic[];
}

export const DiagnosticsPanel = memo(function DiagnosticsPanel({ diagnostics: propDiagnostics }: DiagnosticsPanelProps) {
  const { frames, currentFrameIndex, error } = useStreamData();
  const [filterSeverity, setFilterSeverity] = useState<DiagnosticSeverity | 'all'>('all');
  const [selectedDiagnostic, setSelectedDiagnostic] = useState<Diagnostic | null>(null);

  // Combine prop diagnostics with stream error
  const diagnostics = useMemo(() => {
    const diags: Diagnostic[] = [...(propDiagnostics || [])];

    // Add stream error if present
    if (error) {
      diags.push({
        id: 'stream-error',
        severity: 'error',
        code: 'STREAM_ERROR',
        message: error,
        source: 'Stream',
        timestamp: Date.now(),
      });
    }

    // Add mock diagnostics for demonstration
    if (diags.length === 0 && frames.length > 0) {
      // Simulate some potential issues
      const frame = frames[currentFrameIndex];
      if (frame) {
        // Check for large frames (potential quality issue)
        if (frame.size > 100000) {
          diags.push({
            id: `frame-${frame.frame_index}-size`,
            severity: 'warning',
            code: 'LARGE_FRAME',
            message: `Frame ${frame.frame_index} is unusually large (${(frame.size / 1024).toFixed(1)} KB)`,
            frameIndex: frame.frame_index,
            source: 'FrameAnalyzer',
            timestamp: Date.now(),
          });
        }

        // Check for missing references (should have refs for P/B frames)
        if ((frame.frame_type === 'P' || frame.frame_type === 'B') && !frame.ref_frames?.length) {
          diags.push({
            id: `frame-${frame.frame_index}-no-refs`,
            severity: 'info',
            code: 'NO_REFERENCES',
            message: `Frame ${frame.frame_index} (${frame.frame_type}) has no reference frames`,
            frameIndex: frame.frame_index,
            source: 'FrameAnalyzer',
            timestamp: Date.now(),
          });
        }
      }
    }

    return diags;
  }, [propDiagnostics, error, frames, currentFrameIndex]);

  // Filter by severity
  const filteredDiagnostics = useMemo(() => {
    if (filterSeverity === 'all') return diagnostics;
    return diagnostics.filter(d => d.severity === filterSeverity);
  }, [diagnostics, filterSeverity]);

  // Count by severity
  const severityCounts = useMemo(() => {
    return diagnostics.reduce((acc, d) => {
      acc[d.severity] = (acc[d.severity] || 0) + 1;
      return acc;
    }, {} as Record<DiagnosticSeverity, number>);
  }, [diagnostics]);

  const handleFilterSeverity = useCallback((severity: DiagnosticSeverity | 'all') => {
    setFilterSeverity(severity);
  }, []);

  const handleSelectDiagnostic = useCallback((diag: Diagnostic | null) => {
    setSelectedDiagnostic(diag);
  }, []);

  const getSeverityIcon = (severity: DiagnosticSeverity) => {
    const icons = {
      error: 'codicon-error',
      warning: 'codicon-warning',
      info: 'codicon-info',
      hint: 'codicon-lightbulb',
    };
    return icons[severity];
  };

  const getSeverityColor = (severity: DiagnosticSeverity) => {
    const colors = {
      error: 'var(--error-fg)',
      warning: 'var(--warning-fg)',
      info: 'var(--status-info)',
      hint: 'var(--status-success)',
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
            className={`diagnostics-filter ${filterSeverity === 'all' ? 'active' : ''}`}
            onClick={() => handleFilterSeverity('all')}
          >
            All ({diagnostics.length})
          </button>
          <button
            className={`diagnostics-filter diagnostics-filter-error ${filterSeverity === 'error' ? 'active' : ''}`}
            onClick={() => handleFilterSeverity('error')}
          >
            <i className="codicon codicon-error" />
            {severityCounts.error || 0}
          </button>
          <button
            className={`diagnostics-filter diagnostics-filter-warning ${filterSeverity === 'warning' ? 'active' : ''}`}
            onClick={() => handleFilterSeverity('warning')}
          >
            <i className="codicon codicon-warning" />
            {severityCounts.warning || 0}
          </button>
          <button
            className={`diagnostics-filter diagnostics-filter-info ${filterSeverity === 'info' ? 'active' : ''}`}
            onClick={() => handleFilterSeverity('info')}
          >
            <i className="codicon codicon-info" />
            {severityCounts.info || 0}
          </button>
        </div>
      </div>

      {/* Diagnostics table */}
      <div className="diagnostics-content">
        {filteredDiagnostics.length === 0 ? (
          <div className="diagnostics-empty">
            <i className="codicon codicon-check" />
            <p>No {filterSeverity === 'all' ? '' : filterSeverity} diagnostics</p>
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
                    selectedDiagnostic?.id === diag.id ? 'selected' : ''
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
                      <span className="diagnostics-frame-ref">Frame {diag.frameIndex}</span>
                    )}
                  </td>
                  <td className="diagnostics-col-source">
                    {diag.source}
                  </td>
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
            <span className="diagnostics-details-code">{selectedDiagnostic.code}</span>
            <button
              className="diagnostics-details-close"
              onClick={() => handleSelectDiagnostic(null)}
            >
              <i className="codicon codicon-close" />
            </button>
          </div>
          <div className="diagnostics-details-body">
            <p className="diagnostics-details-message">{selectedDiagnostic.message}</p>
            <div className="diagnostics-details-info">
              <div className="diagnostics-info-row">
                <span className="diagnostics-info-label">Severity:</span>
                <span className={`diagnostics-info-value diagnostics-${selectedDiagnostic.severity}`}>
                  {selectedDiagnostic.severity.toUpperCase()}
                </span>
              </div>
              <div className="diagnostics-info-row">
                <span className="diagnostics-info-label">Source:</span>
                <span className="diagnostics-info-value">{selectedDiagnostic.source}</span>
              </div>
              {selectedDiagnostic.frameIndex !== undefined && (
                <div className="diagnostics-info-row">
                  <span className="diagnostics-info-label">Frame:</span>
                  <span className="diagnostics-info-value">{selectedDiagnostic.frameIndex}</span>
                </div>
              )}
              {selectedDiagnostic.unitType && (
                <div className="diagnostics-info-row">
                  <span className="diagnostics-info-label">Unit Type:</span>
                  <span className="diagnostics-info-value">{selectedDiagnostic.unitType}</span>
                </div>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
});
