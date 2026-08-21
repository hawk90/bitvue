/**
 * useSamplePaths Hook Tests
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { useSamplePaths } from "@/hooks/useSamplePaths";
import { getSamplePath } from "@/services/electronBridgeService";

vi.mock("@/services/electronBridgeService", () => ({
  getSamplePath: vi.fn(),
}));

describe("useSamplePaths", () => {
  beforeEach(() => {
    vi.mocked(getSamplePath).mockReset();
  });

  it("resolves each filename to its real path", async () => {
    vi.mocked(getSamplePath).mockImplementation(
      async (filename: string) => `/samples/${filename}`,
    );
    const filenames = ["foreman_av1.ivf"];

    const { result } = renderHook(() => useSamplePaths(filenames));

    await waitFor(() => {
      expect(result.current["foreman_av1.ivf"]).toBe(
        "/samples/foreman_av1.ivf",
      );
    });
  });

  it("resolves to null (not a crash) when the bridge call rejects", async () => {
    vi.mocked(getSamplePath).mockRejectedValue(new Error("no bridge"));
    const filenames = ["foreman_av1.ivf"];

    const { result } = renderHook(() => useSamplePaths(filenames));

    await waitFor(() => {
      expect(result.current["foreman_av1.ivf"]).toBeNull();
    });
  });

  it("returns an empty map for an empty filenames list", () => {
    const filenames: string[] = [];
    const { result } = renderHook(() => useSamplePaths(filenames));
    expect(result.current).toEqual({});
    expect(getSamplePath).not.toHaveBeenCalled();
  });
});
