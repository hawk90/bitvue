/**
 * useIntersectionObserver Hook
 *
 * Custom hook for using IntersectionObserver API with React
 * Useful for lazy loading, infinite scroll, and visibility detection
 *
 * @example
 * ```tsx
 * const ref = useRef<HTMLDivElement>(null);
 * const isVisible = useIntersectionObserver(ref, { threshold: 0.5 });
 *
 * return <div ref={ref}>{isVisible ? 'Visible!' : 'Not visible'}</div>;
 * ```
 */

import { useEffect, useState, useRef, RefObject } from "react";

export interface UseIntersectionObserverOptions {
  /** The element's intersection rectangle with the root */
  root?: Element | null;
  /** The element that is used as the viewport for checking visibility */
  rootMargin?: string;
  /** A threshold of 0-1 at which to trigger the callback */
  threshold?: number | number[];
  /** Whether to disconnect the observer on unmount */
  triggerOnce?: boolean;
}

export interface UseIntersectionObserverResult {
  /** The ref to attach to the target element */
  ref: RefObject<Element>;
  /** Whether the element is currently intersecting */
  isVisible: boolean;
  /** The entry object from IntersectionObserver */
  entry?: IntersectionObserverEntry;
}

/**
 * Hook for using IntersectionObserver API
 *
 * @param options - IntersectionObserver options
 * @returns Object containing ref and visibility state
 */
export function useIntersectionObserver(
  options: UseIntersectionObserverOptions = {},
): UseIntersectionObserverResult {
  const {
    root = null,
    rootMargin = "0px",
    threshold = 0,
    triggerOnce = false,
  } = options;

  const ref = useRef<Element>(null);
  const [isVisible, setIsVisible] = useState(false);
  const [entry, setEntry] = useState<IntersectionObserverEntry>();

  useEffect(() => {
    const element = ref.current;
    if (!element) return;

    // Create observer
    const observer = new IntersectionObserver(
      ([entry]) => {
        setIsVisible(entry.isIntersecting);
        setEntry(entry);

        // Disconnect after first trigger if triggerOnce is true
        if (triggerOnce && entry.isIntersecting) {
          observer.disconnect();
        }
      },
      { root, rootMargin, threshold },
    );

    // Start observing
    observer.observe(element);

    // Cleanup
    return () => {
      observer.disconnect();
    };
  }, [root, rootMargin, threshold, triggerOnce]);

  return { ref, isVisible, entry };
}

/**
 * Alternative hook that accepts a ref as parameter
 * Useful when you need to control the ref externally
 *
 * @param targetRef - The ref to observe
 * @param options - IntersectionObserver options
 * @returns Whether the element is intersecting
 */
export function useIntersectionObserverRef(
  targetRef: RefObject<Element>,
  options: UseIntersectionObserverOptions = {},
): boolean {
  const {
    root = null,
    rootMargin = "0px",
    threshold = 0,
    triggerOnce = false,
  } = options;

  const [isVisible, setIsVisible] = useState(false);

  useEffect(() => {
    const element = targetRef.current;
    if (!element) return;

    // Create observer
    const observer = new IntersectionObserver(
      ([entry]) => {
        setIsVisible(entry.isIntersecting);

        // Disconnect after first trigger if triggerOnce is true
        if (triggerOnce && entry.isIntersecting) {
          observer.disconnect();
        }
      },
      { root, rootMargin, threshold },
    );

    // Start observing
    observer.observe(element);

    // Cleanup
    return () => {
      observer.disconnect();
    };
  }, [root, rootMargin, threshold, triggerOnce]);

  return isVisible;
}

export default useIntersectionObserver;
