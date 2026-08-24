/**
 * B-Pyramid Timeline Component
 *
 * Renders the timeline visualization with temporal level rows
 * Includes reference arrows for the current frame
 */

import {
  ForwardedRef,
  forwardRef,
  useRef,
  useEffect,
  useCallback,
} from "react";
import type { FrameInfo } from "../../../types/video";
import {
  TemporalLevel,
  FrameWithLevel,
  TemporalAnalysis,
} from "./BPyramidTypes";
import {
  usePreRenderedArrows,
  ArrowPosition,
  PathCalculator,
  FrameInfoBase,
} from "../../usePreRenderedArrows";
import { getFrameTypeColor, isKeyframe } from "../../../types/video";

/**
 * Analyze all frames and use temporal_id from AV1 bitstream
 * Falls back to calculated level for codecs without temporal_id
 */
export function analyzeTemporalLevels(frames: FrameInfo[]): TemporalAnalysis {
  if (frames.length === 0) {
    return {
      levels: [],
      gopBoundaries: [],
      frameMap: new Map(),
      framePositions: new Map(),
    };
  }

  // Find all keyframes to identify GOP boundaries
  const keyframes = [
    0,
    ...frames
      .filter((f, i) => i > 0 && isKeyframe(f.frame_type, f.key_frame))
      .map((f) => f.frame_index),
  ];

  // Use temporal_id from AV1 bitstream, fallback to 0
  const frameMap = new Map<number, FrameWithLevel>();
  const levelMap = new Map<number, FrameWithLevel[]>();

  frames.forEach((frame) => {
    // Use real temporal_id from AV1 OBU header
    const level = frame.temporal_id ?? 0;
    const isKey = isKeyframe(frame.frame_type, frame.key_frame);

    const frameWithLevel: FrameWithLevel = {
      index: frame.frame_index,
      frameIndex: frame.frame_index,
      frameType: frame.frame_type,
      refFrames: frame.ref_frames || [],
      level,
      isKeyframe: isKey,
    };

    frameMap.set(frame.frame_index, frameWithLevel);

    if (!levelMap.has(level)) {
      levelMap.set(level, []);
    }
    levelMap.get(level)!.push(frameWithLevel);
  });

  // Convert to sorted levels (highest first for top-down display)
  const maxLevel = Math.max(...levelMap.keys());
  const levels: TemporalLevel[] = [];

  for (let l = maxLevel; l >= 0; l--) {
    if (levelMap.has(l)) {
      levels.push({
        level: l,
        frames: levelMap.get(l)!.sort((a, b) => a.frameIndex - b.frameIndex),
      });
    }
  }

  return {
    levels,
    gopBoundaries: keyframes,
    frameMap,
    framePositions: new Map(), // Will be populated by DOM measurements
  };
}

interface BPyramidTimelineProps {
  frames: FrameInfo[];
  currentFrameIndex: number;
  onFrameClick: (frameIndex: number) => void;
  getFrameTypeColorClass: (frameType: string) => string;
  levels: TemporalLevel[];
  frameMap: Map<number, FrameWithLevel>;
  gopBoundaries: number[];
  /** See BPyramidViewProps.showAllArrows -- shows every frame's arrows at once (dimmed for
   * non-selected sources) instead of gating to only the currently-selected frame. */
  showAllArrows?: boolean;
}

export const BPyramidTimeline = forwardRef<
  HTMLDivElement,
  BPyramidTimelineProps
