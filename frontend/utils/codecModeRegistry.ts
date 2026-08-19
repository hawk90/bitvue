/**
 * Codec Mode Registry
 *
 * Maps each supported codec to its available analysis modes and F-key assignments.
 * This is the single source of truth for VQ Analyzer parity:
 *   - Which modes exist per codec
 *   - Which F-key triggers each mode
 *   - Labels and descriptions shown in UI
 *
 * Adding a new codec: append an entry to CODEC_MODE_REGISTRY.
 */

// ─── All possible visualization modes across all codecs ──────────────────────

export type VisualizationMode =
  // ── Common ────────────────────────────────────────────────────────────────
  | "yuv" // Pure decoded frame, no overlay (replaces "overview")
  | "coding-flow" // Block partition tree (CTU/CU/SB structure)
  | "prediction" // Intra/inter prediction mode visualization
  | "transform" // Transform block sizes and types
  | "reconstruction" // Prediction + residual reconstruction stages
  | "loop-filter" // Deblocking filter boundary strength
  | "qp-map" // QP heatmap (info overlay)
  | "heat-map" // Bit-cost heatmap (info overlay)
  | "mv-field" // Motion vector field
  | "reference" // Reference frame dependency graph
  | "residuals" // Residual energy heatmap
  // ── HEVC / VVC shared ─────────────────────────────────────────────────────
  | "sao" // Sample Adaptive Offset filter
  // ── VVC exclusive ─────────────────────────────────────────────────────────
  | "dual-tree" // VVC dual-tree partitioning (luma / chroma separate)
  | "inverse-map" // LMCS inverse mapping visualization
  | "adaptive-filter" // ALF (Adaptive Loop Filter) parameters
  // ── AV1 exclusive ─────────────────────────────────────────────────────────
  | "cdef-filter" // CDEF direction / strength per superblock
  | "super-res" // Downscale + upscale regions
  | "loop-restoration" // Wiener / self-guided restoration units
  | "film-grain" // Film grain synthesis — before vs after
  // ── AVS3 exclusive ────────────────────────────────────────────────────────
  | "esao" // Enhanced SAO (AVS3)
  | "ccsao" // Cross-Component SAO (AVS3)
  // ── JPEG XS exclusive ─────────────────────────────────────────────────────
  | "precinct" // Precinct boundary visualization
  | "dequant" // De-quantized coefficients
  | "mct" // Multiple Component Transform
  | "nlt" // Non-Linear Transform
  // ── VC-3 / DNxHD exclusive ────────────────────────────────────────────────
  | "segment" // DNxHD segment / macroblock grid
  // ── Dedicated info-overlay modes ─────────────────────────────────────────
  | "psnr-overlay" // Per-block PSNR heatmap (requires debug YUV reference)
  | "ssim-overlay" // Per-block SSIM map (HEVC only)
  | "block-type" // Block type color map (AV1, VP9)
  | "pu-type" // PU prediction type color map (HEVC)
  | "mb-type" // MB prediction type color map (AVC)
  | "reference-indices" // Reference list index color map (HEVC, AVC)
  | "efficiency-map" // Bits-per-pixel efficiency heatmap (AV1, VP9)
  | "inter-memory" // Inter prediction memory access heatmap (VVC)
  | "simple-motion" // Simplified MV arrows (all codecs)
  // ── Legacy / fallback ─────────────────────────────────────────────────────
  | "deblocking" // Alias kept for backward compat (= loop-filter)
  | "av1-features" // Legacy AV1 catch-all (superseded by individual modes)
  | "overview"; // No-codec default (welcome / no file loaded)

// ─── Registry entry ──────────────────────────────────────────────────────────

export interface CodecModeEntry {
  /** F-key number (1–12). null = info overlay (toggle, no F-key). */
  fKey: number | null;
  mode: VisualizationMode;
  label: string;
  description: string;
  /** If true, rendered as an overlay ON TOP of the current main mode. */
  isInfoOverlay?: boolean;
}

// ─── Per-codec registry ───────────────────────────────────────────────────────

