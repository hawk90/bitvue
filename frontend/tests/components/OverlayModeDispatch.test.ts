/**
 * Characterization tests: overlay mode dispatch (`renderModeOverlay`).
 *
 * For EVERY `VisualizationMode` (plus an unknown string) and BOTH dispatch paths -- the main
 * mode (pass 1) and the info overlay (pass 2) -- this records, on a fake 2D context with a real
 * save/restore stack:
 *   - the ordered sequence of property sets and method calls,
 *   - save/restore counts and underflow,
 *   - which tracked context properties (globalAlpha, font, fillStyle, ...) differ from their
 *     initial value after the render (= state leaked out of the call).
 * It pins what each mode does for several frame shapes (no data, rich data, JPEG XS transform
 * map, no bit grid, intra frame) and AV1 option injection (absent / full / cdef only), plus the
 * set of modes each path handles. A refactor of the dispatch (switch -> registry) must keep
 * every snapshot and every explicit expectation here passing unchanged.
 *
 * Colour strings are compared with spaces removed (spelling, not a visible difference).
 */

import { describe, it, expect, vi } from "vitest";
import type { FrameInfo } from "@/types/video";
import type {
  Av1FeaturesData,
  OverlayRenderOptionsExtended,
} from "@/components/panels/OverlayRenderer";
import { renderModeOverlay } from "@/components/panels/OverlayRenderer";
import { CODEC_MODE_REGISTRY } from "@/utils/codecModeRegistry";

const webglCalls = vi.hoisted(() => [] as string[]);

vi.mock("@/components/panels/OverlayRenderer/webgl/mv-webgl", () => ({
  WEBGL_MV_THRESHOLD: 4,
  renderMVWebGL: () => {
    webglCalls.push("renderMVWebGL");
  },
  clearMVWebGL: () => {
    webglCalls.push("clearMVWebGL");
  },
}));

const W = 128;
const H = 72;

/** Every member of the `VisualizationMode` union (utils/codecModeRegistry.ts). */
const ALL_MODES = [
  "yuv",
  "coding-flow",
  "prediction",
  "transform",
  "reconstruction",
  "loop-filter",
  "qp-map",
  "heat-map",
  "mv-field",
  "reference",
  "residuals",
  "sao",
  "dual-tree",
  "inverse-map",
  "adaptive-filter",
  "cdef-filter",
  "super-res",
  "loop-restoration",
  "film-grain",
  "esao",
  "ccsao",
  "precinct",
  "dequant",
  "mct",
  "nlt",
  "segment",
  "psnr-overlay",
  "ssim-overlay",
  "block-type",
  "pu-type",
  "mb-type",
  "reference-indices",
  "efficiency-map",
  "inter-memory",
  "simple-motion",
  "deblocking",
  "av1-features",
  "overview",
  "not-a-real-mode",
] as const;

/** Context properties whose post-render value is compared against the initial value. */
const INITIAL_STATE: Record<string, unknown> = {
  globalAlpha: 1,
  font: "10px sans-serif",
  fillStyle: "#000000",
  strokeStyle: "#000000",
  lineWidth: 1,
  lineCap: "butt",
  lineJoin: "miter",
  textAlign: "start",
  textBaseline: "alphabetic",
  shadowBlur: 0,
  shadowColor: "rgba(0,0,0,0)",
  globalCompositeOperation: "source-over",
  imageSmoothingEnabled: true,
  lineDash: "",
};

interface Recording {
  log: string[];
  saves: number;
  restores: number;
  underflow: number;
  leaked: Record<string, unknown>;
}

const norm = (v: unknown) =>
  typeof v === "string" ? v.replace(/\s+/g, "") : JSON.stringify(v);

