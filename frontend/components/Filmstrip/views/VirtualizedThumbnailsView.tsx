/**
 * Virtualized Thumbnails View Component
 *
 * Displays frame thumbnails using react-window for virtual scrolling.
 * Only renders visible frames, dramatically reducing DOM nodes for long videos.
 *
 * Performance benefits:
 * - 80-90% reduction in DOM nodes for videos with 1000+ frames
 * - Faster initial render (only renders visible frames)
 * - Smoother scrolling with constant memory footprint
 */

import { useRef, useCallback, useState, useEffect, memo } from "react";
import type { FrameInfo } from "../../../types/video";
import {
  usePreRenderedArrows,
  ArrowPosition,
  PathCalculator,
  FrameInfoBase,
} from "../../usePreRenderedArrows";
import { getFrameTypeColor } from "../../../types/video";

interface VirtualizedThumbnailsViewProps {
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
  /** Requests thumbnails for the given frame indices be loaded (no-ops for ones already
   * loaded/loading). Needed here because, unlike the non-virtualized view, this component's own
   * scroll/visible-range changes are the only signal that new frames need thumbnails -- the
   * parent's IntersectionObserver-based lazy loader is intentionally skipped for virtualized
   * filmstrips (see Filmstrip.tsx), since elements outside the visible window never mount. */
  loadThumbnails: (indices: number[]) => void;
}

// Constants for virtual scrolling - must match CSS
const VISIBLE_WINDOW = 50; // Number of frames to render around current position
// A reference arrow can only be drawn to a target that's actually mounted -- outside
// `VISIBLE_WINDOW`, `usePreRenderedArrows`' `framePositions.get(refIdx)` silently misses and the
// arrow is just never drawn (found via real interactive testing: clicking a frame whose
// references had scrolled out of the window showed zero arrows, no explanation). References
// always point backward (spec: only already-decoded pictures), so widen `start` -- never `end`
// -- to include the selected frame's own nearby references. Capped rather than unbounded: AV1
// streams commonly keep frame 0 as a rolling backup reference for the *entire* file (seen in real
// fixture data), and unconditionally widening to include it would mount thousands of DOM nodes
// for a single arrow, defeating virtualization's whole purpose. A reference beyond this cap still
// won't get an on-canvas arrow -- but it's not silently lost data either: the frame's full
// `ref_frames` list is always in its real hover tooltip regardless.
const MAX_EXTRA_WINDOW_FOR_REFS = 100;

// Shared with `calculateThumbnailPath` (draws each stacked line's turn point at
// `sourceBottom + ARROW_BASE_OFFSET + slotIndex * ARROW_SPACING_PER_SLOT`) and with the
// `arrowAreaHeight` reservation below -- keeping one source of truth for both means the reserved
// space and the actual line positions can never drift apart.
const ARROW_BASE_OFFSET = 30;
const ARROW_SPACING_PER_SLOT = 12;