/**
 * HEVC / H.265
 *
 * F1 Coding Flow | F2 Predictions | F3 Transform | F4 Reconstruction
 * F5 Loop Filter | F6 SAO         | F7 YUV
 * Info overlays: QP Map, Heat Map, MV Heat, PU Type, PU Ref Indices, PSNR
 */
const HEVC_MODES: CodecModeEntry[] = [
  {
    fKey: 1,
    mode: "coding-flow",
    label: "Coding Flow",
    description: "CTU / CU partition tree with TU overlay",
  },
  {
    fKey: 2,
    mode: "prediction",
    label: "Predictions",
    description: "Intra direction arrows and inter MV arrows with ref indices",
  },
  {
    fKey: 3,
    mode: "transform",
    label: "Transform",
    description: "TU boundaries and non-zero coefficient indicator",
  },
  {
    fKey: 4,
    mode: "reconstruction",
    label: "Reconstruction",
    description: "Prediction + residual reconstruction stages",
  },
  {
    fKey: 5,
    mode: "loop-filter",
    label: "Loop Filter",
    description: "Deblocking boundary strength (BS 0/1/2 colour-coded)",
  },
  {
    fKey: 6,
    mode: "sao",
    label: "SAO",
    description: "SAO type per CTU: Edge (blue), Band (red), None (grey)",
  },
  {
    fKey: 7,
    mode: "yuv",
    label: "YUV",
    description: "Decoded picture without any overlay",
  },
  // ── Info overlays ──────────────────────────────────────────────────────────
  {
    fKey: null,
    mode: "qp-map",
    label: "QP Map",
    description: "Per-block QP heatmap (jet colormap)",
    isInfoOverlay: true,
  },
  {
    fKey: null,
    mode: "heat-map",
    label: "Heat Map",
    description: "Per-block bit-cost heatmap",
    isInfoOverlay: true,
  },
  {
    fKey: null,
    mode: "mv-field",
    label: "MV Heat",
    description: "Motion-vector magnitude heatmap",
    isInfoOverlay: true,
  },
  {
    fKey: null,
    mode: "pu-type",
    label: "PU Type",
    description: "PU prediction type color map (INTRA/INTER/SKIP/MERGE)",
    isInfoOverlay: true,
  },
  {
    fKey: null,
    mode: "reference-indices",
    label: "Ref Indices",
    description: "Reference list index color map (L0/L1/BI per partition)",
    isInfoOverlay: true,
  },
  {
    fKey: null,
    mode: "psnr-overlay",
    label: "PSNR",
    description: "Per-block PSNR heatmap (requires debug YUV reference)",
    isInfoOverlay: true,
  },
  {
    fKey: null,
    mode: "ssim-overlay",
    label: "SSIM",
    description: "Per-block SSIM map",
    isInfoOverlay: true,
  },
];

/**
 * VVC / H.266
 *
 * F1 Dual Tree | F2 Coding Flow | F3 Predictions | F4 Transform
 * F5 Reconstruction | F6 Inverse Map | F7 Loop Filter | F8 SAO
 * F9 Adaptive Filter (ALF) | F10 YUV
 * Info overlays: QP Map, Heat Map, Inter Memory Reads
 */
