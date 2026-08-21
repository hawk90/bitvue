/**
 * useWelcomeActions Hook Tests
 */

import { describe, it, expect, vi } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";
import { useWelcomeActions } from "@/hooks/useWelcomeActions";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

describe("useWelcomeActions", () => {
  it("sets isOpening for the duration of handleOpenFile, then clears it", async () => {
    const { promise, resolve } = deferred<void>();
    const onOpenFile = vi.fn(() => promise);
    const openFileAtPath = vi.fn();

    const { result } = renderHook(() =>
      useWelcomeActions(onOpenFile, openFileAtPath),
    );

    expect(result.current.isOpening).toBe(false);

    let callPromise!: Promise<void>;
    act(() => {
      callPromise = result.current.handleOpenFile();
    });
    expect(result.current.isOpening).toBe(true);

    resolve();
    await act(async () => {
      await callPromise;
    });
    expect(result.current.isOpening).toBe(false);
  });

  it("ignores a second handleOpenFile call while one is already in flight", async () => {
    const { promise, resolve } = deferred<void>();
    const onOpenFile = vi.fn(() => promise);
    const openFileAtPath = vi.fn();

    const { result } = renderHook(() =>
      useWelcomeActions(onOpenFile, openFileAtPath),
    );

    act(() => {
      void result.current.handleOpenFile();
    });
    act(() => {
      void result.current.handleOpenFile();
    });
    expect(onOpenFile).toHaveBeenCalledTimes(1);

    resolve();
    await waitFor(() => expect(result.current.isOpening).toBe(false));
  });

  it("ignores handleOpenRecent while handleOpenFile is in flight (shared guard)", async () => {
    const { promise, resolve } = deferred<void>();
    const onOpenFile = vi.fn(() => promise);
    const openFileAtPath = vi.fn();

    const { result } = renderHook(() =>
      useWelcomeActions(onOpenFile, openFileAtPath),
    );

    act(() => {
      void result.current.handleOpenFile();
    });
    await act(async () => {
      await result.current.handleOpenRecent("/a/file.ivf");
    });
    expect(openFileAtPath).not.toHaveBeenCalled();

    resolve();
    await waitFor(() => expect(result.current.isOpening).toBe(false));
  });

  it("clears isOpening even when the open operation rejects", async () => {
    const { promise, reject } = deferred<void>();
    const onOpenFile = vi.fn(() => promise);
    const openFileAtPath = vi.fn();

    const { result } = renderHook(() =>
      useWelcomeActions(onOpenFile, openFileAtPath),
    );

    let callPromise!: Promise<void>;
    act(() => {
      callPromise = result.current.handleOpenFile();
    });
    expect(result.current.isOpening).toBe(true);

    reject(new Error("open failed"));
    // handleOpenFile itself doesn't catch -- the underlying onOpenFile (real: openFileAtPath's
    // own try/catch) is what's responsible for not throwing. This hook only guarantees the
    // `finally` cleanup runs regardless.
    await expect(callPromise).rejects.toThrow("open failed");
    await waitFor(() => expect(result.current.isOpening).toBe(false));
  });

  it("calls openFileAtPath with the given path on handleOpenRecent", async () => {
    const onOpenFile = vi.fn(async () => {});
    const openFileAtPath = vi.fn(async () => {});

    const { result } = renderHook(() =>
      useWelcomeActions(onOpenFile, openFileAtPath),
    );

    await act(async () => {
      await result.current.handleOpenRecent("/a/file.ivf");
    });
    expect(openFileAtPath).toHaveBeenCalledWith("/a/file.ivf");
  });
});
