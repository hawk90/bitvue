/**
 * Characterization tests: codec-specific overlay renderers' canvas call sequences.
 *
 * A canvas renderer's output is its sequence of 2D-context calls and property sets, so these
 * tests record that sequence (every `ctx.x = v` and `ctx.f(args)`, in order) and snapshot it.
 * They pin the "no data" notice boxes, the `frame` field reads, and the grid-cell/grid-line
 * drawing so refactoring the duplicated drawing code into shared helpers cannot change a pixel.
 *
 * Colour strings are compared with spaces removed: `rgba(0, 0, 0, 0.70)` and `rgba(0,0,0,0.70)`
 * are the same colour to a canvas, so that spelling is not a visible difference.
 */

import { describe, it, expect } from "vitest";
import type { FrameInfo } from "@/types/video";
import type {
  Av1CdefData,
  Av1FilmGrainData,
  Av1LoopRestorationData,
  Av1SuperResData,
} from "@/components/panels/OverlayRenderer/types";
import { Avs3CcsaoRenderer } from "@/components/panels/OverlayRenderer/renderers/Avs3CcsaoRenderer";
import { Avs3EsaoRenderer } from "@/components/panels/OverlayRenderer/renderers/Avs3EsaoRenderer";
import { JpegXsDequantRenderer } from "@/components/panels/OverlayRenderer/renderers/JpegXsDequantRenderer";
import { JpegXsMctRenderer } from "@/components/panels/OverlayRenderer/renderers/JpegXsMctRenderer";
import { JpegXsNltRenderer } from "@/components/panels/OverlayRenderer/renderers/JpegXsNltRenderer";
import { JpegXsPrecinctRenderer } from "@/components/panels/OverlayRenderer/renderers/JpegXsPrecinctRenderer";
import { JpegXsTransformRenderer } from "@/components/panels/OverlayRenderer/renderers/JpegXsTransformRenderer";
import { Vc3SegmentRenderer } from "@/components/panels/OverlayRenderer/renderers/Vc3SegmentRenderer";
import { Av1CdefRenderer } from "@/components/panels/OverlayRenderer/renderers/Av1CdefRenderer";
import { Av1FilmGrainRenderer } from "@/components/panels/OverlayRenderer/renderers/Av1FilmGrainRenderer";
import { Av1LoopRestorationRenderer } from "@/components/panels/OverlayRenderer/renderers/Av1LoopRestorationRenderer";
import { Av1SuperResRenderer } from "@/components/panels/OverlayRenderer/renderers/Av1SuperResRenderer";

const W = 640;
const H = 360;

/** Records every property set and method call on a fake 2D context. */
function recordingCtx(): { ctx: CanvasRenderingContext2D; log: string[] } {
  const log: string[] = [];
  const norm = (v: unknown) =>
    typeof v === "string" ? v.replace(/\s+/g, "") : JSON.stringify(v);
  const state: Record<string, unknown> = {};
  const ctx = new Proxy(state, {
    get(target, prop: string) {
      if (prop === "measureText") {
        return (text: string) => {
          log.push(`measureText(${norm(text)}) font=${norm(target.font)}`);
          return { width: text.length * 7 };
        };
      }
      if (prop in target) return target[prop];
      return (...args: unknown[]) => {
        log.push(`${prop}(${args.map(norm).join(",")})`);
      };
    },
    set(target, prop: string, value) {
      target[prop] = value;
      log.push(`${prop}=${norm(value)}`);
      return true;
    },
  }) as unknown as CanvasRenderingContext2D;
  return { ctx, log };
}

function frameWith(extra: Record<string, unknown>): FrameInfo {
  return {
    frame_index: 0,
    frame_type: "I",
    size: 1000,
    ...extra,
  } as FrameInfo;
}

function record(run: (ctx: CanvasRenderingContext2D) => void): string[] {
  const { ctx, log } = recordingCtx();
  run(ctx);
  return log;
}

const bare = frameWith({});
const props = (ctx: CanvasRenderingContext2D, frame: FrameInfo) => ({
  ctx,
  width: W,
  height: H,
  frame,
});