const VVC_MODES: CodecModeEntry[] = [
  {
    fKey: 1,
    mode: "dual-tree",
    label: "Dual Tree",
    description: "Luma (blue) and chroma (red) partition trees side-by-side",
  },
  {
    fKey: 2,
    mode: "coding-flow",
    label: "Coding Flow",
    description: "CTU / CU partition tree",
  },
  {
    fKey: 3,
    mode: "prediction",
    label: "Predictions",
    description:
      "Includes CCLM, MRLP, IBC, geometric partition mode (GPM) arrows",
  },
  {
    fKey: 4,
    mode: "transform",
    label: "Transform",
    description: "MTS / LFNST transform type per TU",
  },
  {
    fKey: 5,
    mode: "reconstruction",
    label: "Reconstruction",
    description: "Reconstruction stages including LMCS remapping",
  },
  {
    fKey: 6,
    mode: "inverse-map",
    label: "Inverse Map",
    description: "LMCS inverse-mapping function visualized per luma CTU",
  },
  {
    fKey: 7,
    mode: "loop-filter",
    label: "Loop Filter",
    description: "Deblocking boundary strength",
  },
  {
    fKey: 8,
    mode: "sao",
    label: "SAO",
    description: "SAO type per CTU",
  },
  {
    fKey: 9,
    mode: "adaptive-filter",
    label: "Adaptive Filter",
    description: "ALF — filtered blocks (blue) vs unfiltered (grey)",
  },
  {
    fKey: 10,
    mode: "yuv",
    label: "YUV",
    description: "Decoded picture without any overlay",
  },
  // ── Info overlays ──────────────────────────────────────────────────────────
  {
    fKey: null,
    mode: "qp-map",
    label: "QP Map",
    description: "Per-block QP heatmap",
    isInfoOverlay: true,
  },
  {
    fKey: null,
    mode: "heat-map",
    label: "Heat Map",
    description: "Per-block bit-cost heatmap",
    isInfoOverlay: true,
  },
  {
    fKey: null,
    mode: "inter-memory",
    label: "Inter Mem",
    description: "Inter prediction memory access frequency heatmap",
    isInfoOverlay: true,
  },
];

/**
 * AV1
 *
 * F1 Coding Flow | F2 Predictions | F3 Transform | F4 Reconstruction
 * F5 Loop Filter | F6 CDEF Filter | F7 SuperRes Filter
 * F8 Loop Restoration | F9 Film Grain Pixels | F10 YUV
 * Info overlays: Heat Map, Efficiency Map, Block Type, PSNR
 */
const AV1_MODES: CodecModeEntry[] = [
  {
    fKey: 1,
    mode: "coding-flow",
    label: "Coding Flow",
    description: "Superblock → block partition (quad / rectangular splits)",
  },
  {
    fKey: 2,
    mode: "prediction",
    label: "Predictions",
    description:
      "63 intra direction modes, inter MV with compound prediction overlay",
  },
  {
    fKey: 3,
    mode: "transform",
    label: "Transform",
    description: "Transform type per TU: DCT/ADST/FLIPADST/Identity (colour)",
  },
  {
    fKey: 4,
    mode: "reconstruction",
    label: "Reconstruction",
    description: "Reconstruction stages visualization",
  },
  {
    fKey: 5,
    mode: "loop-filter",
    label: "Loop Filter",
    description: "Deblocking + CDEF + Loop Restoration combined view",
  },
  {
    fKey: 6,
    mode: "cdef-filter",
    label: "CDEF Filter",
    description: "CDEF direction arrows and primary/secondary strength per SB",
  },
  {
    fKey: 7,
    mode: "super-res",
    label: "SuperRes Filter",
    description: "Downscaled region vs upscaled region boundary",
  },
  {
    fKey: 8,
    mode: "loop-restoration",
    label: "Loop Restoration",
    description:
      "Restoration unit type: Wiener (blue), Self-guided (green), None (grey)",
  },
  {
    fKey: 9,
    mode: "film-grain",
    label: "Film Grain Pixels",
    description: "Film grain synthesis: pre-grain vs post-grain comparison",
  },
  {
    fKey: 10,
    mode: "yuv",
    label: "YUV",
    description: "Decoded picture without any overlay",
  },
  // ── Info overlays ──────────────────────────────────────────────────────────
  {
    fKey: null,
    mode: "heat-map",
    label: "Heat Map",
    description: "Per-block bit-cost heatmap",
    isInfoOverlay: true,
  },
  {
    fKey: null,
    mode: "block-type",
    label: "Block Type",
    description: "Block/inter prediction type color map",
    isInfoOverlay: true,
  },
  {
    fKey: null,
    mode: "efficiency-map",
    label: "Efficiency",
    description: "Bits-per-pixel efficiency heatmap (higher = less efficient)",
    isInfoOverlay: true,
  },
  {
    fKey: null,
    mode: "psnr-overlay",
    label: "PSNR",
    description: "Per-block PSNR heatmap (requires debug YUV reference)",
    isInfoOverlay: true,
  },
];

