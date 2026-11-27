import { describe, it, expect, vi, afterEach } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";
import { useBridgeQuery } from "@/hooks/useBridgeQuery";

interface Deferred<T> {
  promise: Promise<T>;
  resolve: (value: T) => void;
  reject: (error: unknown) => void;
}

function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("useBridgeQuery", () => {
  it("loads data and clears loading", async () => {
    const { result } = renderHook(() => useBridgeQuery(async () => "ok", []));
    await waitFor(() => expect(result.current.data).toBe("ok"));
    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it("is loading while the request is pending", async () => {
    const d = deferred<string>();
    const { result } = renderHook(() => useBridgeQuery(() => d.promise, []));
    await waitFor(() => expect(result.current.loading).toBe(true));
    expect(result.current.data).toBeNull();
    await act(async () => d.resolve("done"));
    expect(result.current.loading).toBe(false);
    expect(result.current.data).toBe("done");
  });

  it("ignores a late response of an earlier deps value", async () => {
    const first = deferred<string>();
    const second = deferred<string>();
    const pending = [first, second];
    const fetcher = vi.fn((_signal: AbortSignal) => pending.shift()!.promise);
    const { result, rerender } = renderHook(
      ({ n }) => useBridgeQuery(fetcher, [n]),
      { initialProps: { n: 1 } },
    );
    rerender({ n: 2 });
    expect(fetcher).toHaveBeenCalledTimes(2);
    expect(fetcher.mock.calls[0][0].aborted).toBe(true);
    await act(async () => second.resolve("second"));
    expect(result.current.data).toBe("second");
    await act(async () => first.resolve("first"));
    expect(result.current.data).toBe("second");
    expect(result.current.loading).toBe(false);
  });

  it("ignores a late failure of an earlier deps value", async () => {
    const first = deferred<string>();
    const second = deferred<string>();
    const pending = [first, second];
    const onError = vi.fn();
    const { result, rerender } = renderHook(
      ({ n }) =>
        useBridgeQuery(() => pending.shift()!.promise, [n], { onError }),
      { initialProps: { n: 1 } },
    );
    rerender({ n: 2 });
    await act(async () => second.resolve("second"));
    await act(async () => first.reject(new Error("late")));
    expect(onError).not.toHaveBeenCalled();
    expect(result.current.data).toBe("second");
    expect(result.current.error).toBeNull();
  });

  it("keeps the previous data while the next request is pending", async () => {
    const next = deferred<number>();
    const { result, rerender } = renderHook(
      ({ n }) =>
        useBridgeQuery(
          () => (n === 1 ? Promise.resolve(1) : next.promise),
          [n],
        ),
      { initialProps: { n: 1 } },
    );
    await waitFor(() => expect(result.current.data).toBe(1));
    rerender({ n: 2 });
    expect(result.current.data).toBe(1);
    await waitFor(() => expect(result.current.loading).toBe(true));
    await act(async () => next.resolve(2));
    expect(result.current.data).toBe(2);
  });

  it("does not update state or warn after unmount", async () => {
    const d = deferred<string>();
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    const onError = vi.fn();
    const { unmount } = renderHook(() =>
      useBridgeQuery(() => d.promise, [], { onError }),
    );
    unmount();
    await act(async () => d.reject(new Error("after unmount")));
    expect(onError).not.toHaveBeenCalled();
    expect(errorSpy).not.toHaveBeenCalled();
  });

  it("aborts the signal on unmount", () => {
    let seen: AbortSignal | undefined;
    const { unmount } = renderHook(() =>
      useBridgeQuery((signal) => {
        seen = signal;
        return new Promise<string>(() => {});
      }, []),
    );
    expect(seen?.aborted).toBe(false);
    unmount();
    expect(seen?.aborted).toBe(true);
  });

  it("does not call the fetcher when enabled is false", () => {
    const fetcher = vi.fn(async () => "x");
    const { result } = renderHook(() =>
      useBridgeQuery(fetcher, [], { enabled: false }),
    );
    expect(fetcher).not.toHaveBeenCalled();
    expect(result.current).toEqual({ data: null, loading: false, error: null });
  });

  it("fetches once enabled turns true", async () => {
    const fetcher = vi.fn(async () => "x");
    const { result, rerender } = renderHook(
      ({ enabled }) => useBridgeQuery(fetcher, [], { enabled }),
      { initialProps: { enabled: false } },
    );
    rerender({ enabled: true });
    await waitFor(() => expect(result.current.data).toBe("x"));
    expect(fetcher).toHaveBeenCalledTimes(1);
  });

  it("clears data when disabled by default, keeps it with keepDataWhenDisabled", async () => {
    const clearing = renderHook(
      ({ enabled }) => useBridgeQuery(async () => "x", [], { enabled }),
      { initialProps: { enabled: true } },
    );
    await waitFor(() => expect(clearing.result.current.data).toBe("x"));
    clearing.rerender({ enabled: false });
    expect(clearing.result.current.data).toBeNull();

    const keeping = renderHook(
      ({ enabled }) =>
        useBridgeQuery(async () => "x", [], {
          enabled,
          keepDataWhenDisabled: true,
        }),
      { initialProps: { enabled: true } },
    );
    await waitFor(() => expect(keeping.result.current.data).toBe("x"));
    keeping.rerender({ enabled: false });
    expect(keeping.result.current.data).toBe("x");
  });

  it("reports an error through error and onError, clearing data by default", async () => {
    const boom = new Error("boom");
    const onError = vi.fn();
    const { result, rerender } = renderHook(
      ({ n }) =>
        useBridgeQuery(
          async () => {
            if (n === 2) throw boom;
            return "ok";
          },
          [n],
          { onError },
        ),
      { initialProps: { n: 1 } },
    );
    await waitFor(() => expect(result.current.data).toBe("ok"));
    rerender({ n: 2 });
    await waitFor(() => expect(result.current.error).toBe(boom));
    expect(onError).toHaveBeenCalledWith(boom);
    expect(result.current.data).toBeNull();
    expect(result.current.loading).toBe(false);
  });

  it("keeps the previous data on error with keepPreviousOnError", async () => {
    const boom = new Error("boom");
    const { result, rerender } = renderHook(
      ({ n }) =>
        useBridgeQuery(
          async () => {
            if (n === 2) throw boom;
            return "ok";
          },
          [n],
          { keepPreviousOnError: true },
        ),
      { initialProps: { n: 1 } },
    );
    await waitFor(() => expect(result.current.data).toBe("ok"));
    rerender({ n: 2 });
    await waitFor(() => expect(result.current.error).toBe(boom));
    expect(result.current.data).toBe("ok");
  });

  it("treats a synchronous throw in the fetcher as an error", async () => {
    const boom = new Error("sync");
    const { result } = renderHook(() =>
      useBridgeQuery<string>(() => {
        throw boom;
      }, []),
    );
    await waitFor(() => expect(result.current.error).toBe(boom));
    expect(result.current.loading).toBe(false);
  });

  it("uses the latest fetcher without refetching on every render", async () => {
    const fetcher = vi.fn(async () => "x");
    const { result, rerender } = renderHook(() =>
      useBridgeQuery(() => fetcher(), []),
    );
    await waitFor(() => expect(result.current.data).toBe("x"));
    rerender();
    rerender();
    expect(fetcher).toHaveBeenCalledTimes(1);
  });
});
