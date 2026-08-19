import { ModeSelector } from "bitvue";
import type { CSSProperties } from "react";
import { useState } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

// `availableModes` mirrors frontend/utils/codecModeRegistry.ts entries -- ModeSelector itself takes
// the list as a prop (injected by YuvViewerPanel from the registry) rather than reading the registry
// directly, so we inline realistic per-codec subsets here instead of importing the registry module.

const AV1_MAIN_MODES = [
  { fKey: 1, mode: "coding-flow", label: "Coding Flow", description: "Superblock → block partition" },
  { fKey: 2, mode: "prediction", label: "Predictions", description: "63 intra direction modes, inter MV" },
  { fKey: 3, mode: "transform", label: "Transform", description: "Transform type per TU" },
  { fKey: 4, mode: "reconstruction", label: "Reconstruction", description: "Reconstruction stages" },
  { fKey: 5, mode: "loop-filter", label: "Loop Filter", description: "Deblocking + CDEF + LR combined" },
  { fKey: 6, mode: "cdef-filter", label: "CDEF Filter", description: "CDEF direction/strength per SB" },
  { fKey: 7, mode: "super-res", label: "SuperRes Filter", description: "Downscale/upscale boundary" },
  { fKey: 8, mode: "loop-restoration", label: "Loop Restoration", description: "Wiener / self-guided units" },
  { fKey: 9, mode: "film-grain", label: "Film Grain Pixels", description: "Pre vs post grain synthesis" },
  { fKey: 10, mode: "yuv", label: "YUV", description: "Decoded picture, no overlay" },
] as const;

const HEVC_MAIN_MODES = [
  { fKey: 1, mode: "coding-flow", label: "Coding Flow", description: "CTU / CU partition tree" },
  { fKey: 2, mode: "prediction", label: "Predictions", description: "Intra direction / inter MV arrows" },
  { fKey: 3, mode: "transform", label: "Transform", description: "TU boundaries" },
  { fKey: 4, mode: "reconstruction", label: "Reconstruction", description: "Reconstruction stages" },
  { fKey: 5, mode: "loop-filter", label: "Loop Filter", description: "Deblocking boundary strength" },
  { fKey: 6, mode: "sao", label: "SAO", description: "SAO type per CTU" },
  { fKey: 7, mode: "yuv", label: "YUV", description: "Decoded picture, no overlay" },
] as const;

const VVC_MAIN_MODES = [
  { fKey: 1, mode: "dual-tree", label: "Dual Tree", description: "Luma/chroma partition trees" },
  { fKey: 2, mode: "coding-flow", label: "Coding Flow", description: "CTU / CU partition tree" },
  { fKey: 3, mode: "prediction", label: "Predictions", description: "CCLM, MRLP, IBC, GPM arrows" },
  { fKey: 4, mode: "transform", label: "Transform", description: "MTS / LFNST transform type" },
  { fKey: 5, mode: "reconstruction", label: "Reconstruction", description: "Reconstruction incl. LMCS" },
  { fKey: 6, mode: "inverse-map", label: "Inverse Map", description: "LMCS inverse-mapping" },
  { fKey: 7, mode: "loop-filter", label: "Loop Filter", description: "Deblocking boundary strength" },
  { fKey: 8, mode: "sao", label: "SAO", description: "SAO type per CTU" },
  { fKey: 9, mode: "adaptive-filter", label: "Adaptive Filter", description: "ALF filtered vs unfiltered" },
  { fKey: 10, mode: "yuv", label: "YUV", description: "Decoded picture, no overlay" },
] as const;

const DEFAULT_MODE = [
  { fKey: 1, mode: "overview", label: "Overview", description: "No file loaded" },
] as const;

export const Av1 = () => {
  const [mode, setMode] = useState<string>("coding-flow");
  return (
    <div style={previewBg}>
      <ModeSelector
        currentMode={mode as never}
        onModeChange={(m) => setMode(m)}
        availableModes={AV1_MAIN_MODES as never}
      />
    </div>
  );
};

export const Hevc = () => {
  const [mode, setMode] = useState<string>("sao");
  return (
    <div style={previewBg}>
      <ModeSelector
        currentMode={mode as never}
        onModeChange={(m) => setMode(m)}
        availableModes={HEVC_MAIN_MODES as never}
      />
    </div>
  );
};

export const Vvc = () => {
  const [mode, setMode] = useState<string>("dual-tree");
  return (
    <div style={previewBg}>
      <ModeSelector
        currentMode={mode as never}
        onModeChange={(m) => setMode(m)}
        availableModes={VVC_MAIN_MODES as never}
      />
    </div>
  );
};

export const NoFileLoaded = () => (
  <div style={previewBg}>
    <ModeSelector
      currentMode={"overview" as never}
      onModeChange={() => {}}
      availableModes={DEFAULT_MODE as never}
    />
  </div>
);
