/**
 * useRecentFiles Hook Tests
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useRecentFiles } from "@/hooks/useRecentFiles";

describe("useRecentFiles", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  afterEach(() => {
    localStorage.clear();
  });

  it("starts empty when no saved state", () => {
    const { result } = renderHook(() => useRecentFiles());
    expect(result.current.recentFiles).toEqual([]);
  });

  it("adds a file to the front of the list", () => {
    const { result } = renderHook(() => useRecentFiles());
    act(() => {
      result.current.addRecentFile("/foo/bar.ivf");
    });
    expect(result.current.recentFiles[0]).toBe("/foo/bar.ivf");
  });

  it("moves an existing file to the front", () => {
    const { result } = renderHook(() => useRecentFiles());
    act(() => {
      result.current.addRecentFile("/a.ivf");
      result.current.addRecentFile("/b.ivf");
      result.current.addRecentFile("/a.ivf"); // re-add
    });
    expect(result.current.recentFiles[0]).toBe("/a.ivf");
    expect(result.current.recentFiles).toHaveLength(2); // no duplicate
  });

  it("limits list to 10 entries", () => {
    const { result } = renderHook(() => useRecentFiles());
    act(() => {
      for (let i = 0; i < 15; i++) {
        result.current.addRecentFile(`/file${i}.ivf`);
      }
    });
    expect(result.current.recentFiles).toHaveLength(10);
  });

  it("removes a specific file", () => {
    const { result } = renderHook(() => useRecentFiles());
    act(() => {
      result.current.addRecentFile("/a.ivf");
      result.current.addRecentFile("/b.ivf");
      result.current.removeRecentFile("/a.ivf");
    });
    expect(result.current.recentFiles).not.toContain("/a.ivf");
    expect(result.current.recentFiles).toContain("/b.ivf");
  });

  it("clears all files", () => {
    const { result } = renderHook(() => useRecentFiles());
    act(() => {
      result.current.addRecentFile("/a.ivf");
      result.current.addRecentFile("/b.ivf");
      result.current.clearRecentFiles();
    });
    expect(result.current.recentFiles).toHaveLength(0);
  });

  it("persists to and loads from localStorage", () => {
    const { result: r1 } = renderHook(() => useRecentFiles());
    act(() => {
      r1.current.addRecentFile("/persist.ivf");
    });

    // Fresh hook should see the saved data
    const { result: r2 } = renderHook(() => useRecentFiles());
    expect(r2.current.recentFiles[0]).toBe("/persist.ivf");
  });
});
