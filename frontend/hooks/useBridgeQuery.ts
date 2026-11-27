/**
 * useBridgeQuery
 *
 * One place for the "call the sidecar/bridge -> data/loading/error -> ignore a stale response"
 * effect that views used to hand-write with a `let cancelled = false` flag. Re-runs `fetcher`
 * whenever `deps` (or `enabled`) change; a response that arrives after the deps changed again or
 * after unmount is dropped and never touches state.
 *
 * Failure policy is explicit because the views differ: `keepPreviousOnError` keeps the last
 * successful `data` (default: clear it to `null`). Between a deps change and the new answer the
 * previous `data` stays visible in both cases (stale-while-loading), matching what the views did.
 */

import { useEffect, useRef, useState, type DependencyList } from "react";

export interface UseBridgeQueryOptions {
  /** `false` skips the request. Default `true`. */
  enabled?: boolean;
  /** Called for a failure of the current (not stale) request, before state is updated. */
  onError?: (error: unknown) => void;
  /** Keep the last successful `data` when a request fails. Default `false` (data becomes `null`). */
  keepPreviousOnError?: boolean;
  /** Keep `data` when `enabled` turns `false`. Default `false` (data becomes `null`). */
  keepDataWhenDisabled?: boolean;
}

export interface UseBridgeQueryResult<T> {
  data: T | null;
  loading: boolean;
  /** The current request's failure; `null` while loading or after a success. */
  error: unknown;
}

export function useBridgeQuery<T>(
  fetcher: (signal: AbortSignal) => Promise<T>,
  deps: DependencyList,
  options: UseBridgeQueryOptions = {},
): UseBridgeQueryResult<T> {
  const {
    enabled = true,
    keepPreviousOnError = false,
    keepDataWhenDisabled = false,
  } = options;
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<unknown>(null);

  // Latest callbacks without making them effect dependencies: callers pass inline closures.
  const fetcherRef = useRef(fetcher);
  const onErrorRef = useRef(options.onError);
  fetcherRef.current = fetcher;
  onErrorRef.current = options.onError;

  useEffect(() => {
    if (!enabled) {
      if (!keepDataWhenDisabled) setData(null);
      setLoading(false);
      setError(null);
      return;
    }

    const controller = new AbortController();
    setLoading(true);
    setError(null);

    let promise: Promise<T>;
    try {
      promise = Promise.resolve(fetcherRef.current(controller.signal));
    } catch (err) {
      promise = Promise.reject(err);
    }
    promise.then(
      (result) => {
        if (controller.signal.aborted) return;
        setData(result);
        setLoading(false);
      },
      (err: unknown) => {
        if (controller.signal.aborted) return;
        onErrorRef.current?.(err);
        if (!keepPreviousOnError) setData(null);
        setError(err);
        setLoading(false);
      },
    );

    return () => controller.abort();
    // `deps` is the caller's dependency list, spread on purpose.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [enabled, keepPreviousOnError, keepDataWhenDisabled, ...deps]);

  return { data, loading, error };
}