/**
 * VP9
 *
 * F1 Coding Flow | F2 Predictions | F3 Transform | F4 Reconstruction
 * F5 Loop Filter | F6 YUV
 * Info overlays: Heat Map, Block Type, Efficiency Map, PSNR
 */
const VP9_MODES: CodecModeEntry[] = [
  {
    fKey: 1,
    mode: "coding-flow",
    label: "Coding Flow",
    description: "Superblock 64×64 → sub-block partition tree",
  },
  {
    fKey: 2,
    mode: "prediction",
    label: "Predictions",
    description: "Intra / inter prediction modes with MV arrows",
  },
  {
    fKey: 3,
    mode: "transform",
    label: "Transform",
    description: "Transform block sizes (4×4 to 32×32)",
  },
  {
    fKey: 4,
    mode: "reconstruction",
    label: "Reconstruction",
    description: "Reconstruction stages",
  },
  {
    fKey: 5,
    mode: "loop-filter",
    label: "Loop Filter",
    description: "Deblocking filter level per block",
  },
  {
    fKey: 6,
    mode: "yuv",
    label: "YUV",
    description: "Decoded picture without any overlay",
  },
  // ── Info overlays ──────────────────────────────────────────────────────────
  {
    fKey: null,
    mode: "heat-map",
    label: "Heat Map",
    description: "Per-block bit-cost heatmap",
    isInfoOverlay: true,
  },
  {
    fKey: null,
    mode: "qp-map",
    label: "QP Map",
    description: "Segment QP heatmap",
    isInfoOverlay: true,
  },
  {
    fKey: null,
    mode: "block-type",
    label: "Block Type",
    description: "Block prediction type color map",
    isInfoOverlay: true,
  },
  {
    fKey: null,
    mode: "efficiency-map",
    label: "Efficiency",
    description: "Bits-per-pixel efficiency heatmap",
    isInfoOverlay: true,
  },
  {
    fKey: null,
    mode: "psnr-overlay",
    label: "PSNR",
    description: "Per-block PSNR heatmap (requires debug YUV reference)",
    isInfoOverlay: true,
  },
];

/**
 * AVC / H.264
 *
 * F1 Coding Flow | F2 Predictions | F3 Transform | F4 Reconstruction
 * F5 Loop Filter | F6 YUV
 * Info overlays: QP Map, Heat Map, MB Type, MB Ref Indices, PSNR
 */
const AVC_MODES: CodecModeEntry[] = [
  {
    fKey: 1,
    mode: "coding-flow",
    label: "Coding Flow",
    description: "Macroblock and sub-MB partition boundaries",
  },
  {
    fKey: 2,
    mode: "prediction",
    label: "Predictions",
    description: "Intra / inter prediction with MV arrows per partition",
  },
  {
    fKey: 3,
    mode: "transform",
    label: "Transform",
    description: "4×4 / 8×8 transform block boundaries",
  },
  {
    fKey: 4,
    mode: "reconstruction",
    label: "Reconstruction",
    description: "Reconstruction stages",
  },
  {
    fKey: 5,
    mode: "loop-filter",
    label: "Loop Filter",
    description: "Deblocking boundary strength (BS 0–4)",
  },
  {
    fKey: 6,
    mode: "yuv",
    label: "YUV",
    description: "Decoded picture without any overlay",
  },
  // ── Info overlays ──────────────────────────────────────────────────────────
  {
    fKey: null,
    mode: "qp-map",
    label: "QP Map",
    description: "Per-MB QP heatmap",
    isInfoOverlay: true,
  },
  {
    fKey: null,
    mode: "heat-map",
    label: "Heat Map",
    description: "Per-MB bit-cost heatmap",
    isInfoOverlay: true,
  },
  {
    fKey: null,
    mode: "mb-type",
    label: "MB Type",
    description: "Macroblock prediction type color map (I/P/B/Skip)",
    isInfoOverlay: true,
  },
  {
    fKey: null,
    mode: "reference-indices",
    label: "Ref Indices",
    description: "Reference list index color map (L0/L1/BI per partition)",
    isInfoOverlay: true,
  },
  {
    fKey: null,
    mode: "psnr-overlay",
    label: "PSNR",
    description: "Per-MB PSNR heatmap (requires debug YUV reference)",
    isInfoOverlay: true,
  },
];