describe("codec overlay renderers: no-data notice", () => {
  const cases: Array<[string, (ctx: CanvasRenderingContext2D) => void]> = [
    ["JpegXsDequant", (c) => JpegXsDequantRenderer(props(c, bare))],
    ["JpegXsTransform", (c) => JpegXsTransformRenderer(props(c, bare))],
    ["JpegXsMct", (c) => JpegXsMctRenderer(props(c, bare))],
    ["JpegXsNlt", (c) => JpegXsNltRenderer(props(c, bare))],
    ["JpegXsPrecinct", (c) => JpegXsPrecinctRenderer(props(c, bare))],
    ["Avs3Ccsao", (c) => Avs3CcsaoRenderer(props(c, bare))],
    ["Avs3Esao", (c) => Avs3EsaoRenderer(props(c, bare))],
    ["Vc3Segment", (c) => Vc3SegmentRenderer(props(c, bare))],
    ["Av1Cdef", (c) => Av1CdefRenderer({ ...props(c, bare), cdef: undefined })],
    [
      "Av1LoopRestoration",
      (c) =>
        Av1LoopRestorationRenderer({
          ...props(c, bare),
          loopRestoration: undefined,
        }),
    ],
    [
      "Av1FilmGrain",
      (c) => Av1FilmGrainRenderer({ ...props(c, bare), filmGrain: undefined }),
    ],
    [
      "Av1SuperRes",
      (c) =>
        Av1SuperResRenderer({ ...props(c, bare), superResolution: undefined }),
    ],
  ];

  it.each(cases)("%s", (_name, run) => {
    expect(record(run)).toMatchSnapshot();
  });

  it("disabled AV1 variants (enabled=false) draw the same notice as absent data", () => {
    const grain: Av1FilmGrainData = {
      enabled: false,
      seed: 1,
      scalingShift: 8,
      arCoeffLag: 0,
      chromaScalingFromLuma: false,
      overlap: false,
    };
    const sr: Av1SuperResData = {
      enabled: false,
      scaleDenominator: 8,
      upscaledWidth: 640,
      upscaledHeight: 360,
    };
    expect({
      grain: record((c) =>
        Av1FilmGrainRenderer({ ...props(c, bare), filmGrain: grain }),
      ),
      sr: record((c) =>
        Av1SuperResRenderer({ ...props(c, bare), superResolution: sr }),
      ),
    }).toMatchSnapshot();
  });

  it("empty-but-present data takes the notice branch too", () => {
    expect({
      ccsao: record((c) =>
        Avs3CcsaoRenderer(
          props(c, frameWith({ ccsao_map: { ccsao_applied: [] } })),
        ),
      ),
      esao: record((c) =>
        Avs3EsaoRenderer(props(c, frameWith({ esao_map: { esao_type: [] } }))),
      ),
      nlt: record((c) =>
        JpegXsNltRenderer(
          props(c, frameWith({ nlt_info: { present: false } })),
        ),
      ),
      vc3: record((c) =>
        Vc3SegmentRenderer(
          props(c, frameWith({ mb_grid: { cols: 0, rows: 2, bits: [1] } })),
        ),
      ),
      precinct: record((c) =>
        JpegXsPrecinctRenderer(
          props(
            c,
            frameWith({ precinct_map: { cols: 2, rows: 0, bits: [1, 2] } }),
          ),
        ),
      ),
    }).toMatchSnapshot();
  });
});

