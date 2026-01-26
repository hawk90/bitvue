/**
 * Enhanced View Component
 *
 * Bitvue-exclusive multi-metric visualization with:
 * 1. GOP boundary markers overlay
 * 2. Scene change detection
 * 3. Smart navigation buttons
 * 4. Diagnostic severity heatmap
 */

import { memo, useMemo, useEffect, useState, useCallback } from 'react';
import type { FrameInfo } from '../../types/video';
import ThumbnailsView from './ThumbnailsView';
import { getFrameTypeColorClass } from '../../types/video';

interface EnhancedViewProps {
  frames: FrameInfo[];
  currentFrameIndex: number;
  thumbnails: Map<number, string>;
  loadingThumbnails: Set<number>;
  onFrameClick: (frameIndex: number) => void;
  onHoverFrame: (frame: FrameInfo | null, x: number, y: number) => void;
}

interface GOPBoundary {
  frameIndex: number;
  gopNumber: number;
  frameCount: number;
}

interface SceneChange {
  frameIndex: number;
  severity: number; // 0-1
}

export const EnhancedView = memo(function EnhancedView({
  frames,
  currentFrameIndex,
  thumbnails,
  loadingThumbnails,
  onFrameClick,
  onHoverFrame,
}: EnhancedViewProps) {
  const [gopBoundaries, setGopBoundaries] = useState<GOPBoundary[]>([]);
  const [sceneChanges, setSceneChanges] = useState<SceneChange[]>([]);
  const [hoveredFrame, setHoveredFrame] = useState<number | null>(null);

  // Analyze GOP structure
  useEffect(() => {
    if (frames.length === 0) {
      setGopBoundaries([]);
      return;
    }

    const boundaries: GOPBoundary[] = [];
    let currentGopStart = 0;
    let gopNumber = 1;

    for (let i = 0; i < frames.length; i++) {
      if (frames[i].frameType === 'I' && i > 0) {
        boundaries.push({
          frameIndex: currentGopStart,
          gopNumber,
          frameCount: i - currentGopStart,
        });
        currentGopStart = i;
        gopNumber++;
      }
    }

    // Add last GOP
    if (currentGopStart < frames.length) {
      boundaries.push({
        frameIndex: currentGopStart,
        gopNumber,
        frameCount: frames.length - currentGopStart,
      });
    }

    setGopBoundaries(boundaries);
  }, [frames]);

  // Detect scene changes (mock implementation using frame size variance)
  useEffect(() => {
    if (frames.length === 0) {
      setSceneChanges([]);
      return;
    }

    const changes: SceneChange[] = [];
    const windowSize = 10;

    for (let i = windowSize; i < frames.length - windowSize; i++) {
      const beforeWindow = frames.slice(i - windowSize, i);
      const afterWindow = frames.slice(i, i + windowSize);

      const avgBefore = beforeWindow.reduce((sum, f) => sum + (f.size || 0), 0) / windowSize;
      const avgAfter = afterWindow.reduce((sum, f) => sum + (f.size || 0), 0) / windowSize;

      const variance = Math.abs(avgAfter - avgBefore) / Math.max(avgBefore, avgAfter);

      if (variance > 0.5) {
        changes.push({
          frameIndex: i,
          severity: Math.min(variance, 1),
        });
      }
    }

    setSceneChanges(changes);
  }, [frames]);

  const currentGOP = useMemo(() => {
    return gopBoundaries.find(
      (b, idx) =>
        b.frameIndex <= currentFrameIndex &&
        (idx === gopBoundaries.length - 1 || gopBoundaries[idx + 1].frameIndex > currentFrameIndex)
    );
  }, [gopBoundaries, currentFrameIndex]);

  const navigateToGOP = useCallback((direction: 'prev' | 'next') => {
    if (!currentGOP) return;

    const currentIndex = gopBoundaries.indexOf(currentGOP);
    if (direction === 'prev' && currentIndex > 0) {
      onFrameClick(gopBoundaries[currentIndex - 1].frameIndex);
    } else if (direction === 'next' && currentIndex < gopBoundaries.length - 1) {
      onFrameClick(gopBoundaries[currentIndex + 1].frameIndex);
    }
  }, [currentGOP, gopBoundaries, onFrameClick]);

  const navigateToSceneChange = useCallback((direction: 'prev' | 'next') => {
    if (sceneChanges.length === 0) return;

    const currentIndex = sceneChanges.findIndex(c => c.frameIndex >= currentFrameIndex);
    if (direction === 'prev') {
      const prevIndex = currentIndex <= 0 ? sceneChanges.length - 1 : currentIndex - 1;
      onFrameClick(sceneChanges[prevIndex].frameIndex);
    } else {
      const nextIndex = currentIndex < 0 ? 0 :
        currentIndex >= sceneChanges.length - 1 ? 0 : currentIndex + 1;
      onFrameClick(sceneChanges[nextIndex].frameIndex);
    }
  }, [sceneChanges, currentFrameIndex, onFrameClick]);

  const getDiagnosticColor = (frameIndex: number) => {
    const frame = frames[frameIndex];
    if (!frame) return '';

    // Check for various diagnostic conditions
    const issues = [];

    if (frame.qp && frame.qp > 45) issues.push('high-qp');
    if (frame.size && frame.size > 100000) issues.push('large-frame');
    if (frame.frameType === 'B' && !frame.refFrames?.length) issues.push('no-reference');

    if (issues.length === 0) return '';
    return `diagnostic-${issues[0]}`;
  };

  return (
    <div className="enhanced-view">
      {/* Smart Navigation Bar */}
      <div className="enhanced-nav-bar">
        <div className="enhanced-nav-section">
          <span className="enhanced-nav-label">GOP:</span>
          <button
            className="enhanced-nav-btn"
            onClick={() => navigateToGOP('prev')}
            disabled={!currentGOP || gopBoundaries.indexOf(currentGOP) === 0}
            title="Previous GOP"
          >
            <span className="codicon codicon-arrow-left" aria-hidden="true"></span>
            <span>I</span>
          </button>
          <span className="enhanced-nav-info">
            {currentGOP ? `GOP ${currentGOP.gopNumber}/${currentGOP.frameCount}f` : '-'}
          </span>
          <button
            className="enhanced-nav-btn"
            onClick={() => navigateToGOP('next')}
            disabled={!currentGOP || gopBoundaries.indexOf(currentGOP) === gopBoundaries.length - 1}
            title="Next GOP"
          >
            <span>I</span>
            <span className="codicon codicon-arrow-right" aria-hidden="true"></span>
          </button>
        </div>

        <div className="enhanced-nav-separator"></div>

        <div className="enhanced-nav-section">
          <span className="enhanced-nav-label">Scene:</span>
          <button
            className="enhanced-nav-btn"
            onClick={() => navigateToSceneChange('prev')}
            disabled={sceneChanges.length === 0}
            title="Previous Scene Change"
          >
            <span className="codicon codicon-arrow-left" aria-hidden="true"></span>
            <span>SC</span>
          </button>
          <span className="enhanced-nav-info">{sceneChanges.length} changes</span>
          <button
            className="enhanced-nav-btn"
            onClick={() => navigateToSceneChange('next')}
            disabled={sceneChanges.length === 0}
            title="Next Scene Change"
          >
            <span>SC</span>
            <span className="codicon codicon-arrow-right" aria-hidden="true"></span>
          </button>
        </div>
      </div>

      {/* Enhanced Thumbnails with overlays */}
      <div className="enhanced-thumbnails-container">
        <ThumbnailsView
          frames={frames}
          currentFrameIndex={currentFrameIndex}
          thumbnails={thumbnails}
          loadingThumbnails={loadingThumbnails}
          referencedFrameIndices={new Set()}
          expandedFrameIndex={null}
          onFrameClick={onFrameClick}
          onToggleReferenceExpansion={() => {}}
          onHoverFrame={(frame, x, y) => {
            setHoveredFrame(frame?.frameNumber ?? null);
            onHoverFrame(frame, x, y);
          }}
          getFrameTypeColorClass={getFrameTypeColorClass}
        />

        {/* GOP Boundary Markers */}
        <div className="enhanced-overlays">
          {gopBoundaries.map((boundary) => (
            <div
              key={`gop-${boundary.frameIndex}`}
              className="enhanced-gop-marker"
              style={{
                left: `${(boundary.frameIndex / frames.length) * 100}%`,
              }}
              title={`GOP ${boundary.gopNumber}: ${boundary.frameCount} frames`}
            >
              <div className="enhanced-gop-line"></div>
              <div className="enhanced-gop-label">G{boundary.gopNumber}</div>
            </div>
          ))}

          {/* Scene Change Markers */}
          {sceneChanges.map((change) => (
            <div
              key={`scene-${change.frameIndex}`}
              className="enhanced-scene-marker"
              style={{
                left: `${(change.frameIndex / frames.length) * 100}%`,
                opacity: 0.5 + change.severity * 0.5,
              }}
              title={`Scene change (${(change.severity * 100).toFixed(0)}% severity)`}
            >
              <div className="enhanced-scene-icon">⚡</div>
            </div>
          ))}
        </div>
      </div>

      {/* Diagnostic Severity Heatmap (below thumbnails) */}
      <div className="enhanced-heatmap">
        {frames.map((frame, index) => {
          const diagnosticClass = getDiagnosticColor(index);
          const sceneChange = sceneChanges.find(c => c.frameIndex === index);
          const isGOPStart = gopBoundaries.some(b => b.frameIndex === index);

          return (
            <div
              key={`heatmap-${index}`}
              className={`enhanced-heatmap-cell ${diagnosticClass} ${
                isGOPStart ? 'heatmap-gop-start' : ''
              } ${sceneChange ? 'heatmap-scene-change' : ''}`}
              style={{
                width: `${100 / frames.length}%`,
              }}
              title={`Frame ${index}${diagnosticClass ? ` (${diagnosticClass})` : ''}${
                sceneChange ? ` - Scene change` : ''
              }`}
            />
          );
        })}
      </div>

      {/* Legend */}
      <div className="enhanced-legend">
        <div className="enhanced-legend-item">
          <div className="enhanced-legend-box enhanced-gop-marker"></div>
          <span>GOP Start</span>
        </div>
        <div className="enhanced-legend-item">
          <div className="enhanced-legend-box enhanced-scene-marker"></div>
          <span>Scene Change</span>
        </div>
        <div className="enhanced-legend-item">
          <div className="enhanced-legend-box heatmap-gop-start"></div>
          <span>I-Frame</span>
        </div>
        <div className="enhanced-legend-item">
          <div className="enhanced-legend-box diagnostic-high-qp"></div>
          <span>High QP</span>
        </div>
        <div className="enhanced-legend-item">
          <div className="enhanced-legend-box diagnostic-large-frame"></div>
          <span>Large Frame</span>
        </div>
      </div>

      {/* Hover Info */}
      {hoveredFrame !== null && frames[hoveredFrame] && (
        <div className="enhanced-hover-info">
          <div className="enhanced-hover-section">
            <span className="enhanced-hover-label">Frame:</span>
            <span className="enhanced-hover-value">{hoveredFrame}</span>
          </div>
          <div className="enhanced-hover-section">
            <span className="enhanced-hover-label">Type:</span>
            <span className={`enhanced-hover-value ${getFrameTypeColorClass(frames[hoveredFrame].frameType)}`}>
              {frames[hoveredFrame].frameType}
            </span>
          </div>
          {frames[hoveredFrame].size && (
            <div className="enhanced-hover-section">
              <span className="enhanced-hover-label">Size:</span>
              <span className="enhanced-hover-value">{frames[hoveredFrame].size?.toLocaleString()} bytes</span>
            </div>
          )}
          {currentGOP && hoveredFrame >= currentGOP.frameIndex && (
            <div className="enhanced-hover-section">
              <span className="enhanced-hover-label">GOP:</span>
              <span className="enhanced-hover-value">{currentGOP.gopNumber}</span>
            </div>
          )}
        </div>
      )}
    </div>
  );
});