/** Fake 2D context: logs everything, keeps a real property save/restore stack. */
function recordingCtx() {
  const log: string[] = [];
  const state: Record<string, unknown> = { ...INITIAL_STATE };
  const stack: Array<Record<string, unknown>> = [];
  const counters = { saves: 0, restores: 0, underflow: 0 };
  const ctx = new Proxy(state, {
    get(target, prop: string) {
      if (prop === "measureText") {
        return (text: string) => {
          log.push(`measureText(${norm(text)}) font=${norm(target.font)}`);
          return { width: text.length * 7 };
        };
      }
      if (prop === "save") {
        return () => {
          log.push("save()");
          counters.saves++;
          stack.push({ ...target });
        };
      }
      if (prop === "restore") {
        return () => {
          log.push("restore()");
          counters.restores++;
          const top = stack.pop();
          if (!top) counters.underflow++;
          else Object.assign(target, top);
        };
      }
      if (prop === "createLinearGradient" || prop === "createRadialGradient") {
        return (...args: unknown[]) => {
          log.push(`${prop}(${args.map(norm).join(",")})`);
          return {
            addColorStop: (o: number, c: string) =>
              log.push(`addColorStop(${o},${norm(c)})`),
          };
        };
      }
      if (prop === "setLineDash") {
        return (segments: number[]) => {
          log.push(`setLineDash(${norm(segments)})`);
          target.lineDash = segments.join(",");
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
  return { ctx, log, state, counters };
}

function run(
  mode: string,
  frame: FrameInfo | null,
  opts: {
    overlays?: ReadonlySet<string>;
    av1Features?: Av1FeaturesData;
    webgl?: boolean;
    dpr?: number;
  } = {},
): Recording {
  webglCalls.length = 0;
  const { ctx, log, state, counters } = recordingCtx();
  const dpr = opts.dpr ?? 1;
  const canvas = {
    width: W * dpr,
    height: H * dpr,
  } as unknown as HTMLCanvasElement;
  const options = {
    mode,
    frame,
    canvas,
    ctx,
    av1Features: opts.av1Features,
    activeOverlays: opts.overlays,
    webglCanvas: opts.webgl ? ({} as HTMLCanvasElement) : undefined,
    dpr,
  } as unknown as OverlayRenderOptionsExtended;
  renderModeOverlay(options);
  const leaked: Record<string, unknown> = {};
  for (const k of Object.keys(INITIAL_STATE)) {
    if (state[k] !== INITIAL_STATE[k]) leaked[k] = norm(state[k]);
  }
  return {
    log: [...log, ...webglCalls.map((c) => `webgl:${c}`)],
    saves: counters.saves,
    restores: counters.restores,
    underflow: counters.underflow,
    leaked,
  };
}

// ─── Frame fixtures ──────────────────────────────────────────────────────────

const bare = { frame_index: 0, frame_type: "I", size: 1000 } as FrameInfo;

function grid(n: number, f: (i: number) => unknown) {
  return Array.from({ length: n }, (_, i) => f(i));
}

const richBase = {
  frame_index: 3,
  frame_type: "P",
  size: 5000,
  ref_frames: [1, 2],
  qp_grid: {
    grid_w: 4,
    grid_h: 3,
    block_w: 32,
    block_h: 24,
    qp: grid(12, (i) => 20 + i * 3),
    qp_min: 20,
    qp_max: 53,
  },
  energy_grid: {
    grid_w: 4,
    grid_h: 3,
    block_w: 32,
    block_h: 24,
    energy_bpp: grid(12, (i) => i * 0.1),
  },
  bit_grid: {
    grid_w: 4,
    grid_h: 3,
    block_w: 32,
    block_h: 24,
    bits_per_pixel: grid(12, (i) => i * 0.05),
    symbols_per_bit: grid(12, (i) => 0.5 + i * 0.1),
  },
  mv_grid: {
    coded_width: W,
    coded_height: H,
    block_w: 32,
    block_h: 24,
    grid_w: 4,
    grid_h: 3,
    mv_l0: grid(12, (i) => ({ dx_qpel: i * 4 - 20, dy_qpel: 8 - i * 2 })),
    mv_l1: grid(12, () => ({ dx_qpel: 0, dy_qpel: 0 })),
    mode: grid(12, (i) => i % 6),
  },
  partition_grid: {
    coded_width: W,
    coded_height: H,
    sb_size: 64,
    blocks: [
      { x: 0, y: 0, width: 64, height: 64, partition: 3, depth: 0 },
      {
        x: 64,
        y: 0,
        width: 32,
        height: 32,
        partition: 1,
        depth: 1,
        tree_type: 1,
      },
      {
        x: 96,
        y: 0,
        width: 32,
        height: 32,
        partition: 2,
        depth: 1,
        tree_type: 2,
      },
      {
        x: 64,
        y: 32,
        width: 64,
        height: 32,
        partition: 0,
        depth: 1,
        tree_type: 0,
      },
    ],
  },
  prediction_mode_grid: {
    coded_width: W,
    coded_height: H,
    block_w: 32,
    block_h: 24,
    grid_w: 4,
    grid_h: 3,
    modes: grid(12, (i) => (i % 5 === 4 ? null : i * 3)),
  },
  transform_grid: {
    coded_width: W,
    coded_height: H,
    block_w: 32,
    block_h: 24,
    grid_w: 4,
    grid_h: 3,
    tx_sizes: grid(12, (i) => (i % 6 === 5 ? null : i % 5)),
  },
  mb_type_grid: {
    coded_width: W,
    coded_height: H,
    block_w: 32,
    block_h: 24,
    grid_w: 4,
    grid_h: 3,
    mb_types: grid(12, (i) => (i % 7 === 6 ? null : i)),
  },
  ref_idx_grid: {
    coded_width: W,
    coded_height: H,
    block_w: 32,
    block_h: 24,
    grid_w: 4,
    grid_h: 3,
    ref_idx_l0: grid(12, (i) => (i % 4 === 3 ? null : i % 3)),
    ref_idx_l1: grid(12, (i) => (i % 2 === 0 ? null : i % 2)),
  },
  // codec-exclusive extras read off the frame object
  ccsao_map: {
    grid_w: 3,
    grid_h: 2,
    ctu_size: 64,
    ccsao_applied: [true, false, true, false, true, false],
    luma_code: [0, 1, 2, 3, 4, 0],
  },
  esao_map: {
    grid_w: 3,
    grid_h: 2,
    ctu_size: 64,
    esao_type: [0, 1, 2, 3, 4, 5],
    esao_class: [0, 0, 0, 0, 0, 0],
  },
  dequant_map: {
    grid_w: 2,
    grid_h: 2,
    energy: [1, 2, 3, 4],
    labels: ["a", "b", "c", "d"],
  },
  mct_info: {
    mct_type: "Rct",
    num_comps: 3,
    comp_labels: ["Y", "Cb"],
    reversible: true,
  },
  nlt_info: { present: true, num_comps: 3, comp_types: ["QUA", "XEXP"] },
  precinct_map: {
    cols: 2,
    rows: 2,
    precinct_w: 64,
    precinct_h: 36,
    bits: [10, 20, 30, 40],
  },
  mb_grid: { cols: 3, rows: 2, bits: [1, 2, 3, 4, 5, 6] },
  comp_id: "1237",
  chroma_sampling: 422,
  bits_per_component: 10,
  is_dnxhr: true,
};

const rich = richBase as unknown as FrameInfo;
const richNoBitGrid = {
  ...richBase,
  bit_grid: undefined,
} as unknown as FrameInfo;
const richIntra = { ...richBase, frame_type: "I" } as unknown as FrameInfo;
const richJpegXs = {
  ...richBase,
  transform_map: {
    decomp_h: 2,
    decomp_v: 1,
    nodes: [
      { name: "LL", x: 0, y: 0, w: 0.5, h: 0.5, level: 1 },
      { name: "HL", x: 0.5, y: 0, w: 0.5, h: 0.5, level: 1 },
    ],
  },
} as unknown as FrameInfo;

const av1Full: Av1FeaturesData = {
  frameIndex: 3,
  cdef: {
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
  },
  loopRestoration: {
    width: W,
    height: H,
    unitSize: 64,
    yType: 1,
    units: [
      { x: 0, y: 0, size: 64, restorationType: 0 },
      { x: 64, y: 0, size: 64, restorationType: 2 },
    ],
  },
  filmGrain: {
    enabled: true,
    seed: 7,
    scalingShift: 9,
    arCoeffLag: 2,
    chromaScalingFromLuma: true,
    overlap: false,
  },
  superResolution: {
    enabled: true,
    scaleDenominator: 12,
    upscaledWidth: W,
    upscaledHeight: H,
  },
};
const av1CdefOnly: Av1FeaturesData = {
  frameIndex: 3,
  cdef: av1Full.cdef,
};

type Variant = [string, FrameInfo, { av1Features?: Av1FeaturesData }];
const VARIANTS: Variant[] = [
  ["bare", bare, {}],
  ["rich", rich, {}],
  ["rich-no-bit-grid", richNoBitGrid, {}],
  ["rich-intra", richIntra, {}],
  ["rich-jpegxs-transform-map", richJpegXs, {}],
  ["rich+av1-full", rich, { av1Features: av1Full }],
  ["rich+av1-cdef-only", rich, { av1Features: av1CdefOnly }],
];

function mainRun(mode: string, frame: FrameInfo, av1?: Av1FeaturesData) {
  return run(mode, frame, { av1Features: av1 });
}
function infoRun(mode: string, frame: FrameInfo, av1?: Av1FeaturesData) {
  // "overview" is a no-op main mode; use "yuv" when the overlay under test is "overview" itself
  // (an overlay equal to the main mode is skipped).
  const main = mode === "overview" ? "yuv" : "overview";
  return run(main, frame, { overlays: new Set([mode]), av1Features: av1 });
}

/** The 3 calls an info overlay path emits for a mode with no renderer. */
const INFO_WRAPPER_ONLY = ["save()", "globalAlpha=0.72", "restore()"];

// ─── Per-mode snapshots ──────────────────────────────────────────────────────

describe("main-mode dispatch: every mode x frame shape x AV1 options", () => {
  it.each(ALL_MODES)("%s", (mode) => {
    const out: Record<string, Recording> = {};
    for (const [name, frame, o] of VARIANTS) {
      out[name] = mainRun(mode, frame, o.av1Features);
    }
    expect(out).toMatchSnapshot();
  });
});

describe("info-overlay dispatch: every mode x frame shape x AV1 options", () => {
  it.each(ALL_MODES)("%s", (mode) => {
    const out: Record<string, Recording> = {};
    for (const [name, frame, o] of VARIANTS) {
      out[name] = infoRun(mode, frame, o.av1Features);
    }
    expect(out).toMatchSnapshot();
  });
});

// ─── Explicit expectations (readable, independent of the snapshots) ──────────

describe("which modes each dispatch path handles", () => {
  const handled = (path: "main" | "info") =>
    ALL_MODES.filter((mode) =>
      VARIANTS.some(([, frame, o]) => {
        const r =
          path === "main"
            ? mainRun(mode, frame, o.av1Features)
            : infoRun(mode, frame, o.av1Features);
        return path === "main"
          ? r.log.length > 0
          : JSON.stringify(r.log) !== JSON.stringify(INFO_WRAPPER_ONLY);
      }),
    );

  it("main switch handles exactly these modes (everything else is a no-op)", () => {
    expect(handled("main")).toEqual([
      "coding-flow",
      "prediction",
      "transform",
      "qp-map",
      "mv-field",
      "reference",
      "dual-tree",
      "inverse-map",
      "adaptive-filter",
      "cdef-filter",
      "super-res",
      "loop-restoration",
      "film-grain",
      "esao",
      "ccsao",
      "precinct",
      "dequant",
      "mct",
      "nlt",
      "segment",
    ]);
  });

  it("info switch handles exactly these modes", () => {
    expect(handled("info")).toEqual([
      "qp-map",
      "heat-map",
      "mv-field",
      "block-type",
      "mb-type",
      "reference-indices",
      "efficiency-map",
    ]);
  });

  it("unhandled main modes record nothing at all", () => {
    for (const mode of [
      "yuv",
      "overview",
      "reconstruction",
      "loop-filter",
      "residuals",
      "sao",
      "psnr-overlay",
      "ssim-overlay",
      "pu-type",
      "inter-memory",
      "simple-motion",
      "deblocking",
      "av1-features",
      "not-a-real-mode",
    ]) {
      for (const [, frame, o] of VARIANTS) {
        expect(mainRun(mode, frame, o.av1Features).log).toEqual([]);
      }
    }
  });

  it("unhandled info modes still emit exactly the save/alpha/restore wrapper", () => {
    for (const mode of [
      "yuv",
      "coding-flow",
      "transform",
      "psnr-overlay",
      "pu-type",
      "av1-features",
      "not-a-real-mode",
    ]) {
      expect(infoRun(mode, rich).log).toEqual(INFO_WRAPPER_ONLY);
    }
  });

  it("every mode in the codec registry is covered by ALL_MODES", () => {
    const registryModes = new Set(
      Object.values(CODEC_MODE_REGISTRY).flatMap((es) => es.map((e) => e.mode)),
    );
    for (const m of registryModes) expect(ALL_MODES).toContain(m);
  });
});

describe("save/restore discipline of the main path (rich frame)", () => {
  // Modes whose dispatch wraps the renderer in its own save()/restore().
  const WRAPPED = [
    "dual-tree",
    "adaptive-filter",
    "inverse-map",
    "esao",
    "ccsao",
    "precinct",
    "dequant",
    "mct",
    "nlt",
    "segment",
    "cdef-filter",
    "loop-restoration",
    "film-grain",
    "super-res",
  ];
  // Handled modes dispatched bare (no wrapper), so any state they set is NOT undone here.
  const BARE = [
    "coding-flow",
    "prediction",
    "qp-map",
    "mv-field",
    "reference",
    "heat-map",
    "transform",
  ];

  it("wrapped modes: log starts with save(), ends with restore(), and leak nothing", () => {
    for (const mode of WRAPPED) {
      const r = mainRun(mode, rich, av1Full);
      expect(r.log[0], mode).toBe("save()");
      expect(r.log[r.log.length - 1], mode).toBe("restore()");
      expect(r.saves, mode).toBe(r.restores);
      expect(r.underflow, mode).toBe(0);
      expect(r.leaked, mode).toEqual({});
    }
  });

  it("bare modes: do not start with a wrapper save()", () => {
    for (const mode of BARE.filter((m) => m !== "heat-map")) {
      const r = mainRun(mode, rich);
      expect(r.log[0], mode).not.toBe("save()");
    }
  });

  it("heat-map and transform (non-JPEG-XS) as main modes: no wrapper", () => {
    // heat-map is not handled by the main switch at all.
    expect(mainRun("heat-map", rich).log).toEqual([]);
    expect(mainRun("transform", rich).log[0]).not.toBe("save()");
  });

  it("transform: JPEG XS (transform_map) is wrapped, others are not", () => {
    const xs = mainRun("transform", richJpegXs);
    expect(xs.log[0]).toBe("save()");
    expect(xs.log[xs.log.length - 1]).toBe("restore()");
    expect(xs.leaked).toEqual({});
    const other = mainRun("transform", rich);
    expect(other.log).not.toEqual(xs.log);
    // transform_map present but falsy/absent -> generic path
    expect(
      mainRun("transform", {
        ...richBase,
        transform_map: null,
      } as unknown as FrameInfo).log,
    ).toEqual(other.log);
  });

  it("record the actual balance and leaks of every mode (documents the bare modes' leaks)", () => {
    const table: Record<string, unknown> = {};
    for (const mode of ALL_MODES) {
      const r = mainRun(mode, rich, av1Full);
      table[mode] = {
        balanced: r.saves === r.restores,
        underflow: r.underflow,
        leaked: Object.keys(r.leaked),
      };
    }
    expect(table).toMatchSnapshot();
  });
});

describe("info path wraps every renderer in save + globalAlpha 0.72 + restore", () => {
  it("each handled info mode: first calls save, globalAlpha=0.72; last call restore; no leak", () => {
    for (const mode of [
      "qp-map",
      "heat-map",
      "mv-field",
      "block-type",
      "mb-type",
      "reference-indices",
      "efficiency-map",
    ]) {
      const r = infoRun(mode, rich);
      expect(r.log.slice(0, 2), mode).toEqual(["save()", "globalAlpha=0.72"]);
      expect(r.log[r.log.length - 1], mode).toBe("restore()");
      expect(r.saves, mode).toBe(r.restores);
      expect(r.underflow, mode).toBe(0);
      expect(r.leaked, mode).toEqual({});
    }
  });
});

describe("codec branches", () => {
  it("heat-map as info overlay: bit_grid -> bit-cost heat map, no bit_grid -> MV field fallback", () => {
    const withGrid = infoRun("heat-map", rich).log;
    const without = infoRun("heat-map", richNoBitGrid).log;
    expect(withGrid).not.toEqual(without);
    // The fallback draws exactly what mv-field draws.
    expect(without).toEqual(infoRun("mv-field", richNoBitGrid).log);
    // And the bit-grid branch is not the MV field.
    expect(withGrid).not.toEqual(infoRun("mv-field", rich).log);
  });

  it("heat-map without bit_grid forwards the webgl canvas to the MV field renderer", () => {
    const r = run("overview", richNoBitGrid, {
      overlays: new Set(["heat-map"]),
      webgl: true,
    });
    expect(r.log).toContain("webgl:renderMVWebGL");
    const withGrid = run("overview", rich, {
      overlays: new Set(["heat-map"]),
      webgl: true,
    });
    expect(withGrid.log.some((l) => l.startsWith("webgl:"))).toBe(false);
  });

  it("mv-field webgl: sparse grid clears the webgl canvas, dense grid renders via webgl", () => {
    // 12 blocks > mocked threshold of 4 -> dense
    const dense = run("mv-field", rich, { webgl: true });
    expect(dense.log).toContain("webgl:renderMVWebGL");
    const sparseFrame = {
      ...richBase,
      mv_grid: { ...richBase.mv_grid, grid_w: 2, grid_h: 1 },
    } as unknown as FrameInfo;
    const sparse = run("mv-field", sparseFrame, { webgl: true });
    expect(sparse.log).toContain("webgl:clearMVWebGL");
    expect(sparse.log).not.toContain("webgl:renderMVWebGL");
    // No webgl canvas -> never touches webgl
    expect(run("mv-field", rich).log.some((l) => l.startsWith("webgl:"))).toBe(
      false,
    );
  });

  it("mv-field main path vs info path: same renderer calls, info adds the wrapper", () => {
    const main = mainRun("mv-field", rich).log;
    const info = infoRun("mv-field", rich).log;
    expect(info).toEqual(["save()", "globalAlpha=0.72", ...main, "restore()"]);
    const qpMain = mainRun("qp-map", rich).log;
    const qpInfo = infoRun("qp-map", rich).log;
    expect(qpInfo).toEqual([
      "save()",
      "globalAlpha=0.72",
      ...qpMain,
      "restore()",
    ]);
  });

  it("AV1 options: absent options draw the renderers' no-data notice, present ones draw data", () => {
    for (const mode of [
      "cdef-filter",
      "loop-restoration",
      "film-grain",
      "super-res",
    ]) {
      const none = mainRun(mode, rich).log;
      const full = mainRun(mode, rich, av1Full).log;
      expect(none, mode).not.toEqual(full);
      expect(
        none.some((l) => l.startsWith("measureText(")),
        mode,
      ).toBe(true);
    }
    // cdef-only injects cdef data but leaves the other three features absent.
    expect(mainRun("cdef-filter", rich, av1CdefOnly).log).toEqual(
      mainRun("cdef-filter", rich, av1Full).log,
    );
    expect(mainRun("film-grain", rich, av1CdefOnly).log).toEqual(
      mainRun("film-grain", rich).log,
    );
  });

  it("AV1 options have no effect on non-AV1 modes", () => {
    for (const mode of [
      "coding-flow",
      "prediction",
      "qp-map",
      "dual-tree",
      "segment",
    ]) {
      expect(mainRun(mode, rich, av1Full).log, mode).toEqual(
        mainRun(mode, rich).log,
      );
    }
  });
});

describe("renderModeOverlay composition", () => {
  it("null frame is a complete no-op, even with overlays", () => {
    const r = run("coding-flow", null, { overlays: new Set(["qp-map"]) });
    expect(r.log).toEqual([]);
  });

  it("main pass runs first, then info overlays in set order; an overlay equal to the main mode is skipped", () => {
    const both = run("prediction", rich, {
      overlays: new Set(["qp-map", "prediction", "block-type", "mv-field"]),
    });
    const expected = [
      ...mainRun("prediction", rich).log,
      ...infoRun("qp-map", rich).log,
      ...infoRun("block-type", rich).log,
      ...infoRun("mv-field", rich).log,
    ];
    expect(both.log).toEqual(expected);
    // Reordering the set reorders the info passes.
    const reordered = run("prediction", rich, {
      overlays: new Set(["mv-field", "qp-map"]),
    });
    expect(reordered.log).toEqual([
      ...mainRun("prediction", rich).log,
      ...infoRun("mv-field", rich).log,
      ...infoRun("qp-map", rich).log,
    ]);
  });

  it("empty overlay set draws only the main mode", () => {
    expect(run("qp-map", rich, { overlays: new Set() }).log).toEqual(
      mainRun("qp-map", rich).log,
    );
  });

  it("dpr divides the physical canvas size into the logical size every renderer sees", () => {
    for (const mode of ["coding-flow", "segment", "qp-map"]) {
      const a = run(mode, rich, { dpr: 1 });
      const b = run(mode, rich, { dpr: 2 });
      expect(b.log, mode).toEqual(a.log);
    }
    expect(
      run("segment", rich, { dpr: 2, overlays: new Set(["qp-map"]) }).log,
    ).toEqual(run("segment", rich, { overlays: new Set(["qp-map"]) }).log);
  });
});