>(
  (
    {
      frames,
      currentFrameIndex,
      onFrameClick,
      getFrameTypeColorClass,
      levels,
      frameMap,
      gopBoundaries,
      showAllArrows = false,
    }: BPyramidTimelineProps,
    forwardedRef: ForwardedRef<HTMLDivElement>,
  ) => {
    const circleSize = 12; // 12px diameter circles
    const cellGap = 8;
    const totalCellWidth = circleSize + cellGap;

    const svgRef = useRef<SVGSVGElement>(null);
    const containerRef = useRef<HTMLDivElement>(null);

    // Path calculator for arrows from circle edges (left-to-left or right-to-right)
    const calculateStraightPath: PathCalculator = useCallback(
      (
        sourcePos: ArrowPosition,
        targetPos: ArrowPosition,
        _sourceFrame: FrameInfoBase,
        _targetFrame: FrameInfoBase,
        slotIndex: number,
      ) => {
        const circleRadius = 6; // 12px diameter = 6px radius

        // Each outgoing arrow gets a different vertical offset to space them out
        const baseOffset = 0;
        const spacingPerSlot = 10;
        const verticalOffset = baseOffset + slotIndex * spacingPerSlot;

        // Determine direction based on horizontal position
        const goRight = targetPos.centerX > sourcePos.centerX;

        // Start from left or right edge of source circle, with vertical offset
        const startX = goRight
          ? sourcePos.centerX + circleRadius
          : sourcePos.centerX - circleRadius;
        const startY = sourcePos.top + circleRadius + verticalOffset; // Center of circle vertically + offset

        // End at left or right edge of target circle, with vertical offset
        const endX = goRight
          ? targetPos.centerX - circleRadius
          : targetPos.centerX + circleRadius;
        const endY = targetPos.top + circleRadius + verticalOffset; // Center of circle vertically + offset

        // Arrow path: horizontal line from source to target
        return `M ${startX} ${startY} L ${endX} ${endY}`;
      },
      [],
    );

    // Path calculator for showAllArrows ("Show All Refs" toggle): routes below the frame row instead of
    // straight through circle centers. calculateStraightPath's near-zero vertical offset is fine
    // when only one frame's 1-2 arrows are visible at a time (the original B-Pyramid behavior),
    // but with every frame's arrows shown at once, most single-ref frames all land on slotIndex 0
    // -- i.e. the same near-zero offset -- and the resulting pile of overlapping horizontal lines
    // draws straight through the frame circles themselves rather than around them. Dropping below
    // the row (down -> across -> up, matching ThumbnailsView/VirtualizedThumbnailsView's shape)
    // keeps arrows visually clear of frame boundaries regardless of how many stack up.
    const calculateBelowPath: PathCalculator = useCallback(
      (
        sourcePos: ArrowPosition,
        targetPos: ArrowPosition,
        sourceFrame: FrameInfoBase,
        targetFrame: FrameInfoBase,
        slotIndex: number,
      ) => {
        // `slotIndex` is already scoped per direction (usePreRenderedArrows tracks
        // backward/forward refs as separate stacks), so refs on both sides don't pile into one
        // tower.
        const baseOffset = 16;
        const spacingPerSlot = 8;
        const verticalOffset = baseOffset + slotIndex * spacingPerSlot;
        // Fan the takeoff point instead of every stacked line dropping from the exact same pixel
        // (must match usePreRenderedArrows' identical `sourceX` formula, which positions this
        // slot's label on the same point) -- leans each line toward the side it's about to turn.
        const isForward = targetFrame.frame_index > sourceFrame.frame_index;
        const fanSpacing = 6;
        const fannedSourceX =
          sourcePos.centerX + (isForward ? 1 : -1) * slotIndex * fanSpacing;
        const sourceBottom = sourcePos.bottom ?? sourcePos.top;
        const targetBottom = targetPos.bottom ?? targetPos.top;
        return `M ${fannedSourceX} ${sourceBottom} L ${fannedSourceX} ${sourceBottom + verticalOffset} L ${targetPos.centerX} ${targetBottom + verticalOffset} L ${targetPos.centerX} ${targetBottom}`;
      },
      [],
    );

    // Use shared hook for pre-rendered arrows
    const { allArrowData, svgWidth } = usePreRenderedArrows({
      containerRef,
      frames,
      getFrameTypeColor,
      calculatePath: showAllArrows ? calculateBelowPath : calculateStraightPath,
      enabled: true,
    });

    // Sync forwardedRef with containerRef
    useEffect(() => {
      if (typeof forwardedRef === "function") {
        forwardedRef(containerRef.current);
      } else if (forwardedRef) {
        forwardedRef.current = containerRef.current;
      }
    }, [forwardedRef]);

    return (
      <div className="bpyramid-timeline-container" ref={containerRef}>
        <div
          className={`bpyramid-levels ${showAllArrows ? "bpyramid-levels--with-graph" : ""}`}
          style={{ position: "relative" }}
        >
          {levels.map((levelData) => (
            <div key={levelData.level} className="bpyramid-level-row">
              <div className="bpyramid-level-label">L{levelData.level}</div>
              <div
                className="bpyramid-timeline-track"
                style={{
                  display: "grid",
                  gridTemplateColumns: `repeat(${frames.length}, ${totalCellWidth}px)`,
                  gap: `${cellGap}px`,
                }}
              >
                {frames.map((frame) => {
                  const frameWithLevel = frameMap.get(frame.frame_index);
                  const belongsToLevel =
                    frameWithLevel?.level === levelData.level;

                  if (!belongsToLevel) {
                    return (
                      <div
                        key={frame.frame_index}
                        className="bpyramid-cell-empty"
                      />
                    );
                  }

                  const isSelected = frame.frame_index === currentFrameIndex;
                  const isGopBoundary = gopBoundaries.includes(
                    frame.frame_index,
                  );

                  return (
                    <div
                      key={frame.frame_index}
                      data-frame-index={frame.frame_index}
                      className={`bpyramid-frame-circle ${getFrameTypeColorClass(frame.frame_type)} ${
                        isSelected ? "selected" : ""
                      } ${isGopBoundary ? "gop-boundary" : ""}`}
                      onClick={() => onFrameClick(frame.frame_index)}
                      style={{
                        width: `${circleSize}px`,
                        height: `${circleSize}px`,
                        backgroundColor: getFrameTypeColor(frame.frame_type),
                      }}
                      title={`Frame ${frame.frame_index}: ${frame.frame_type} (Level ${levelData.level})`}
                    />
                  );
                })}
              </div>
            </div>
          ))}

          {/* Reference arrows SVG overlay */}
          {allArrowData.length > 0 && svgWidth > 0 && (
            <svg
              ref={svgRef}
              className="bpyramid-arrows-overlay"
              xmlns="http://www.w3.org/2000/svg"
              style={{
                position: "absolute",
                top: 0,
                left: 0,
                width: `${svgWidth}px`,
                height: "100%",
                pointerEvents: "none",
                zIndex: 10,
              }}
            >
              <defs>
                <marker
                  id="bpyramid-arrowhead"
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
                const isSelectedSource =
                  arrow.sourceFrameIndex === currentFrameIndex;
                const isVisible = showAllArrows || isSelectedSource;
                const opacity = !isVisible
                  ? 0
                  : showAllArrows && !isSelectedSource
                    ? 0.25
                    : 0.7;
                // With every frame's arrows shown at once (showAllArrows), rendering every
                // dimmed arrow's slot label too just tiles hundreds of overlapping "REF0"/"REF1"
                // strings into solid noise bands with no per-frame info at that density -- keep
                // labels only for the frame the user actually selected.
                const showLabel =
                  isVisible && (!showAllArrows || isSelectedSource);

                return (
                  <g
                    key={`${arrow.sourceFrameIndex}-${arrow.targetFrameIndex}-${arrow.slotIndex}`}
                  >
                    <path
                      d={arrow.pathData}
                      fill="none"
                      stroke={arrow.color}
                      strokeWidth="1.5"
                      strokeOpacity={opacity}
                      visibility={isVisible ? "visible" : "hidden"}
                      markerEnd="url(#bpyramid-arrowhead)"
                    />
                    {/* Slot label at the start of the arrow */}
                    <text
                      x={arrow.sourceX}
                      y={arrow.labelY}
                      fill={arrow.color}
                      fontSize="7"
                      fontFamily="monospace"
                      fontWeight="600"
                      opacity={opacity}
                      visibility={showLabel ? "visible" : "hidden"}
                      textAnchor="start"
                      dominantBaseline="middle"
                    >
                      {arrow.label}
                    </text>
                  </g>
                );
              })}
            </svg>
          )}
        </div>
      </div>
    );
  },
);

BPyramidTimeline.displayName = "BPyramidTimeline";
