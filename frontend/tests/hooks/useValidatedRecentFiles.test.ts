/**
 * useValidatedRecentFiles Hook Tests
 *
 * Every test passes `recentFiles` as a reference that's stable across re-renders (hoisted to a
 * variable, or via renderHook's `initialProps`/`rerender`) -- never an inline array literal
 * constructed directly inside the `renderHook(() => ...)` callback. That callback re-runs on
 * every render this hook triggers, so an inline literal would be a *new* array reference each
 * time, and since the hook's effect is keyed on `recentFiles` by reference (matching how the real
 * caller, `useRecentFiles`, provides a `useState`-backed stable reference), that mismatch caused a
 * genuine infinite render loop the first time this file was written -- not a bug in the hook, but
 * worth this comment so it isn't reintroduced.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { useValidatedRecentFiles } from "@/hooks/useValidatedRecentFiles";
import { pathExists } from "@/services/electronBridgeService";

vi.mock("@/services/electronBridgeService", () => ({
  pathExists: vi.fn(),
}));

describe("useValidatedRecentFiles", () => {
  beforeEach(() => {
    vi.mocked(pathExists).mockReset();
  });

  it("keeps a path that exists", async () => {
    vi.mocked(pathExists).mockResolvedValue(true);
    const removeRecentFile = vi.fn();
    const files = ["/a/real.ivf"];

    const { result } = renderHook(() =>
      useValidatedRecentFiles(files, removeRecentFile),
    );

    await waitFor(() => {
      expect(result.current).toEqual(["/a/real.ivf"]);
    });
    expect(removeRecentFile).not.toHaveBeenCalled();
  });

  it("prunes a path that no longer exists and persists the removal", async () => {
    vi.mocked(pathExists).mockImplementation(
      async (path: string) => path !== "/a/gone.ivf",
    );
    const removeRecentFile = vi.fn();
    const files = ["/a/real.ivf", "/a/gone.ivf"];

    const { result } = renderHook(() =>
      useValidatedRecentFiles(files, removeRecentFile),
    );

    await waitFor(() => {
      expect(result.current).toEqual(["/a/real.ivf"]);
    });
    expect(removeRecentFile).toHaveBeenCalledWith("/a/gone.ivf");
    expect(removeRecentFile).not.toHaveBeenCalledWith("/a/real.ivf");
  });

  it("does not re-check a path it already confirmed exists", async () => {
    vi.mocked(pathExists).mockResolvedValue(true);
    const removeRecentFile = vi.fn();
    const firstFiles = ["/a/real.ivf"];

    const { rerender } = renderHook(
      ({ files }) => useValidatedRecentFiles(files, removeRecentFile),
      { initialProps: { files: firstFiles } },
    );

    await waitFor(() => {
      expect(pathExists).toHaveBeenCalledTimes(1);
    });

    // A second, unrelated file gets added -- the already-confirmed one shouldn't be re-checked.
    const secondFiles = ["/b/new.ivf", "/a/real.ivf"];
    rerender({ files: secondFiles });

    await waitFor(() => {
      expect(pathExists).toHaveBeenCalledWith("/b/new.ivf");
    });
    // Exactly 2 calls total across both renders: the one legitimate initial check of
    // "/a/real.ivf" (asserted above, before the rerender) plus this one new check of
    // "/b/new.ivf" -- if the already-confirmed path were being re-checked, this would be 3.
    expect(pathExists).toHaveBeenCalledTimes(2);
  });

  it("does not apply a stale check result from a superseded recentFiles list", async () => {
    const removeRecentFile = vi.fn();
    let resolveFirstCheck!: (exists: boolean) => void;
    vi.mocked(pathExists).mockImplementation(
      () => new Promise((resolve) => (resolveFirstCheck = resolve)),
    );
    const firstFiles = ["/a/slow.ivf"];

    const { rerender } = renderHook(
      ({ files }) => useValidatedRecentFiles(files, removeRecentFile),
      { initialProps: { files: firstFiles } },
    );

    // Supersede before the first (slow) check resolves.
    vi.mocked(pathExists).mockResolvedValue(true);
    const secondFiles = ["/b/fresh.ivf"];
    rerender({ files: secondFiles });

    await waitFor(() => {
      expect(pathExists).toHaveBeenCalledWith("/b/fresh.ivf");
    });

    // The first check now resolves "doesn't exist" -- this must not prune /a/slow.ivf, since
    // it's no longer even part of the current recentFiles list this hook is tracking.
    resolveFirstCheck(false);

    await new Promise((r) => setTimeout(r, 0));
    expect(removeRecentFile).not.toHaveBeenCalledWith("/a/slow.ivf");
  });

  it("fails open (keeps the entry) when the existence check itself errors", async () => {
    vi.mocked(pathExists).mockRejectedValue(new Error("IPC broke"));
    const removeRecentFile = vi.fn();
    const files = ["/a/maybe.ivf"];

    const { result } = renderHook(() =>
      useValidatedRecentFiles(files, removeRecentFile),
    );

    await waitFor(() => {
      expect(pathExists).toHaveBeenCalled();
    });
    expect(result.current).toEqual(["/a/maybe.ivf"]);
    expect(removeRecentFile).not.toHaveBeenCalled();
  });
});
