import { describe, it, expect } from "vitest";
import {
  PREDICTION_MODE_COLORS,
  getPredictionModeName,
} from "../../components/panels/OverlayRenderer/utils/colors";

// Wire codes are produced by the sidecar's `prediction_mode_to_wire` (frame_analysis.rs).
describe("getPredictionModeName — AV1 inter modes", () => {
  it.each([
    [64, "NEWMV"],
    [65, "NEARESTMV"],
    [66, "NEARMV"],
    [68, "GLOBALMV"],
    [69, "Compound"],
  ])("code %i -> %s", (code, name) => {
    expect(getPredictionModeName(code)).toBe(name);
  });

  it("never yields a generic InterNN label for a code the sidecar emits", () => {
    for (const code of [64, 65, 66, 68, 69]) {
      expect(getPredictionModeName(code)).not.toMatch(/^Inter\d+$/);
      expect(PREDICTION_MODE_COLORS[code]).toBeDefined();
    }
  });
});
