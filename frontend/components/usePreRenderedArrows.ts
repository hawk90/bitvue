/**
 * usePreRenderedArrows Hook
 *
 * Shared hook for managing pre-rendered reference arrows.
 * Pre-calculates all arrow paths once at mount, then controls visibility via frame index.
 * Used by both ThumbnailsView and BPyramidTimeline for optimal performance.
 */

import { useRef, useEffect, useState, useCallback } from "react";

export interface ArrowData {
  sourceFrameIndex: number;
  targetFrameIndex: number;
  slotIndex: number;
  label: string;
  color: string;
  pathData: string;
  sourceX: number;
  sourceY: number;
  labelY: number;
}

export interface ArrowPosition {
  centerX: number;
  top: number;
  bottom?: number;
}

/**
 * Base frame info interface to avoid circular dependencies
 * Extracted from FrameInfo for shared use across components
 */
export interface FrameInfoBase {
  frame_index: number;
  frame_type: string;
  size: number;
  ref_frames?: number[];
  ref_slots?: number[];
  ref_slot_info?: Array<{ name: string }>;
}

/**
 * Path calculator function type
 * Components can provide their own path calculation logic
 */
export type PathCalculator = (
  sourcePos: ArrowPosition,
  targetPos: ArrowPosition,
  sourceFrame: FrameInfoBase,
  targetFrame: FrameInfoBase,
  slotIndex: number,
) => string;

interface UsePreRenderedArrowsProps {
  containerRef: React.RefObject<HTMLDivElement>;
  frames: FrameInfoBase[];
  getFrameTypeColor: (frameType: string) => string;
  calculatePath: PathCalculator;
  enabled: boolean;
  /** Value that, when changed, forces arrows to be recalculated even though `frames` itself is
   * the same reference -- needed for virtualized views where only a sliding window of frames is
   * actually mounted in the DOM at any given time, so a one-time-at-mount measurement goes stale
   * as the window scrolls (previously-measured elements unmount, newly-visible ones never get
   * measured). Omit for non-virtualized views, which keep the original mount-once behavior. */
  recalcKey?: string | number;
}

/**
 * Extract reference frame indices from a frame
 */
function extractRefFrameIndices(frame: FrameInfoBase): number[] {
  if (frame.ref_frames && frame.ref_frames.length > 0) {
    return frame.ref_frames;
  }
  return [];
}

/**
 * Get slot label for reference
 */
function getSlotLabel(frame: FrameInfoBase, slotIdx: number): string {
  if (frame.ref_slot_info && slotIdx < frame.ref_slot_info.length) {
    return frame.ref_slot_info[slotIdx].name;
  } else if (frame.ref_slots && slotIdx < frame.ref_slots.length) {
    const slotIndex = frame.ref_slots[slotIdx];
    return `SLOT${slotIndex}`;
  }
  return `REF${slotIdx}`;
}

/**
 * Main hook for pre-rendered arrows
 */
