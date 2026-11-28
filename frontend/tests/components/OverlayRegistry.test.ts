/**
 * The overlay renderer registry (components/panels/OverlayRenderer/registry.ts) and the tables
 * derived from it. Canvas behaviour of the dispatch itself is pinned by
 * OverlayModeDispatch.test.ts; this pins the registry's declared shape and the derived sets.
 */

import { describe, it, expect } from "vitest";
import {
  INFO_OVERLAY_RENDERERS,
  MAIN_MODES_NEEDING_AV1_FEATURES,
  MAIN_OVERLAY_RENDERERS,
} from "@/components/panels/OverlayRenderer/registry";
import { AV1_FEATURE_MODES } from "@/hooks/useAv1Features";

describe("overlay registry", () => {
  it("main table: modes and their explicit saveRestore flag", () => {
    const table = Object.fromEntries(
      Object.entries(MAIN_OVERLAY_RENDERERS).map(([m, s]) => [
        m,
        s.saveRestore,
      ]),
    );
    expect(table).toEqual({
      "coding-flow": false,
      prediction: false,
      transform: false, // the JPEG XS branch wraps itself
      "qp-map": false,
      "mv-field": false,
      reference: false,
      "cdef-filter": true,
      "loop-restoration": true,
      "film-grain": true,
      "super-res": true,
      "dual-tree": true,
      "adaptive-filter": true,
      "inverse-map": true,
      esao: true,
      ccsao: true,
      precinct: true,
      dequant: true,
      mct: true,
      nlt: true,
      segment: true,
    });
  });

  it("info table: modes (the outer save/alpha/restore wrapper is the dispatcher's)", () => {
    expect(Object.keys(INFO_OVERLAY_RENDERERS)).toEqual([
      "qp-map",
      "mv-field",
      "heat-map",
      "block-type",
      "efficiency-map",
      "mb-type",
      "reference-indices",
    ]);
    for (const spec of Object.values(INFO_OVERLAY_RENDERERS)) {
      expect(spec.saveRestore).toBe(false);
    }
  });

  it("AV1_FEATURE_MODES derived from the registry equals the old hand-written set, in order", () => {
    expect([...AV1_FEATURE_MODES]).toEqual([
      "cdef-filter",
      "loop-restoration",
      "film-grain",
      "super-res",
      "av1-features",
    ]);
    expect([...MAIN_MODES_NEEDING_AV1_FEATURES]).toEqual([
      "cdef-filter",
      "loop-restoration",
      "film-grain",
      "super-res",
    ]);
  });
});