/**
 * MPEG-2 Video
 *
 * F1 Predictions | F2 Transform | F3 YUV
 * Info overlays: QP Map, Simple Motion
 */
const MPEG2_MODES: CodecModeEntry[] = [
  {
    fKey: 1,
    mode: "prediction",
    label: "Predictions",
    description: "Macroblock I/P/B prediction modes with MV arrows",
  },
  {
    fKey: 2,
    mode: "transform",
    label: "Transform",
    description: "8×8 DCT block boundaries",
  },
  {
    fKey: 3,
    mode: "yuv",
    label: "YUV",
    description: "Decoded picture without any overlay",
  },
  // ── Info overlays ──────────────────────────────────────────────────────────
  {
    fKey: null,
    mode: "qp-map",
    label: "QP Map",
    description: "Per-MB quantizer scale heatmap",
    isInfoOverlay: true,
  },
  {
    fKey: null,
    mode: "simple-motion",
    label: "Simple MV",
    description: "Simplified motion vector arrows",
    isInfoOverlay: true,
  },
];

/**
 * AVS3
 *
 * F1 Coding Flow | F2 Predictions | F3 Transform | F4 Reconstruction
 * F5 Loop Filter | F6 SAO | F7 ESAO | F8 CCSAO | F9 YUV
 */
const AVS3_MODES: CodecModeEntry[] = [
  {
    fKey: 1,
    mode: "coding-flow",
    label: "Coding Flow",
    description: "CTU / CU partition tree (similar to HEVC)",
  },
  {
    fKey: 2,
    mode: "prediction",
    label: "Predictions",
    description: "Intra / inter prediction modes",
  },
  {
    fKey: 3,
    mode: "transform",
    label: "Transform",
    description: "Transform block boundaries",
  },
  {
    fKey: 4,
    mode: "reconstruction",
    label: "Reconstruction",
    description: "Reconstruction stages",
  },
  {
    fKey: 5,
    mode: "loop-filter",
    label: "Loop Filter",
    description: "Deblocking filter boundaries",
  },
  {
    fKey: 6,
    mode: "sao",
    label: "SAO",
    description: "SAO type per CTU",
  },
  {
    fKey: 7,
    mode: "esao",
    label: "ESAO",
    description: "Enhanced SAO — AVS3-specific adaptive offset",
  },
  {
    fKey: 8,
    mode: "ccsao",
    label: "CCSAO",
    description: "Cross-Component SAO — chroma adaptive offset from luma",
  },
  {
    fKey: 9,
    mode: "yuv",
    label: "YUV",
    description: "Decoded picture without any overlay",
  },
  {
    fKey: null,
    mode: "qp-map",
    label: "QP Map",
    description: "Per-block QP heatmap",
    isInfoOverlay: true,
  },
];

/**
 * JPEG XS
 *
 * F1 Precinct | F2 Dequant | F3 Transform | F4 MCT | F5 NLT | F6 YUV
 */
const JPEGXS_MODES: CodecModeEntry[] = [
  {
    fKey: 1,
    mode: "precinct",
    label: "Precinct",
    description: "Precinct boundary visualization",
  },
  {
    fKey: 2,
    mode: "dequant",
    label: "Dequant",
    description: "De-quantized wavelet coefficients",
  },
  {
    fKey: 3,
    mode: "transform",
    label: "Transform",
    description: "Wavelet transform sub-band structure",
  },
  {
    fKey: 4,
    mode: "mct",
    label: "MCT",
    description: "Multiple Component Transform visualization",
  },
  {
    fKey: 5,
    mode: "nlt",
    label: "NLT",
    description: "Non-Linear Transform — tone-mapping stages",
  },
  {
    fKey: 6,
    mode: "yuv",
    label: "YUV",
    description: "Decoded picture without any overlay",
  },
];

