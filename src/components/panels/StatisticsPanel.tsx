/**
 * Statistics Panel
 *
 * Stream statistics and metrics
 * Reference: VQAnalyzer Statistics
 */

import { useMemo, memo } from 'react';
import { useStreamData } from '../../contexts/StreamDataContext';
import { BarChart } from '../charts/BarChart';
import { getCssVar } from '../../utils/css';
import './StatisticsPanel.css';

export const StatisticsPanel = memo(function StatisticsPanel() {
  const { frames, getFrameStats } = useStreamData();

  const stats = useMemo(() => getFrameStats(), [frames, getFrameStats]);

  // Calculate frame size distribution for histogram
  const frameSizes = useMemo(() => {
    if (frames.length === 0) return {};

    // Group by size ranges
    const ranges = {
      '< 10KB': 0,
      '10-50KB': 0,
      '50-100KB': 0,
      '100-500KB': 0,
      '> 500KB': 0,
    };

    frames.forEach(frame => {
      const sizeKB = frame.size / 1024;
      if (sizeKB < 10) ranges['< 10KB']++;
      else if (sizeKB < 50) ranges['10-50KB']++;
      else if (sizeKB < 100) ranges['50-100KB']++;
      else if (sizeKB < 500) ranges['100-500KB']++;
      else ranges['> 500KB']++;
    });

    return ranges;
  }, [frames]);

  const maxSizeRange = Math.max(...Object.values(frameSizes) as number[]);

  return (
    <div className="statistics-panel">
      <div className="panel-header">
        <span className="panel-title">Statistics</span>
      </div>

      <div className="statistics-content">
        {/* Frame Type Distribution */}
        <section className="stats-section">
          <h3 className="stats-section-title">Frame Types</h3>
          <div className="stats-section-content">
            <BarChart
              data={stats.frameTypes}
              maxValue={Math.max(...Object.values(stats.frameTypes))}
            />
            <div className="stats-summary">
              <span className="stats-summary-item">
                <span className="stats-label">Total:</span>
                <span className="stats-value">{stats.totalFrames}</span>
              </span>
              <span className="stats-summary-item">
                <span className="stats-label">Keyframes:</span>
                <span className="stats-value">{stats.keyFrames}</span>
              </span>
              <span className="stats-summary-item">
                <span className="stats-label">Avg Size:</span>
                <span className="stats-value">{(stats.avgSize / 1024).toFixed(2)} KB</span>
              </span>
            </div>
          </div>
        </section>

        {/* Frame Size Distribution */}
        <section className="stats-section">
          <h3 className="stats-section-title">Frame Sizes</h3>
          <div className="stats-section-content">
            <BarChart
              data={frameSizes}
              maxValue={maxSizeRange}
              colors={{
                '< 10KB': getCssVar('--color-info'),
                '10-50KB': getCssVar('--frame-p'),
                '50-100KB': getCssVar('--color-success'),
                '100-500KB': getCssVar('--accent-primary-light'),
                '> 500KB': getCssVar('--frame-b'),
              }}
            />
          </div>
        </section>

        {/* Bitrate Info */}
        <section className="stats-section">
          <h3 className="stats-section-title">Bitrate</h3>
          <div className="stats-section-content">
            <div className="bitrate-info">
              <div className="bitrate-label">Total Size:</div>
              <div className="bitrate-value">{(stats.totalSize / 1024 / 1024).toFixed(2)} MB</div>
            </div>
            <div className="bitrate-info">
              <div className="bitrate-label">Avg Bitrate:</div>
              <div className="bitrate-value">
                {frames.length > 0 ? ((stats.totalSize / frames.length) * 30 / 1000).toFixed(2) : '0'} Mbps
              </div>
            </div>
            <div className="bitrate-note">Assuming 30fps</div>
          </div>
        </section>
      </div>
    </div>
  );
});