export function usePreRenderedArrows({
  containerRef,
  frames,
  getFrameTypeColor,
  calculatePath,
  enabled,
  recalcKey,
}: UsePreRenderedArrowsProps) {
  const [allArrowData, setAllArrowData] = useState<ArrowData[]>([]);
  const [svgWidth, setSvgWidth] = useState(0);
  // Deepest stack of same-source arrows seen across the currently mounted frames (post-dedup) --
  // callers use this to reserve exactly as much vertical room as the *real* data needs (0 slots
  // apart when nothing references anything, up to AV1's real 7-slot max), instead of a hardcoded
  // constant that either wastes space for simple streams or clips a genuinely 7-deep one. Computed
  // from the whole mounted window (not just the selected frame) so switching between two
  // already-mounted frames never needs a reflow/jump.
  const [maxStackDepth, setMaxStackDepth] = useState(0);
  const arrowsCalculatedRef = useRef(false);
  const lastRecalcKeyRef = useRef(recalcKey);

  /**
   * Calculate all arrow paths from DOM positions
   */
  const calculateArrows = useCallback((): {
    arrows: ArrowData[];
    maxStackDepth: number;
  } => {
    const container = containerRef.current;
    if (!container || frames.length === 0)
      return { arrows: [], maxStackDepth: 0 };

    const arrows: ArrowData[] = [];
    const framePositions = new Map<number, ArrowPosition>();

    // Measure frame positions. Deliberately `offsetLeft`/`offsetTop` (position relative to
    // `container`, the nearest positioned ancestor -- `.filmstrip-thumbnails` is `position:
    // relative` and each `.filmstrip-frame` is its direct child), NOT
    // `getBoundingClientRect()`-based viewport math. `getBoundingClientRect()` is
    // scroll-POSITION-relative (viewport-visible coordinates) -- fine for two elements measured
    // at the same instant when both happen to be on-screen, but breaks down the moment a target
    // (e.g. a reference to frame 0) is scrolled out of the currently-visible area: its rect ends
    // up far off-screen (a large negative `centerX` was actually observed via real debug
    // instrumentation, not a hunch), even though the element is genuinely mounted and its
    // *content-space* position is perfectly well-defined. `offsetLeft`/`offsetTop` describe that
    // content-space position directly and are unaffected by the container's current `scrollLeft`,
    // so an arrow to an off-screen-but-mounted target still gets a correct (if currently
    // off-screen) path -- exactly what's needed once the target scrolls into view.
    frames.forEach((frame) => {
      const el = container.querySelector(
        `[data-frame-index="${frame.frame_index}"]`,
      ) as HTMLElement;
      if (!el) return;

      const pos: ArrowPosition = {
        centerX: el.offsetLeft + el.offsetWidth / 2,
        top: el.offsetTop,
        bottom: el.offsetTop + el.offsetHeight,
      };

      framePositions.set(frame.frame_index, pos);
    });

    // Calculate arrows for all frames
    let maxArrowOrdinal = -1;
    frames.forEach((frame) => {
      const refFrameIndices = extractRefFrameIndices(frame);
      if (refFrameIndices.length === 0) return;

      const sourcePos = framePositions.get(frame.frame_index);
      if (!sourcePos) return;

      // AV1 has up to 7 real reference *slots* (LAST/LAST2/LAST3/GOLDEN/BWDREF/ALTREF2/ALTREF),
      // and it's normal -- not a data bug -- for several of them to point at the exact same
      // real frame (e.g. GOLDEN and ALTREF both reusing the same long-term DPB slot). Drawing one
      // arrow per raw slot then draws fully overlapping duplicate lines for the same target;
      // group by target frame index first so each real target gets exactly one arrow, with a
      // combined label ("REF4,5") for the slots that share it. Found via real interactive
      // testing: frame 69's real 7 slots were `[68,67,66,60,0,0,61]` (two of them both `0`).
      const slotsByTarget = new Map<number, number[]>();
      refFrameIndices.forEach((refIdx, slotIdx) => {
        const existing = slotsByTarget.get(refIdx);
        if (existing) {
          existing.push(slotIdx);
        } else {
          slotsByTarget.set(refIdx, [slotIdx]);
        }
      });

      // Stacking ordinal among this frame's *unique* targets (not the raw 0-6 slot index) --
      // deduping means a frame with duplicate slots needs fewer stacked lines than its raw slot
      // count, so this can be smaller than `REFS_PER_FRAME - 1`.
      let arrowOrdinal = 0;
      slotsByTarget.forEach((slotIndices, refIdx) => {
        const targetPos = framePositions.get(refIdx);
        if (!targetPos) return; // target frame isn't mounted in the current (possibly
        // virtualized) window -- see VirtualizedThumbnailsView's window-widening for the fix that
        // covers the common near-distance case; a still-missing very-distant reference (e.g. AV1
        // streams that keep frame 0 as a rolling backup reference for the whole file) is a
        // deliberate, documented boundary, not silently dropped data -- the frame's full
        // `ref_frames` list is still surfaced via its real hover tooltip regardless of whether an
        // on-canvas arrow could be drawn for it.

        const refFrame = frames.find((f) => f.frame_index === refIdx);
        if (!refFrame) return;

        const label = slotIndices.map((s) => getSlotLabel(frame, s)).join(",");
        const pathData = calculatePath(
          sourcePos,
          targetPos,
          frame,
          refFrame,
          arrowOrdinal,
        );

        // Label position -- must land ON this arrow's own horizontal line segment, not float
        // between it and the card. `calculatePath` (the caller-supplied path fn) draws that
        // segment at `sourceBottom + verticalOffset`; using half that offset here put every
        // slot's label in a shared band only spacingPerSlot/2 (6px) apart, misaligned with -- and
        // overlapping -- every line but its own (found via real interactive testing, not just a
        // screenshot: reported as "line and label don't line up at all" after clicking a 3rd
        // reference-bearing frame). Matching the same `verticalOffset` here gives each label the
        // full spacingPerSlot (12px) separation its own line already has. Uses `arrowOrdinal`
        // (this frame's unique-target index), matching what `calculatePath` above was given.
        const baseOffset = 30;
        const spacingPerSlot = 12;
        const verticalOffset = baseOffset + arrowOrdinal * spacingPerSlot;
        const sourceX = sourcePos.centerX;
        const sourceY = sourcePos.bottom ?? sourcePos.top;
        const labelY = sourceY + verticalOffset;

        arrows.push({
          sourceFrameIndex: frame.frame_index,
          targetFrameIndex: refIdx,
          slotIndex: arrowOrdinal,
          label,
          color: getFrameTypeColor(refFrame.frame_type),
          pathData,
          sourceX,
          sourceY,
          labelY,
        });
        maxArrowOrdinal = Math.max(maxArrowOrdinal, arrowOrdinal);
        arrowOrdinal++;
      });
    });

    return { arrows, maxStackDepth: maxArrowOrdinal + 1 };
  }, [containerRef, frames, getFrameTypeColor, calculatePath]);

  /**
   * One-time calculation of arrow paths from DOM positions
   */
  useEffect(() => {
    const keyChanged = recalcKey !== lastRecalcKeyRef.current;
    if (
      (arrowsCalculatedRef.current && !keyChanged) ||
      !enabled ||
      frames.length === 0
    )
      return;

    const timer = setTimeout(() => {
      const container = containerRef.current;
      if (!container) return;

      const { arrows, maxStackDepth: depth } = calculateArrows();

      setAllArrowData(arrows);
      setMaxStackDepth(depth);
      arrowsCalculatedRef.current = true;
      lastRecalcKeyRef.current = recalcKey;

      // Set SVG width to cover entire scrollable content
      if (container.scrollWidth > 0) {
        setSvgWidth(container.scrollWidth);
      }
    }, 100);

    return () => clearTimeout(timer);
  }, [frames, enabled, calculateArrows, containerRef, recalcKey]);

  /**
   * Update SVG width on resize
   */
  useEffect(() => {
    const updateSvgWidth = () => {
      const container = containerRef.current;
      if (container && allArrowData.length > 0) {
        setSvgWidth(container.scrollWidth);
      }
    };

    updateSvgWidth();
    window.addEventListener("resize", updateSvgWidth);
    return () => window.removeEventListener("resize", updateSvgWidth);
  }, [allArrowData.length, containerRef]);

  return {
    allArrowData,
    svgWidth,
    maxStackDepth,
    isReady: arrowsCalculatedRef.current,
  };
}
