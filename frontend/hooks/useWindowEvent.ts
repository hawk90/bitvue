/**
 * useWindowEvent Hook
 *
 * Custom hook for subscribing to window events with automatic cleanup.
 *
 * Usage:
 * ```tsx
 * useWindowEvent('keydown', (e) => {
 *   if (e.key === 'Escape') handleClose();
 * });
 *
 * // With options
 * useWindowEvent('resize', handleResize, { passive: true });
 * ```
 */

import { useEffect, useRef } from 'react';

/**
 * Subscribe to a window event with automatic cleanup on unmount
 *
 * @param type - The event type to listen for
 * @param handler - The event handler function
 * @param options - Optional event listener options
 */
export function useWindowEvent<K extends keyof WindowEventMap>(
  type: K,
  handler: (event: WindowEventMap[K]) => void,
  options?: AddEventListenerOptions
): void {
  // Keep track of the latest handler using ref to avoid re-adding listener
  const handlerRef = useRef(handler);
  handlerRef.current = handler;

  useEffect(() => {
    // Create a wrapper that calls the latest handler from ref
    const eventListener = (event: Event) => {
      handlerRef.current(event as WindowEventMap[K]);
    };

    window.addEventListener(type, eventListener, options);

    return () => {
      window.removeEventListener(type, eventListener);
    };
  }, [type, options]);
}

/**
 * Subscribe to multiple window events with automatic cleanup
 *
 * Usage:
 * ```tsx
 * useWindowEvents({
 *   keydown: handleKeyDown,
 *   resize: handleResize,
 *   focus: handleFocus,
 * });
 * ```
 */
export function useWindowEvents<K extends keyof WindowEventMap>(
  events: Partial<Record<K, (event: WindowEventMap[K]) => void>>,
  options?: AddEventListenerOptions
): void {
  const handlerRefs = useRef(events);
  handlerRefs.current = events;

  useEffect(() => {
    const listeners: Array<{ type: string; listener: (event: Event) => void }> = [];

    for (const [type, _handler] of Object.entries(events)) {
      const eventListener = (event: Event) => {
        const currentHandler = handlerRefs.current[type as K];
        if (currentHandler) {
          (currentHandler as (event: Event) => void)(event);
        }
      };

      window.addEventListener(type, eventListener, options);
      listeners.push({ type, listener: eventListener });
    }

    return () => {
      for (const { type, listener } of listeners) {
        window.removeEventListener(type, listener);
      }
    };
  }, [events, options]);
}

export default useWindowEvent;
