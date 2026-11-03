import { describe, expect, it } from "vitest";
import {
  getInfoOverlaysForCodec,
  getMainModesForCodec,
} from "../../utils/codecModeRegistry";

describe("AV1 mode registry", () => {
  it("offers the QP map as an info overlay, as the other codecs do (#446)", () => {
    const overlays = getInfoOverlaysForCodec("AV1").map((e) => e.mode);
    expect(overlays).toContain("qp-map");
    // It is an overlay on top of a main mode, not a main mode of its own: F5 stays Loop Filter.
    expect(getMainModesForCodec("AV1").map((e) => e.mode)).not.toContain(
      "qp-map",
    );
  });

  it("keeps the other AV1 overlays", () => {
    expect(getInfoOverlaysForCodec("AV1").map((e) => e.mode)).toEqual([
      "qp-map",
      "heat-map",
      "block-type",
      "efficiency-map",
      "psnr-overlay",
    ]);
  });

  it("matches the codec case-insensitively, like the HEVC list which also has it", () => {
    expect(getInfoOverlaysForCodec("av1")).toEqual(
      getInfoOverlaysForCodec("AV1"),
    );
    expect(getInfoOverlaysForCodec("HEVC").map((e) => e.mode)[0]).toBe(
      "qp-map",
    );
  });
});
