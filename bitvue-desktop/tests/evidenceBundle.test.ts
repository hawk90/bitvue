import { describe, it, expect, vi } from "vitest";
import {
  MAX_SCREENSHOT_CHARS,
  exportEvidenceBundleViaDialog,
  validateEvidenceRequest,
} from "../electron/evidenceBundle.js";

const PNG = "data:image/png;base64,AAAA";

function deps(chosen: string | null, result: unknown = { success: true }) {
  return {
    chooseDirectory: vi.fn(async () => chosen),
    exportBundle: vi.fn(async () => result),
  };
}

describe("exportEvidenceBundleViaDialog (#640)", () => {
  it("exports into the folder the dialog returned, and only that folder", async () => {
    const d = deps("/Users/me/bundles", { success: true, bundle_path: "/x" });
    const result = await exportEvidenceBundleViaDialog(
      {
        workspace: "player",
        mode: "normal",
        orderType: "display",
        screenshotDataUrl: PNG,
      },
      d,
    );
    expect(result).toEqual({ success: true, bundle_path: "/x" });
    expect(d.exportBundle).toHaveBeenCalledTimes(1);
    expect(d.exportBundle).toHaveBeenCalledWith({
      output_dir: "/Users/me/bundles",
      workspace: "player",
      mode: "normal",
      order_type: "display",
      screenshot_data_url: PNG,
    });
  });

  it("ignores a directory smuggled into the request: only the dialog decides", async () => {
    const d = deps("/chosen/by/user");
    await exportEvidenceBundleViaDialog(
      {
        outputDir: "/etc",
        output_dir: "/etc",
        path: "/etc",
        workspace: "player",
      },
      d,
    );
    expect(d.exportBundle).toHaveBeenCalledTimes(1);
    const [params] = d.exportBundle.mock.calls[0] as unknown as [
      { output_dir: string },
    ];
    expect(params.output_dir).toBe("/chosen/by/user");
  });

  it("exports nothing and returns null when the user cancels", async () => {
    for (const cancelled of [null, ""]) {
      const d = deps(cancelled);
      expect(await exportEvidenceBundleViaDialog({}, d)).toBeNull();
      expect(d.exportBundle).not.toHaveBeenCalled();
    }
  });

  it("shows the dialog only after the request has been validated", async () => {
    const d = deps("/x");
    await expect(
      exportEvidenceBundleViaDialog({ orderType: "sideways" }, d),
    ).rejects.toThrow(/exportEvidenceBundle/);
    expect(d.chooseDirectory).not.toHaveBeenCalled();
    expect(d.exportBundle).not.toHaveBeenCalled();
  });
});

describe("validateEvidenceRequest", () => {
  it("accepts an empty object and the documented fields", () => {
    expect(validateEvidenceRequest({})).toEqual({
      workspace: undefined,
      mode: undefined,
      orderType: undefined,
      screenshotDataUrl: undefined,
    });
    expect(
      validateEvidenceRequest({ orderType: "decode", workspace: "compare" }),
    ).toMatchObject({ orderType: "decode", workspace: "compare" });
  });

  it("rejects what is not an object, a bad order type, long or non-string fields", () => {
    const bad: unknown[] = [
      null,
      "x",
      5,
      { orderType: "x" },
      { workspace: 5 },
      { mode: "m".repeat(65) },
    ];
    for (const b of bad) {
      expect(() => validateEvidenceRequest(b)).toThrow(/exportEvidenceBundle/);
    }
  });

  it("accepts only a PNG data URL within the size limit as the screenshot", () => {
    expect(() =>
      validateEvidenceRequest({ screenshotDataUrl: PNG }),
    ).not.toThrow();
    for (const s of [
      5,
      "data:text/html;base64,AAAA",
      "/etc/passwd",
      "file:///etc/passwd",
      PNG + "A".repeat(MAX_SCREENSHOT_CHARS),
    ]) {
      expect(() => validateEvidenceRequest({ screenshotDataUrl: s })).toThrow(
        /screenshotDataUrl/,
      );
    }
  });
});