/**
 * VC-3 / DNxHD
 *
 * F1 Segment | F2 QP Map
 */
const VC3_MODES: CodecModeEntry[] = [
  {
    fKey: 1,
    mode: "segment",
    label: "Segment",
    description: "DNxHD macroblock grid with bit-cost heatmap",
  },
  {
    fKey: null,
    mode: "qp-map",
    label: "QP Map",
    description: "Per-macroblock QP heatmap",
    isInfoOverlay: true,
  },
];

// ─── Default modes (no file / unknown codec) ─────────────────────────────────

const DEFAULT_MODES: CodecModeEntry[] = [
  {
    fKey: 1,
    mode: "overview",
    label: "Overview",
    description: "No file loaded",
  },
];

// ─── Master registry ──────────────────────────────────────────────────────────

/**
 * Maps codec identifier strings (as returned by the backend) to their mode list.
 *
 * Codec strings are matched case-insensitively via `getModesForCodec()`.
 */
export const CODEC_MODE_REGISTRY: Record<string, CodecModeEntry[]> = {
  HEVC: HEVC_MODES,
  H265: HEVC_MODES,
  "H.265": HEVC_MODES,
  VVC: VVC_MODES,
  H266: VVC_MODES,
  "H.266": VVC_MODES,
  AV1: AV1_MODES,
  VP9: VP9_MODES,
  AVC: AVC_MODES,
  H264: AVC_MODES,
  "H.264": AVC_MODES,
  MPEG2: MPEG2_MODES,
  "MPEG-2": MPEG2_MODES,
  AVS3: AVS3_MODES,
  JPEGXS: JPEGXS_MODES,
  "JPEG XS": JPEGXS_MODES,
  "JPEG-XS": JPEGXS_MODES,
  VC3: VC3_MODES,
  "VC-3": VC3_MODES,
  DNXHD: VC3_MODES,
  DNXHR: VC3_MODES,
};

// ─── Public API ───────────────────────────────────────────────────────────────

/**
 * Returns the mode list for the given codec string.
 * Falls back to DEFAULT_MODES when the codec is unknown or null.
 */
export function getModesForCodec(codec: string | null): CodecModeEntry[] {
  if (!codec) return DEFAULT_MODES;
  const upper = codec.toUpperCase().trim();
  return CODEC_MODE_REGISTRY[upper] ?? DEFAULT_MODES;
}

/**
 * Returns only the main (non-overlay) modes for a codec — the ones shown in
 * the Mode Selector dropdown and triggered by F-keys.
 */
export function getMainModesForCodec(codec: string | null): CodecModeEntry[] {
  return getModesForCodec(codec).filter((m) => !m.isInfoOverlay);
}

/**
 * Returns only the info-overlay modes for a codec (toggle overlays).
 */
export function getInfoOverlaysForCodec(
  codec: string | null,
): CodecModeEntry[] {
  return getModesForCodec(codec).filter((m) => m.isInfoOverlay);
}

/**
 * Given a codec and an F-key number, returns the corresponding mode (or null).
 */
export function getModeByFKey(
  codec: string | null,
  fKey: number,
): VisualizationMode | null {
  const entry = getModesForCodec(codec).find((m) => m.fKey === fKey);
  return entry?.mode ?? null;
}

/**
 * Given a codec and a mode, returns the F-key number (or null if it's an overlay).
 */
export function getFKeyForMode(
  codec: string | null,
  mode: VisualizationMode,
): number | null {
  const entry = getModesForCodec(codec).find((m) => m.mode === mode);
  return entry?.fKey ?? null;
}

/**
 * Returns the default (first main) mode for a codec — used when switching
 * codecs to reset to a sensible starting mode.
 */
export function getDefaultModeForCodec(
  codec: string | null,
): VisualizationMode {
  const main = getMainModesForCodec(codec);
  return main[0]?.mode ?? "overview";
}
