/**
 * Filmstrip Component
 *
 * Main filmstrip component that orchestrates different view modes
 * Refactored to use useFilmstripState hook for better separation of concerns
 */

import { useState, useCallback, useEffect, useRef, useMemo, memo } from 'react';
import { createPortal } from 'react-dom';
import { useSelection } from '../contexts/SelectionContext';
import { FilmstripDropdown, DisplayView } from './FilmstripDropdown';
import { FrameSizesLegend } from './FrameSizesLegend';
import ThumbnailsView from './Filmstrip/views/ThumbnailsView';
import VirtualizedThumbnailsView from './Filmstrip/views/VirtualizedThumbnailsView';
import FrameSizesView from './Filmstrip/views/FrameSizesView';
import BPyramidView from './Filmstrip/views/BPyramidView';
import { TimelineView } from './Filmstrip/views/TimelineView';
import { MinimapView } from './MinimapView';
import { FilmstripTooltip } from './FilmstripTooltip';
import { useFilmstripState } from './useFilmstripState';
import { getFrameTypeColorClass, getFrameTypeColor, type FrameInfo } from '../types/video';
import './Filmstrip/Filmstrip.css';

interface FilmstripProps {
  frames: FrameInfo[];
  className?: string;
  viewMode?: 'overview' | 'coding' | 'prediction' | 'transform' | 'qp' | 'mv' | 'reference';
  onViewModeChange?: (mode: 'overview' | 'coding' | 'prediction' | 'transform' | 'qp' | 'mv' | 'reference') => void;
}