describe("codec overlay renderers: data present", () => {
  it("Avs3Ccsao draws cells, grid lines, legend", () => {
    const f = frameWith({
      ccsao_map: {
        grid_w: 3,
        grid_h: 2,
        ctu_size: 64,
        ccsao_applied: [true, false, true, false, true, false],
        luma_code: [0, 1, 2, 3, 4, 0],
      },
    });
    expect(record((c) => Avs3CcsaoRenderer(props(c, f)))).toMatchSnapshot();
  });

  it("Avs3Esao draws cells (skipping OFF), grid lines, legend", () => {
    const f = frameWith({
      esao_map: {
        grid_w: 3,
        grid_h: 2,
        ctu_size: 128,
        esao_type: [0, 1, 2, 3, 4, 5],
        esao_class: [0, 0, 0, 0, 0, 0],
      },
    });
    expect(record((c) => Avs3EsaoRenderer(props(c, f)))).toMatchSnapshot();
  });

  it("Vc3Segment draws cells, grid lines, info box", () => {
    const f = frameWith({
      mb_grid: { cols: 3, rows: 2, bits: [1, 2, 3, 4, 5, 6] },
      comp_id: "1237",
      chroma_sampling: 422,
      bits_per_component: 10,
      is_dnxhr: true,
    });
    expect(record((c) => Vc3SegmentRenderer(props(c, f)))).toMatchSnapshot();
  });

  it("JpegXsDequant", () => {
    const f = frameWith({
      dequant_map: {
        grid_w: 2,
        grid_h: 2,
        energy: [1, 2, 3, 4],
        labels: ["a", "b", "c", "d"],
      },
    });
    expect(record((c) => JpegXsDequantRenderer(props(c, f)))).toMatchSnapshot();
  });

  it("JpegXsTransform", () => {
    const f = frameWith({
      transform_map: {
        decomp_h: 2,
        decomp_v: 1,
        nodes: [
          { name: "LL", x: 0, y: 0, w: 0.5, h: 0.5, level: 1 },
          { name: "HL", x: 0.5, y: 0, w: 0.5, h: 0.5, level: 1 },
        ],
      },
    });
    expect(
      record((c) => JpegXsTransformRenderer(props(c, f))),
    ).toMatchSnapshot();
  });

  it("JpegXsMct", () => {
    const f = frameWith({
      mct_info: {
        mct_type: "RCT",
        num_comps: 3,
        comp_labels: ["Y", "Cb"],
        reversible: true,
      },
    });
    expect(record((c) => JpegXsMctRenderer(props(c, f)))).toMatchSnapshot();
  });

  it("JpegXsNlt", () => {
    const f = frameWith({
      nlt_info: { present: true, num_comps: 3, comp_types: ["QUA", "EXT"] },
    });
    expect(record((c) => JpegXsNltRenderer(props(c, f)))).toMatchSnapshot();
  });

  it("JpegXsPrecinct", () => {
    const f = frameWith({
      precinct_map: {
        cols: 2,
        rows: 2,
        precinct_w: 64,
        precinct_h: 64,
        bits: [10, 20, 30, 40],
      },
    });
    expect(
      record((c) => JpegXsPrecinctRenderer(props(c, f))),
    ).toMatchSnapshot();
  });

  it("AV1 renderers with data", () => {
    const cdef: Av1CdefData = {
      width: W,
      height: H,
      blockSize: 64,
      blocks: [
        { x: 0, y: 0, size: 64, direction: 0, strength: 0 },
        { x: 64, y: 0, size: 64, direction: 3, strength: 100 },
      ],
      damping: 3,
      yPrimaryStrength: 4,
      ySecondaryStrength: 1,
    };
    const lr: Av1LoopRestorationData = {
      width: W,
      height: H,
      unitSize: 64,
      yType: 1,
      units: [
        { x: 0, y: 0, size: 64, restorationType: 0 },
        { x: 64, y: 0, size: 64, restorationType: 2 },
      ],
    };
    const grain: Av1FilmGrainData = {
      enabled: true,
      seed: 7,
      scalingShift: 9,
      arCoeffLag: 2,
      chromaScalingFromLuma: true,
      overlap: false,
    };
    const sr: Av1SuperResData = {
      enabled: true,
      scaleDenominator: 12,
      upscaledWidth: 640,
      upscaledHeight: 360,
    };
    expect({
      cdef: record((c) => Av1CdefRenderer({ ...props(c, bare), cdef })),
      lr: record((c) =>
        Av1LoopRestorationRenderer({ ...props(c, bare), loopRestoration: lr }),
      ),
      // Small canvas: the grain texture draws one dot per ~25 px.
      grain: record((c) =>
        Av1FilmGrainRenderer({
          ...props(c, bare),
          width: 64,
          height: 36,
          filmGrain: grain,
        }),
      ),
      sr: record((c) =>
        Av1SuperResRenderer({ ...props(c, bare), superResolution: sr }),
      ),
    }).toMatchSnapshot();
  });
});
