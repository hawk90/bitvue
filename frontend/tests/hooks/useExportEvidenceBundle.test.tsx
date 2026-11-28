/**
 * useExportEvidenceBundle (#640): the renderer asks for an export and sends no output path; the
 * folder dialog belongs to the main process. A cancelled dialog is silent, a failure is reported.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";

const bridge = vi.hoisted(() => ({
  exportEvidenceBundle: vi.fn(),
  captureScreenshot: vi.fn(),
}));
vi.mock("@/services/electronBridgeService", () => bridge);

import { useExportEvidenceBundle } from "@/hooks/useExportEvidenceBundle";

describe("useExportEvidenceBundle", () => {
  let alertSpy: ReturnType<typeof vi.spyOn>;
  beforeEach(() => {
    Object.values(bridge).forEach((m) => m.mockReset());
    bridge.captureScreenshot.mockResolvedValue("data:image/png;base64,AAAA");
    alertSpy = vi.spyOn(window, "alert").mockImplementation(() => {});
  });
  afterEach(() => alertSpy.mockRestore());

  it("sends the manifest fields and the screenshot, and no output path", async () => {
    bridge.exportEvidenceBundle.mockResolvedValue({
      success: true,
      bundle_path: "/out/bundle",
    });
    const { result } = renderHook(() =>
      useExportEvidenceBundle({ workspace: "compare", mode: "diff" }),
    );
    await act(async () => {
      await result.current();
    });
    expect(bridge.exportEvidenceBundle).toHaveBeenCalledTimes(1);
    const params = bridge.exportEvidenceBundle.mock.calls[0][0];
    expect(params).toEqual({
      workspace: "compare",
      mode: "diff",
      orderType: "display",
      screenshotDataUrl: "data:image/png;base64,AAAA",
    });
    expect(params).not.toHaveProperty("outputDir");
    expect(alertSpy).toHaveBeenCalledWith(
      expect.stringContaining("/out/bundle"),
    );
  });

  it("captures the screenshot before the export (and so before the folder dialog)", async () => {
    const order: string[] = [];
    bridge.captureScreenshot.mockImplementation(async () => {
      order.push("capture");
      return "data:image/png;base64,AAAA";
    });
    bridge.exportEvidenceBundle.mockImplementation(async () => {
      order.push("export");
      return { success: true, bundle_path: "/x" };
    });
    const { result } = renderHook(() => useExportEvidenceBundle());
    await act(async () => {
      await result.current();
    });
    expect(order).toEqual(["capture", "export"]);
  });

  it("says nothing when the user cancels the folder dialog", async () => {
    bridge.exportEvidenceBundle.mockResolvedValue(null);
    const { result } = renderHook(() => useExportEvidenceBundle());
    await act(async () => {
      await result.current();
    });
    expect(alertSpy).not.toHaveBeenCalled();
  });

  it("still exports (without a screenshot) when the capture fails", async () => {
    bridge.captureScreenshot.mockRejectedValue(new Error("no window"));
    bridge.exportEvidenceBundle.mockResolvedValue({
      success: true,
      bundle_path: "/x",
    });
    const { result } = renderHook(() => useExportEvidenceBundle());
    await act(async () => {
      await result.current();
    });
    expect(
      bridge.exportEvidenceBundle.mock.calls[0][0].screenshotDataUrl,
    ).toBeUndefined();
  });

  it("reports a failed export and a thrown error", async () => {
    bridge.exportEvidenceBundle.mockResolvedValueOnce({
      success: false,
      error: "disk full",
    });
    const { result } = renderHook(() => useExportEvidenceBundle());
    await act(async () => {
      await result.current();
    });
    expect(alertSpy).toHaveBeenLastCalledWith(
      expect.stringContaining("disk full"),
    );

    bridge.exportEvidenceBundle.mockRejectedValueOnce(new Error("bad request"));
    await act(async () => {
      await result.current();
    });
    expect(alertSpy).toHaveBeenLastCalledWith(
      expect.stringContaining("bad request"),
    );
  });
});
