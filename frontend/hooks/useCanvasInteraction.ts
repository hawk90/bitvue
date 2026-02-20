/**
 * useCanvasInteraction Hook
 *
 * Custom hook for canvas zoom, pan, and drag interactions
 * Useful for image viewers, canvas editors, and similar components
 *
 * @example
 * ```tsx
 * const { zoom, pan, isDragging, handlers } = useCanvasInteraction();
 *
 * return (
 *   <div
 *     onWheel={handlers.onWheel}
 *     onMouseDown={handlers.onMouseDown}
 *     onMouseMove={handlers.onMouseMove}
 *     onMouseUp={handlers.onMouseUp}
 *     onMouseLeave={handlers.onMouseUp}
 *     style={{ cursor: isDragging ? 'grabbing' : 'grab' }}
 *   >
 *     <canvas style={{
 *       transform: `scale(${zoom}) translate(${pan.x / zoom}px, ${pan.y / zoom}px)`,
 *       transformOrigin: 'top left',
 *     }} />
 *   </div>
 * );
 * ```
 */

import { useState, useCallback, useRef } from "react";

export interface CanvasInteractionState {
  /** Current zoom level (1 = 100%) */
  zoom: number;
  /** Current pan offset in pixels */
  pan: { x: number; y: number };
  /** Whether the user is currently dragging */
  isDragging: boolean;
  /** Zoom in */
  zoomIn: () => void;
  /** Zoom out */
  zoomOut: () => void;
  /** Reset zoom to 1 and pan to 0,0 */
  resetZoom: () => void;
  /** Set zoom to a specific value */
  setZoom: (zoom: number) => void;
  /** Set pan to specific coordinates */
  setPan: (pan: { x: number; y: number }) => void;
}

export interface CanvasInteractionHandlers {
  /** Handle mouse wheel for zooming */
  onWheel: (e: React.WheelEvent | WheelEvent) => void;
  /** Handle mouse down for starting drag */
  onMouseDown: (e: React.MouseEvent | MouseEvent) => void;
  /** Handle mouse move for panning */
  onMouseMove: (e: React.MouseEvent | MouseEvent) => void;
  /** Handle mouse up for ending drag */
  onMouseUp: (e: React.MouseEvent | MouseEvent) => void;
}

export interface UseCanvasInteractionOptions {
  /** Minimum zoom level (default: 0.25) */
  minZoom?: number;
  /** Maximum zoom level (default: 4) */
  maxZoom?: number;
  /** Zoom step for in/out (default: 0.25) */
  zoomStep?: number;
  /** Initial zoom level (default: 1) */
  initialZoom?: number;
  /** Initial pan position (default: { x: 0, y: 0 }) */
  initialPan?: { x: number; y: number };
  /** Whether to require Ctrl/Cmd key for wheel zoom (default: true) */
  requireModifierKey?: boolean;
}

/**
 * Hook for canvas zoom, pan, and drag interactions
 *
 * @param options - Configuration options
 * @returns Object containing state and handlers
 */
export function useCanvasInteraction(
  options: UseCanvasInteractionOptions = {},
): CanvasInteractionState & { handlers: CanvasInteractionHandlers } {
  const {
    minZoom = 0.25,
    maxZoom = 4,
    zoomStep = 0.25,
    initialZoom = 1,
    initialPan = { x: 0, y: 0 },
    requireModifierKey = true,
  } = options;

  const [zoom, setZoom] = useState(initialZoom);
  const [pan, setPan] = useState(initialPan);
  const [isDragging, setIsDragging] = useState(false);
  const dragStartRef = useRef<{ x: number; y: number }>({ x: 0, y: 0 });

  // Zoom in
  const zoomIn = useCallback(() => {
    setZoom((z) => Math.min(maxZoom, z + zoomStep));
  }, [maxZoom, zoomStep]);

  // Zoom out
  const zoomOut = useCallback(() => {
    setZoom((z) => Math.max(minZoom, z - zoomStep));
  }, [minZoom, zoomStep]);

  // Reset zoom and pan
  const resetZoom = useCallback(() => {
    setZoom(initialZoom);
    setPan(initialPan);
  }, [initialZoom, initialPan]);

  // Handle mouse wheel for zooming
  const onWheel = useCallback(
    (e: React.WheelEvent | WheelEvent) => {
      // Check if modifier key is required and pressed
      const hasModifier = "ctrlKey" in e ? e.ctrlKey || e.metaKey : false;

      if (!requireModifierKey || hasModifier) {
        e.preventDefault();
        const delta = "deltaY" in e && e.deltaY > 0 ? -zoomStep : zoomStep;
        setZoom((z) => Math.max(minZoom, Math.min(maxZoom, z + delta)));
      }
    },
    [minZoom, maxZoom, zoomStep, requireModifierKey],
  );

  // Handle mouse down for starting drag
  const onMouseDown = useCallback(
    (e: React.MouseEvent | MouseEvent) => {
      const button = "button" in e ? e.button : 0;
      if (button === 0 || button === 1) {
        setIsDragging(true);
        const clientX = "clientX" in e ? e.clientX : 0;
        const clientY = "clientY" in e ? e.clientY : 0;
        dragStartRef.current = {
          x: clientX - pan.x,
          y: clientY - pan.y,
        };
      }
    },
    [pan],
  );

  // Handle mouse move for panning
  const onMouseMove = useCallback(
    (e: React.MouseEvent | MouseEvent) => {
      if (isDragging) {
        const clientX = "clientX" in e ? e.clientX : 0;
        const clientY = "clientY" in e ? e.clientY : 0;
        setPan({
          x: clientX - dragStartRef.current.x,
          y: clientY - dragStartRef.current.y,
        });
      }
    },
    [isDragging],
  );

  // Handle mouse up for ending drag
  const onMouseUp = useCallback(() => {
    setIsDragging(false);
  }, []);

  return {
    zoom,
    pan,
    isDragging,
    zoomIn,
    zoomOut,
    resetZoom,
    setZoom,
    setPan,
    handlers: {
      onWheel,
      onMouseDown,
      onMouseMove,
      onMouseUp,
    },
  };
}

export default useCanvasInteraction;
