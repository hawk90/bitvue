/**
 * Selection Info Panel
 *
 * Current frame and selection information
 * Features:
 * - Frame information display
 * - Stream statistics
 * - Current selection analysis
 */

import { useStreamData } from '../../contexts/StreamDataContext';
import { memo } from 'react';
import './SelectionInfoPanel.css';

interface SectionProps {
  title: string;
  children: React.ReactNode;
}

function InfoSection({ title, children }: SectionProps) {
  return (
    <div className="info-section">
      <div className="info-section-title">{title}</div>
      <div className="info-section-content">{children}</div>
    </div>
  );
}

interface InfoRowProps {
  label: string;
  value: React.ReactNode;
  highlight?: boolean;
}

function InfoRow({ label, value, highlight }: InfoRowProps) {
  return (
    <div className={`info-row ${highlight ? 'highlight' : ''}`}>
      <span className="info-label">{label}:</span>
      <span className="info-value">{value}</span>
    </div>
  );
}

interface SelectionInfoPanelProps {
  width?: number;
  height?: number;
  codec?: string;
}

export const SelectionInfoPanel = memo(function SelectionInfoPanel({
  width = 1920,
  height = 1080,
  codec = 'AV1',
}: SelectionInfoPanelProps) {
  const { frames, currentFrameIndex, getFrameStats } = useStreamData();
  const stats = getFrameStats();
  const currentFrame = frames[currentFrameIndex] || null;

  return (
    <div className="selection-info-panel">
      <div className="panel-header">
        <span className="panel-title">Selection Info</span>
      </div>

      <div className="selection-info-content">
        {/* Current Frame Section */}
        <InfoSection title="Current Frame">
          <InfoRow label="Frame Index" value={currentFrame?.frame_index ?? 'N/A'} />
          <InfoRow
            label="Frame Type"
            value={
              currentFrame ? (
                <span className={`frame-type-badge frame-type-${currentFrame.frame_type.toLowerCase()}`}>
                  {currentFrame.frame_type}
                </span>
              ) : 'N/A'
            }
          />
          <InfoRow
            label="Size"
            value={currentFrame ? `${(currentFrame.size / 1024).toFixed(2)} KB` : 'N/A'}
          />
          {currentFrame?.pts !== undefined && (
            <InfoRow label="PTS" value={currentFrame.pts} />
          )}
          {currentFrame?.temporal_id !== undefined && (
            <InfoRow label="Temporal ID" value={currentFrame.temporal_id} />
          )}
          <InfoRow
            label="References"
            value={currentFrame?.ref_frames?.length || 0}
          />
        </InfoSection>

        {/* Video Properties Section */}
        <InfoSection title="Video Properties">
          <InfoRow label="Resolution" value={`${width}x${height}`} />
          <InfoRow label="Codec" value={codec} />
          <InfoRow label="Color Format" value="4:2:0" />
          <InfoRow label="Bit Depth" value="8-bit" />
          <InfoRow label="Frame Rate" value="30 fps" />
        </InfoSection>

        {/* Stream Statistics Section */}
        <InfoSection title="Stream Statistics">
          <InfoRow label="Total Frames" value={stats.totalFrames} highlight />
          <InfoRow label="Keyframes" value={stats.keyFrames} highlight />
          <InfoRow label="Avg Size" value={`${(stats.avgSize / 1024).toFixed(2)} KB`} />
          <InfoRow label="Total Size" value={`${(stats.totalSize / 1024 / 1024).toFixed(2)} MB`} />
        </InfoSection>

        {/* Frame Type Distribution */}
        <InfoSection title="Frame Types">
          {Object.entries(stats.frameTypes).map(([type, count]) => (
            <InfoRow
              key={type}
              label={type}
              value={
                <span className="frame-type-count">
                  <span className={`frame-type-badge frame-type-${type.toLowerCase()}`}>
                    {type}
                  </span>
                  <span className="frame-count">{count}</span>
                  <span className="frame-percent">
                    ({((count / stats.totalFrames) * 100).toFixed(1)}%)
                  </span>
                </span>
              }
            />
          ))}
        </InfoSection>

        {/* Current Selection Section */}
        <InfoSection title="Selection">
          <div className="selection-empty">
            <span className="codicon codicon-location"></span>
            <span>Click on the video to select a block</span>
          </div>
        </InfoSection>

        {/* Codec Badge */}
        {codec && (
          <div className="codec-info">
            <span className="codec-badge">{codec}</span>
          </div>
        )}
      </div>
    </div>
  );
});