function Filmstrip({ frames, className = '', viewMode: _viewMode = 'overview', onViewModeChange: _onViewModeChange }: FilmstripProps) {
  const { selection, setFrameSelection } = useSelection();
  const [displayView, setDisplayView] = useState<DisplayView>('thumbnails');
  const [scrollRef, setScrollRef] = useState<HTMLDivElement | null>(null);
  const [showingReferences, setShowingReferences] = useState(false);

  // Threshold for using virtualized view (200 frames = ~30KB DOM)
  const VIRTUALIZATION_THRESHOLD = 200;
  const useVirtualizedView = frames.length >= VIRTUALIZATION_THRESHOLD;

  const visibleFrameTypes = useMemo(() => new Set(['I', 'P', 'B']), []);

  const [sizeMetrics, setSizeMetrics] = useState({
    showBitrateBar: true,
    showBitrateCurve: false,
    showAvgSize: false,
    showMinSize: false,
    showMaxSize: false,
    showMovingAvg: false,
    showBlockMinQP: false,
    showBlockMaxQP: false,
  });

  const filmstripContentRef = useRef<HTMLDivElement>(null);

  const currentFrameIndex = selection?.frame?.frameIndex ?? 0;

  // Use extracted state management hook
  const {
    thumbnails,
    loadingThumbnails,
    expandedFrameIndex,
    hoveredFrame,
    mousePosition,
    maxSize,
    loadThumbnails,
    handleToggleExpansion,
    handleHoverFrame,
  } = useFilmstripState({
    frames,
    displayView,
  });

  // Handle reference expansion display
  const shouldShowReferences = useMemo(() => {
    const currentFrame = frames[currentFrameIndex];
    return currentFrame?.ref_frames && currentFrame.ref_frames.length > 0;
  }, [currentFrameIndex, frames]);

  useEffect(() => {
    if (shouldShowReferences) {
      setShowingReferences(true);
      const timer = setTimeout(() => setShowingReferences(false), 50);
      return () => clearTimeout(timer);
    } else {
      setShowingReferences(false);
    }
  }, [shouldShowReferences, currentFrameIndex]);

  const effectiveExpandedIndex = shouldShowReferences ? currentFrameIndex : expandedFrameIndex;

  // Setup IntersectionObserver for lazy loading thumbnails (only for non-virtualized view)
  useEffect(() => {
    // Virtualized view handles its own visibility-based loading
    if (useVirtualizedView || displayView !== 'thumbnails' || frames.length === 0 || !scrollRef) return;

    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach(entry => {
          if (entry.isIntersecting) {
            const frameIndex = parseInt(entry.target.getAttribute('data-frame-index') || '0', 10);
            if (!isNaN(frameIndex)) {
              loadThumbnails([frameIndex]);
            }
          }
        });
      },
      { root: scrollRef, rootMargin: '200px' }
    );

    const frameElements = scrollRef.querySelectorAll('[data-frame-index]');
    frameElements.forEach(el => observer.observe(el));

    return () => {
      observer.disconnect();
    };
  }, [displayView, frames.length, scrollRef, loadThumbnails]);

  // Auto-scroll to current frame
  useEffect(() => {
    if (scrollRef && frames.length > 0) {
      const frameEl = scrollRef.querySelector(`[data-frame-index="${currentFrameIndex}"]`);
      if (frameEl) {
        frameEl.scrollIntoView({ behavior: 'smooth', block: 'nearest', inline: 'center' });
      }
    }
  }, [currentFrameIndex, scrollRef, frames.length, displayView]);

  const handleFrameClick = useCallback((frameIndex: number) => {
    setFrameSelection({ stream: 'A', frameIndex }, 'filmstrip');
  }, [setFrameSelection]);

  const toggleMetric = useCallback((metric: keyof typeof sizeMetrics) => {
    setSizeMetrics(prev => ({
      ...prev,
      [metric]: !prev[metric],
    }));
  }, []);

  // Calculate referenced frame indices for highlighting
  const referencedFrameIndices = useMemo(() => {
    if (effectiveExpandedIndex === null) return new Set<number>();
    const frame = frames[effectiveExpandedIndex];
    if (!frame?.ref_frames) return new Set<number>();
    return new Set(frame.ref_frames);
  }, [effectiveExpandedIndex, frames]);

  return (
    <div className={`filmstrip ${className} ${showingReferences ? 'showing-references' : ''} ${displayView === 'timeline' ? 'timeline-view-mode' : ''}`}>
      {/* Filmstrip Header */}
      <div className="filmstrip-header" role="region" aria-label="Filmstrip controls">
        <div className="filmstrip-title">
          <span className="codicon codicon-film" aria-hidden="true"></span>
          Filmstrip
          <span className="filmstrip-count">({frames.length} frames)</span>
        </div>

        <div className="filmstrip-info" role="status" aria-live="polite">
          {frames.length > 0 && (
            <span className="filmstrip-current">Frame {currentFrameIndex}</span>
          )}

          <FilmstripDropdown displayView={displayView} onViewChange={setDisplayView} />
        </div>
      </div>

      {/* Filmstrip Content */}
      <div
        className={`filmstrip-content ${showingReferences ? 'showing-reference-lines' : ''}`}
        ref={(el) => {
          setScrollRef(el);
          (filmstripContentRef as React.MutableRefObject<HTMLDivElement | null>).current = el;
        }}
        role="region"
        aria-label="Filmstrip content"
        aria-live="polite"
      >
        {frames.length === 0 ? (
          <div className="filmstrip-empty" role="status" aria-label="No frames loaded">
            <span className="codicon codicon-film-strip" aria-hidden="true"></span>
            <p>No frames loaded</p>
            <p className="hint">Open a video file to see the filmstrip</p>
          </div>
        ) : displayView === 'thumbnails' ? (
          useVirtualizedView ? (
            <VirtualizedThumbnailsView
              frames={frames}
              currentFrameIndex={currentFrameIndex}
              thumbnails={thumbnails}
              loadingThumbnails={loadingThumbnails}
              referencedFrameIndices={referencedFrameIndices}
              expandedFrameIndex={expandedFrameIndex}
              onFrameClick={handleFrameClick}
              onToggleReferenceExpansion={handleToggleExpansion}
              onHoverFrame={handleHoverFrame}
              getFrameTypeColorClass={getFrameTypeColorClass}
            />
          ) : (
            <ThumbnailsView
              frames={frames}
              currentFrameIndex={currentFrameIndex}
              thumbnails={thumbnails}
              loadingThumbnails={loadingThumbnails}
              referencedFrameIndices={referencedFrameIndices}
              expandedFrameIndex={expandedFrameIndex}
              onFrameClick={handleFrameClick}
              onToggleReferenceExpansion={handleToggleExpansion}
              onHoverFrame={handleHoverFrame}
              getFrameTypeColorClass={getFrameTypeColorClass}
            />
          )
        ) : displayView === 'sizes' ? (
          <>
            <FrameSizesView
              frames={frames}
              maxSize={maxSize}
              currentFrameIndex={currentFrameIndex}
              visibleFrameTypes={visibleFrameTypes}
              onFrameClick={handleFrameClick}
              getFrameTypeColorClass={getFrameTypeColorClass}
              sizeMetrics={sizeMetrics}
            />
            {createPortal(
              <FrameSizesLegend
                sizeMetrics={sizeMetrics}
                onToggleMetric={toggleMetric}
              />,
              document.body
            )}
          </>
        ) : displayView === 'bpyramid' ? (
          <BPyramidView
            frames={frames}
            currentFrameIndex={currentFrameIndex}
            onFrameClick={handleFrameClick}
            getFrameTypeColorClass={getFrameTypeColorClass}
          />
        ) : displayView === 'timeline' ? (
          <TimelineView
            frames={frames}
            currentFrameIndex={currentFrameIndex}
            onFrameClick={handleFrameClick}
            getFrameTypeColorClass={getFrameTypeColorClass}
          />
        ) : displayView === 'hrdbuffer' ? (
          <div className="filmstrip-empty">
            <span className="codicon codicon-database" aria-hidden="true"></span>
            <p>HRD Buffer View</p>
            <p className="hint">Coming soon</p>
          </div>
        ) : displayView === 'enhanced' ? (
          <div className="filmstrip-empty">
            <span className="codicon codicon-sparkle" aria-hidden="true"></span>
            <p>Enhanced View</p>
            <p className="hint">Coming soon</p>
          </div>
        ) : (
          <MinimapView
            frames={frames}
            currentFrameIndex={currentFrameIndex}
            onFrameClick={handleFrameClick}
            getFrameTypeColorClass={getFrameTypeColorClass}
            getFrameTypeColor={getFrameTypeColor}
          />
        )}
      </div>

      {/* Custom Tooltip */}
      {hoveredFrame && mousePosition && (
        <FilmstripTooltip
          frame={hoveredFrame}
          x={mousePosition.x}
          y={mousePosition.y}
          placement={mousePosition.placement}
        />
      )}
    </div>
  );
}

// Memoize Filmstrip to prevent unnecessary re-renders
export const MemoizedFilmstrip = memo(Filmstrip, (prevProps, nextProps) => {
  // Only re-render if frames array reference changed (not just contents)
  // or if className changed
  return (
    prevProps.frames === nextProps.frames &&
    prevProps.className === nextProps.className &&
    prevProps.viewMode === nextProps.viewMode
  );
});

// Export as default for backward compatibility
export default memo(Filmstrip);
