/**
 * Stream Player - Single stream player for compare view
 *
 * Displays a single video stream with frame navigation.
 */

import { memo, useMemo } from 'react';
import { type FrameInfo, AlignmentQuality } from '../../types/video';
import { VideoCanvas } from '../panels/YuvViewerPanel/VideoCanvas';
import { FrameNavigationControls } from '../panels/YuvViewerPanel/FrameNavigationControls';
import './StreamPlayer.css';

interface StreamPlayerProps {
  frames: FrameInfo[];
  currentFrame: number;
  onFrameChange: (index: number) => void;
  streamLabel: 'A' | 'B';
  alignedFrame?: number | null;
  alignmentQuality?: AlignmentQuality;
}

function StreamPlayer({
  frames,
  currentFrame,
  onFrameChange,
  streamLabel,
  alignedFrame,
  alignmentQuality,
}: StreamPlayerProps) {
  const currentFrameData = frames[currentFrame] || null;

  // Memoize alignment color function - it's recreated on every render otherwise
  const getAlignmentColor = useMemo(() => (quality?: AlignmentQuality): string => {
    switch (quality) {
      case AlignmentQuality.Exact:
        return 'var(--color-success)';
      case AlignmentQuality.Nearest:
        return 'var(--color-warning)';
      case AlignmentQuality.Gap:
        return 'var(--color-error)';
      default:
        return 'var(--color-text-secondary)';
    }
  }, []);

  return (
    <div className={`stream-player stream-${streamLabel.toLowerCase()}`}>
      {/* Frame display */}
      <div className="player-viewport">
        {currentFrameData ? (
          <VideoCanvas
            width={currentFrameData.width || 1920}
            height={currentFrameData.height || 1080}
            frameData={currentFrameData}
          />
        ) : (
          <div className="player-placeholder">
            <span>No frame data</span>
          </div>
        )}

        {/* Alignment indicator for stream B */}
        {streamLabel === 'B' && alignmentQuality !== undefined && (
          <div
            className="alignment-indicator"
            style={{ borderColor: getAlignmentColor(alignmentQuality) }}
            title={`Alignment: ${alignmentQuality}`}
          >
            <span
              className="alignment-dot"
              style={{ backgroundColor: getAlignmentColor(alignmentQuality) }}
            />
            {alignmentQuality}
          </div>
        )}

        {/* Frame info overlay */}
        {currentFrameData && (
          <div className="frame-overlay">
            <span className="frame-number">
              {streamLabel}: {currentFrame + 1} / {frames.length}
            </span>
            {currentFrameData.frame_type && (
              <span className={`frame-type frame-${currentFrameData.frame_type.toLowerCase()}`}>
                {currentFrameData.frame_type}
              </span>
            )}
            {currentFrameData.size && (
              <span className="frame-size">
                {(currentFrameData.size / 1024).toFixed(1)} KB
              </span>
            )}
          </div>
        )}
      </div>

      {/* Frame navigation */}
      <div className="player-controls">
        <FrameNavigationControls
          currentFrame={currentFrame}
          totalFrames={frames.length}
          onFrameChange={onFrameChange}
          compact
        />
      </div>
    </div>
  );
}

export default memo(StreamPlayer);