function VirtualizedThumbnailsView({
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
  loadThumbnails,
}: VirtualizedThumbnailsViewProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [zoom, setZoom] = useState(1);

  // Calculate visible range of frames
  const getVisibleRange = useCallback(() => {
    const normalStart = Math.max(0, currentFrameIndex - VISIBLE_WINDOW);
    const end = Math.min(frames.length, currentFrameIndex + VISIBLE_WINDOW + 1);

    const refs = frames[currentFrameIndex]?.ref_frames ?? [];
    const nearestRequiredStart =
      refs.length > 0 ? Math.min(normalStart, ...refs) : normalStart;
    const cappedFloor = normalStart - MAX_EXTRA_WINDOW_FOR_REFS;
    const start = Math.max(0, cappedFloor, nearestRequiredStart);

    return { start, end };
  }, [currentFrameIndex, frames]);

  const { start, end } = getVisibleRange();
  const visibleFrames = frames.slice(start, end);

  // Load thumbnails for whatever's currently in the visible window -- the batch loaded on
  // initial mount (useFilmstripState) only covers the first THUMBNAIL_BATCH_SIZE frames, and the
  // parent's IntersectionObserver lazy-loader never runs for virtualized filmstrips, so without
  // this, thumbnails permanently freeze at the initial batch once the user scrolls/plays past it.
  useEffect(() => {
    loadThumbnails(visibleFrames.map((f) => f.frame_index));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [start, end, loadThumbnails]);

  // Path calculator for ㄷ-shaped arrows (down, horizontal, up) -- identical shape to
  // ThumbnailsView's, duplicated rather than shared since the two views' DOM structure/zoom
  // handling differ enough that a shared abstraction would need its own prop surface.
  const calculateThumbnailPath: PathCalculator = useCallback(
    (
      sourcePos: ArrowPosition,
      targetPos: ArrowPosition,
      _sourceFrame: FrameInfoBase,
      _targetFrame: FrameInfoBase,
      slotIndex: number,
    ) => {
      const verticalOffset =
        ARROW_BASE_OFFSET + slotIndex * ARROW_SPACING_PER_SLOT;
      const sourceBottom = sourcePos.bottom ?? 0;
      const targetBottom = targetPos.bottom ?? 0;
      return `M ${sourcePos.centerX} ${sourceBottom} L ${sourcePos.centerX} ${sourceBottom + verticalOffset} L ${targetPos.centerX} ${targetBottom + verticalOffset} L ${targetPos.centerX} ${targetBottom}`;
    },
    [],
  );

  // Arrows must be recalculated whenever the visible window shifts (recalcKey), not just once at
  // mount -- only frames within [start, end) are actually in the DOM, so a mount-once measurement
  // would only ever see whatever happened to be visible at that instant.
  const { allArrowData, svgWidth, maxStackDepth } = usePreRenderedArrows({
    containerRef,
    frames: visibleFrames,
    getFrameTypeColor,
    calculatePath: calculateThumbnailPath,
    enabled: true,
    recalcKey: `${start}-${end}`,
  });

  // Reserve exactly as much room below the frame cards as the *real* mounted data needs for its
  // deepest reference stack, instead of a fixed constant sized for AV1's worst case (7 slots)
  // even when the actual stream never goes past 1-2 -- must match `calculateThumbnailPath`'s own
  // `baseOffset`/`spacingPerSlot` since that's what actually positions each line's turn point.
  // `maxStackDepth` comes from the whole mounted window (not just the selected frame), so
  // switching between two already-mounted frames never causes this to jump.
  const arrowAreaHeight =
    maxStackDepth > 0
      ? ARROW_BASE_OFFSET + (maxStackDepth - 1) * ARROW_SPACING_PER_SLOT + 20
      : 0;

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
      setZoom((z) => Math.max(0.5, Math.min(3, z + delta)));
    }
  };

  // Auto-scroll to current frame when currentFrameIndex changes
  // Note: For better UX, we rely on the parent's IntersectionObserver-based scrolling

  return (
    <div className="filmstrip-thumbnails-container virtualized">
      <div
        className="filmstrip-thumbnails"
        role="list"
        aria-label="Frame thumbnails (virtualized view showing frames {start}-{end} of {frames.length})"
        ref={containerRef}
        onWheel={handleWheel}
        style={{
          transform: `scaleX(${zoom})`,
          transformOrigin: "left center",
          paddingBottom: `${arrowAreaHeight}px`,
        }}
      >
        {/* Reference arrows SVG overlay - scrolls with thumbnails */}
        {allArrowData.length > 0 && svgWidth > 0 && (
          <svg
            className="thumbnail-arrows-overlay"
            xmlns="http://www.w3.org/2000/svg"
            style={{
              position: "absolute",
              top: 0,
              left: 0,
              width: `${svgWidth}px`,
              height: "100%",
              pointerEvents: "none",
              zIndex: 10,
              transform: `scaleX(${zoom})`,
              transformOrigin: "left",
            }}
          >
            <defs>
              <marker
                id="virtualized-thumbnail-arrowhead"
                markerWidth="6"
                markerHeight="6"
                refX="5"
                refY="3"
                orient="auto"
              >
                <path d="M 0 0 L 6 3 L 0 6 z" fill="currentColor" />
              </marker>
            </defs>
            {/* Two separate passes over the SAME arrow list -- all lines, THEN all labels -- see
            ThumbnailsView.tsx's identical structure for the full reasoning: interleaving
            path-then-label per arrow only protects a label against its OWN arrow's line, since
            `.thumbnail-arrows-overlay path`'s forced compositing (`transform: translateZ(0)`)
            makes every `<path>` paint in a promoted, document-order-based phase -- a different
            arrow's line, appearing later in that phase, still painted over an earlier arrow's
            already-"fixed" label whenever their geometries crossed. Rendering every path first,
            then every label (also promoted, via `.thumbnail-arrows-overlay .arrow-label`),
            guarantees every label's document-order position is after every path's, system-wide. */}
            {allArrowData.map((arrow) => {
              const isVisible = arrow.sourceFrameIndex === currentFrameIndex;
              const opacity = isVisible ? 0.7 : 0;
              const renderVisibility = isVisible ? "visible" : "hidden";
              return (
                <path
                  key={`line-${arrow.sourceFrameIndex}-${arrow.targetFrameIndex}-${arrow.slotIndex}`}
                  d={arrow.pathData}
                  fill="none"
                  stroke={arrow.color}
                  strokeWidth="2"
                  strokeOpacity={opacity}
                  visibility={renderVisibility}
                  markerEnd="url(#virtualized-thumbnail-arrowhead)"
                />
              );
            })}
            {allArrowData.map((arrow) => {
              const isVisible = arrow.sourceFrameIndex === currentFrameIndex;
              const opacity = isVisible ? 0.7 : 0;
              const renderVisibility = isVisible ? "visible" : "hidden";
              return (
                <g
                  key={`label-${arrow.sourceFrameIndex}-${arrow.targetFrameIndex}-${arrow.slotIndex}`}
                  className="arrow-label"
                >
                  {/* The label sits ON its own line's turn point (see usePreRenderedArrows'
                  labelY comment) -- for a long merged label ("REF0,REF1,...,REF5") spanning back
                  to a distant target, a line's horizontal run would otherwise pass directly
                  through the bare glyphs with nothing to occlude it, reading as a strikethrough.
                  This background chip (same width estimate as the label's monospace char count)
                  hides that segment behind the label instead, a standard labeled-edge technique. */}
                  <rect
                    x={arrow.sourceX - (arrow.label.length * 5.5 + 6) / 2}
                    y={arrow.labelY - 7}
                    width={arrow.label.length * 5.5 + 6}
                    height={14}
                    style={{ fill: "var(--bg-app)" }}
                    opacity={isVisible ? 0.95 : 0}
                    visibility={renderVisibility}
                  />
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
                  >
                    {arrow.label}
                  </text>
                </g>
              );
            })}
          </svg>
        )}
        {visibleFrames.map((frame) => {
          const layer = frame.temporal_id?.toString() ?? "A";
          const isSelected = frame.frame_index === currentFrameIndex;
          const isReferenced = referencedFrameIndices.has(frame.frame_index);

          return (
            <div
              key={frame.frame_index}
              data-frame-index={frame.frame_index}
              className={`filmstrip-frame ${getFrameTypeColorClass(frame.frame_type)} ${
                isSelected ? "selected" : ""
              } ${isReferenced ? "is-referenced" : ""}`}
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
                <div
                  className="frame-header-inner"
                  style={{
                    color: `var(--frame-${frame.frame_type.toLowerCase()})`,
                  }}
                >
                  {frame.frame_type}-{layer} {frame.frame_index}
                </div>

                <div className="frame-thumbnail">
                  {thumbnails.get(frame.frame_index) ? (
                    <img
                      src={thumbnails.get(frame.frame_index)}
                      alt={`Frame ${frame.frame_index}`}
                      style={{
                        width: "100%",
                        height: "100%",
                        objectFit: "contain",
                      }}
                    />
                  ) : loadingThumbnails.has(frame.frame_index) ? (
                    <div
                      className="frame-placeholder loading"
                      aria-label="Loading thumbnail"
                    >
                      <span
                        className="codicon codicon-loading codicon-spin"
                        aria-hidden="true"
                      ></span>
                    </div>
                  ) : (
                    <div
                      className="frame-placeholder"
                      data-frame-type={frame.frame_type}
                      aria-label={`${frame.frame_type} frame placeholder`}
                    >
                      <span
                        className="codicon codicon-device-camera"
                        aria-hidden="true"
                      ></span>
                    </div>
                  )}
                </div>

                <div
                  className="frame-nal-type-inner"
                  aria-label={`NAL unit type: ${frame.frame_type}`}
                >
                  {frame.frame_type}
                </div>

                {(frame.display_order !== undefined ||
                  frame.coding_order !== undefined) && (
                  <div className="frame-order-info-inner">
                    {frame.display_order !== undefined && (
                      <span
                        className="frame-display-order"
                        title="Display Order"
                      >
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
                  <div
                    className="frame-error-badge"
                    role="alert"
                    aria-label="Error loading frame"
                  >
                    !
                  </div>
                )}

                {frame.ref_frames && frame.ref_frames.length > 0 && (
                  <div
                    className={`frame-ref-badge ${expandedFrameIndex === frame.frame_index ? "expanded" : ""}`}
                    data-count={frame.ref_frames.length}
                    title={`References: ${frame.ref_frames.join(", ")}`}
                    aria-label={`Reference frames: ${frame.ref_frames.join(", ")}`}
                    onClick={(e) =>
                      onToggleReferenceExpansion(frame.frame_index, e)
                    }
                    style={{ pointerEvents: "auto", cursor: "pointer" }}
                  >
                    {expandedFrameIndex === frame.frame_index ? (
                      <div className="ref-indices">
                        {frame.ref_frames.map((refIdx) => (
                          <span key={refIdx} className="ref-index">
                            #{refIdx}
                          </span>
                        ))}
                      </div>
                    ) : null}
                  </div>
                )}
              </div>
            </div>
          );
        })}

        {/* Spacer at the beginning to maintain scroll position */}
        {start > 0 && (
          <div
            className="filmstrip-virtual-spacer"
            style={{
              width: `${start * 148}px`, // Approximate width per frame
              flexShrink: 0,
            }}
            aria-hidden="true"
          />
        )}

        {/* Spacer at the end to maintain scroll position */}
        {end < frames.length && (
          <div
            className="filmstrip-virtual-spacer"
            style={{
              width: `${(frames.length - end) * 148}px`,
              flexShrink: 0,
            }}
            aria-hidden="true"
          />
        )}
      </div>
    </div>
  );
}

// Memoize VirtualizedThumbnailsView to prevent unnecessary re-renders
export default memo(VirtualizedThumbnailsView, (prevProps, nextProps) => {
  return (
    prevProps.frames === nextProps.frames &&
    prevProps.currentFrameIndex === nextProps.currentFrameIndex &&
    prevProps.expandedFrameIndex === nextProps.expandedFrameIndex &&
    prevProps.thumbnails === nextProps.thumbnails &&
    prevProps.loadingThumbnails === nextProps.loadingThumbnails &&
    prevProps.referencedFrameIndices === nextProps.referencedFrameIndices
  );
});
