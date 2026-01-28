/**
 * Thumbnails View Component
 *
 * Displays frame thumbnails in a horizontal scrollable strip
 *
 * Arrows are pre-rendered for all frames and controlled via visibility,
 * similar to how thumbnail boxes are pre-rendered and scroll naturally.
 */

import { useRef, useCallback, useState, memo } from 'react';
import type { FrameInfo } from '../../../types/video';
import { usePreRenderedArrows, ArrowPosition, PathCalculator, FrameInfoBase } from '../../usePreRenderedArrows';
import { getFrameTypeColor } from '../../../types/video';

interface ThumbnailsViewProps {
  frames: FrameInfo[];
  currentFrameIndex: number;
  thumbnails: Map<number, string>;
  loadingThumbnails: Set<number>;
  referencedFrameIndices: Set<number>;
  expandedFrameIndex: number | null;
  onFrameClick: (frameIndex: number) => void;
  onToggleReferenceExpansion: (frameIndex: number, e: React.MouseEvent) => void;
  onHoverFrame: (frame: FrameInfo | null, x: number, y: number) => void;
  getFrameTypeColorClass: (frameType: string) => string;
}

function ThumbnailsView({
  frames,
  currentFrameIndex,
  thumbnails,
  loadingThumbnails,
  referencedFrameIndices,
  expandedFrameIndex,
  onFrameClick,
  onToggleReferenceExpansion,
  onHoverFrame,
  getFrameTypeColorClass,
}: ThumbnailsViewProps) {

  const containerRef = useRef<HTMLDivElement>(null);
  const svgRef = useRef<SVGSVGElement>(null);
  const [zoom, setZoom] = useState(1);

  // Path calculator for ㄷ-shaped arrows (down, horizontal, up)
  const calculateThumbnailPath: PathCalculator = useCallback((
    sourcePos: ArrowPosition,
    targetPos: ArrowPosition,
    _sourceFrame: FrameInfoBase,
    _targetFrame: FrameInfoBase,
    slotIndex: number
  ) => {
    // Each outgoing arrow gets a different vertical offset to space them out
    const baseOffset = 30;
    const spacingPerSlot = 12;
    const verticalOffset = baseOffset + (slotIndex * spacingPerSlot);
    // ㄷ-shaped path: down from source, then horizontal, then up to target
    // Use default values for bottom position in case DOM elements are not rendered
    const sourceBottom = sourcePos.bottom ?? 0;
    const targetBottom = targetPos.bottom ?? 0;
    return `M ${sourcePos.centerX} ${sourceBottom} L ${sourcePos.centerX} ${sourceBottom + verticalOffset} L ${targetPos.centerX} ${targetBottom + verticalOffset} L ${targetPos.centerX} ${targetBottom}`;
  }, []);

  // Use shared hook for pre-rendered arrows
  const { allArrowData, svgWidth } = usePreRenderedArrows({
    containerRef,
    frames,
    getFrameTypeColor,
    calculatePath: calculateThumbnailPath,
    enabled: true,
  });

  const handleMouseEnter = (frame: FrameInfo, e: React.MouseEvent) => {
    onHoverFrame(frame, e.clientX, e.clientY);
  };

  const handleMouseMove = (frame: FrameInfo, e: React.MouseEvent) => {
    onHoverFrame(frame, e.clientX, e.clientY);
  };

  // Handle wheel zoom for filmstrip
  const handleWheel = (e: React.WheelEvent) => {
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      e.stopPropagation();
      const delta = e.deltaY > 0 ? -0.1 : 0.1;
      setZoom(z => Math.max(0.5, Math.min(3, z + delta)));
    }
  };

  return (
    <div className="filmstrip-thumbnails-container">
      <div
        className="filmstrip-thumbnails"
        role="list"
        aria-label="Frame thumbnails"
        ref={containerRef}
        onWheel={handleWheel}
        style={{ transform: `scaleX(${zoom})`, transformOrigin: 'left center' }}
      >
        {/* Reference arrows SVG overlay - scrolls with thumbnails */}
        {allArrowData.length > 0 && svgWidth > 0 && (
          <svg
            ref={svgRef}
            className="thumbnail-arrows-overlay"
            xmlns="http://www.w3.org/2000/svg"
            style={{
              position: 'absolute',
              top: 0,
              left: 0,
              width: `${svgWidth}px`,
              height: '100%',
              pointerEvents: 'none',
              zIndex: 10,
              transform: `scaleX(${zoom})`,
              transformOrigin: 'left',
            }}
          >
            <defs>
              <marker
                id="thumbnail-arrowhead"
                markerWidth="6"
                markerHeight="6"
                refX="5"
                refY="3"
                orient="auto"
              >
                <path d="M 0 0 L 6 3 L 0 6 z" fill="currentColor" />
              </marker>
            </defs>
            {allArrowData.map((arrow) => {
              // Use opacity instead of visibility to ensure rendering
              // Force render all arrows by using opacity 0 instead of visibility hidden
              const isVisible = arrow.sourceFrameIndex === currentFrameIndex;
              const opacity = isVisible ? 0.7 : 0;
              const renderVisibility = isVisible ? 'visible' : 'hidden';

              return (
                <g key={`${arrow.sourceFrameIndex}-${arrow.targetFrameIndex}-${arrow.slotIndex}`}>
                  <path
                    d={arrow.pathData}
                    fill="none"
                    stroke={arrow.color}
                    strokeWidth="2"
                    strokeOpacity={opacity}
                    visibility={renderVisibility}
                    style={{ willChange: 'opacity, visibility' }}
                    markerEnd="url(#thumbnail-arrowhead)"
                  />
                  {/* Slot label at the start of the arrow */}
                  <text
                    x={arrow.sourceX}
                    y={arrow.labelY}
                    fill={arrow.color}
                    fontSize="9"
                    fontFamily="monospace"
                    fontWeight="600"
                    opacity={opacity}
                    visibility={renderVisibility}
                    textAnchor="middle"
                    dominantBaseline="middle"
                    style={{ willChange: 'opacity, visibility' }}
                  >
                    {arrow.label}
                  </text>
                </g>
              );
            })}
          </svg>
        )}
        {frames.map((frame) => {
          const layer = frame.temporal_id?.toString() ?? 'A';
          const isSelected = frame.frame_index === currentFrameIndex;
          const isReferenced = referencedFrameIndices.has(frame.frame_index);

          return (
            <div
              key={frame.frame_index}
              data-frame-index={frame.frame_index}
              className={`filmstrip-frame ${getFrameTypeColorClass(frame.frame_type)} ${
                isSelected ? 'selected' : ''
              } ${
                isReferenced ? 'is-referenced' : ''
              }`}
              onClick={() => onFrameClick(frame.frame_index)}
              onMouseEnter={(e) => handleMouseEnter(frame, e)}
              onMouseMove={(e) => handleMouseMove(frame, e)}
              onMouseLeave={() => onHoverFrame(null, 0, 0)}
              role="listitem"
              tabIndex={isSelected ? 0 : -1}
              aria-label={`Frame ${frame.frame_index}, type ${frame.frame_type}, size ${(frame.size / 1024).toFixed(1)} KB`}
              aria-selected={isSelected}
            >
              <div className="frame-thumbnail-wrapper">
                <div className="frame-header-inner" style={{ color: getFrameTypeColor(frame.frame_type) }}>
                  {frame.frame_type}-{layer} {frame.frame_index}
                </div>

                <div className="frame-thumbnail">
                  {thumbnails.get(frame.frame_index) ? (
                    <img
                      src={thumbnails.get(frame.frame_index)}
                      alt={`Frame ${frame.frame_index}`}
                      style={{ width: '100%', height: '100%', objectFit: 'contain' }}
                    />
                  ) : loadingThumbnails.has(frame.frame_index) ? (
                    <div className="frame-placeholder loading" aria-label="Loading thumbnail">
                      <span className="codicon codicon-loading codicon-spin" aria-hidden="true"></span>
                    </div>
                  ) : (
                    <div
                      className="frame-placeholder"
                      data-frame-type={frame.frame_type}
                      aria-label={`${frame.frame_type} frame placeholder`}
                    >
                      <span className="codicon codicon-device-camera" aria-hidden="true"></span>
                    </div>
                  )}
                </div>

                <div className="frame-nal-type-inner" aria-label={`NAL unit type: ${frame.frame_type}`}>
                  {frame.frame_type}
                </div>

                {(frame.display_order !== undefined || frame.coding_order !== undefined) && (
                  <div className="frame-order-info-inner">
                    {frame.display_order !== undefined && (
                      <span className="frame-display-order" title="Display Order">
                        D:{frame.display_order}
                      </span>
                    )}
                    {frame.coding_order !== undefined && (
                      <span className="frame-coding-order" title="Coding Order">
                        C:{frame.coding_order}
                      </span>
                    )}
                  </div>
                )}

                {frame.size === 0 && (
                  <div className="frame-error-badge" role="alert" aria-label="Error loading frame">!</div>
                )}

                {frame.ref_frames && frame.ref_frames.length > 0 && (
                  <div
                    className={`frame-ref-badge ${expandedFrameIndex === frame.frame_index ? 'expanded' : ''}`}
                    data-count={frame.ref_frames.length}
                    title={`References: ${frame.ref_frames.join(', ')}`}
                    aria-label={`Reference frames: ${frame.ref_frames.join(', ')}`}
                    onClick={(e) => onToggleReferenceExpansion(frame.frame_index, e)}
                    style={{ pointerEvents: 'auto', cursor: 'pointer' }}
                  >
                    {expandedFrameIndex === frame.frame_index ? (
                      <div className="ref-indices">
                        {frame.ref_frames.map((refIdx) => (
                          <span key={refIdx} className="ref-index">#{refIdx}</span>
                        ))}
                      </div>
                    ) : null}
                  </div>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

// Memoize ThumbnailsView to prevent unnecessary re-renders
export default memo(ThumbnailsView, (prevProps, nextProps) => {
  return (
    prevProps.frames === nextProps.frames &&
    prevProps.currentFrameIndex === nextProps.currentFrameIndex &&
    prevProps.expandedFrameIndex === nextProps.expandedFrameIndex &&
    prevProps.thumbnails === nextProps.thumbnails &&
    prevProps.loadingThumbnails === nextProps.loadingThumbnails &&
    prevProps.referencedFrameIndices === nextProps.referencedFrameIndices
  );
});
