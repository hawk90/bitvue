/**
 * usePreRenderedArrows Hook
 *
 * Shared hook for managing pre-rendered reference arrows.
 * Pre-calculates all arrow paths once at mount, then controls visibility via frame index.
 * Used by both ThumbnailsView and BPyramidTimeline for optimal performance.
 */

import { useRef, useEffect, useState, useCallback } from 'react';

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
  slotIndex: number
) => string;

interface UsePreRenderedArrowsProps {
  containerRef: React.RefObject<HTMLDivElement>;
  frames: FrameInfoBase[];
  getFrameTypeColor: (frameType: string) => string;
  calculatePath: PathCalculator;
  enabled: boolean;
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
}: UsePreRenderedArrowsProps) {
  const [allArrowData, setAllArrowData] = useState<ArrowData[]>([]);
  const [svgWidth, setSvgWidth] = useState(0);
  const arrowsCalculatedRef = useRef(false);

  /**
   * Calculate all arrow paths from DOM positions
   */
  const calculateArrows = useCallback((): ArrowData[] => {
    const container = containerRef.current;
    if (!container || frames.length === 0) return [];

    const arrows: ArrowData[] = [];
    const framePositions = new Map<number, ArrowPosition>();

    // Measure frame positions
    frames.forEach(frame => {
      const el = container.querySelector(`[data-frame-index="${frame.frame_index}"]`) as HTMLElement;
      if (!el) return;

      const rect = el.getBoundingClientRect();
      const containerRect = container.getBoundingClientRect();

      const pos: ArrowPosition = {
        centerX: (rect.left - containerRect.left) + rect.width / 2,
        top: rect.top - containerRect.top,
        bottom: rect.bottom - containerRect.top,
      };

      framePositions.set(frame.frame_index, pos);
    });

    // Calculate arrows for all frames
    frames.forEach((frame) => {
      const refFrameIndices = extractRefFrameIndices(frame);
      if (refFrameIndices.length === 0) return;

      const sourcePos = framePositions.get(frame.frame_index);
      if (!sourcePos) return;

      refFrameIndices.forEach((refIdx, slotIdx) => {
        const targetPos = framePositions.get(refIdx);
        if (!targetPos) return;

        const refFrame = frames.find(f => f.frame_index === refIdx);
        if (!refFrame) return;

        const label = getSlotLabel(frame, slotIdx);
        const pathData = calculatePath(sourcePos, targetPos, frame, refFrame, slotIdx);

        // Calculate label position (at the start of the arrow, vertically centered)
        const baseOffset = 30;
        const spacingPerSlot = 12;
        const verticalOffset = baseOffset + (slotIdx * spacingPerSlot);
        const sourceX = sourcePos.centerX;
        const sourceY = sourcePos.bottom ?? sourcePos.top;
        const labelY = sourceY + verticalOffset / 2;

        arrows.push({
          sourceFrameIndex: frame.frame_index,
          targetFrameIndex: refIdx,
          slotIndex: slotIdx,
          label,
          color: getFrameTypeColor(refFrame.frame_type),
          pathData,
          sourceX,
          sourceY,
          labelY,
        });
      });
    });

    return arrows;
  }, [containerRef, frames, getFrameTypeColor, calculatePath]);

  /**
   * One-time calculation of arrow paths from DOM positions
   */
  useEffect(() => {
    if (arrowsCalculatedRef.current || !enabled || frames.length === 0) return;

    const timer = setTimeout(() => {
      const container = containerRef.current;
      if (!container) return;

      const arrows = calculateArrows();

      setAllArrowData(arrows);
      arrowsCalculatedRef.current = true;

      // Set SVG width to cover entire scrollable content
      if (container.scrollWidth > 0) {
        setSvgWidth(container.scrollWidth);
      }
    }, 100);

    return () => clearTimeout(timer);
  }, [frames, enabled, calculateArrows]);

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
    window.addEventListener('resize', updateSvgWidth);
    return () => window.removeEventListener('resize', updateSvgWidth);
  }, [allArrowData.length, containerRef]);

  return {
    allArrowData,
    svgWidth,
    isReady: arrowsCalculatedRef.current,
  };
}
