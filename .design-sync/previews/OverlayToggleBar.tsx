import { OverlayToggleBar } from "bitvue";
import type { CSSProperties } from "react";
import { useState } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

// `availableOverlays` mirrors the isInfoOverlay:true entries per codec in
// frontend/utils/codecModeRegistry.ts -- inlined here since OverlayToggleBar takes the list as a
// prop rather than reading the registry itself.

const AV1_OVERLAYS = [
  { fKey: null, mode: "heat-map", label: "Heat Map", description: "Per-block bit-cost heatmap", isInfoOverlay: true },
  { fKey: null, mode: "block-type", label: "Block Type", description: "Block/inter prediction type color map", isInfoOverlay: true },
  { fKey: null, mode: "efficiency-map", label: "Efficiency", description: "Bits-per-pixel efficiency heatmap", isInfoOverlay: true },
  { fKey: null, mode: "psnr-overlay", label: "PSNR", description: "Per-block PSNR heatmap (requires debug YUV reference)", isInfoOverlay: true },
] as const;

const HEVC_OVERLAYS = [
  { fKey: null, mode: "qp-map", label: "QP Map", description: "Per-block QP heatmap (jet colormap)", isInfoOverlay: true },
  { fKey: null, mode: "heat-map", label: "Heat Map", description: "Per-block bit-cost heatmap", isInfoOverlay: true },
  { fKey: null, mode: "mv-field", label: "MV Heat", description: "Motion-vector magnitude heatmap", isInfoOverlay: true },
  { fKey: null, mode: "pu-type", label: "PU Type", description: "PU prediction type color map (INTRA/INTER/SKIP/MERGE)", isInfoOverlay: true },
  { fKey: null, mode: "reference-indices", label: "Ref Indices", description: "Reference list index color map (L0/L1/BI)", isInfoOverlay: true },
  { fKey: null, mode: "psnr-overlay", label: "PSNR", description: "Per-block PSNR heatmap", isInfoOverlay: true },
] as const;

const AVC_OVERLAYS = [
  { fKey: null, mode: "qp-map", label: "QP Map", description: "Per-MB QP heatmap", isInfoOverlay: true },
  { fKey: null, mode: "heat-map", label: "Heat Map", description: "Per-MB bit-cost heatmap", isInfoOverlay: true },
  { fKey: null, mode: "mb-type", label: "MB Type", description: "Macroblock prediction type color map (I/P/B/Skip)", isInfoOverlay: true },
  { fKey: null, mode: "reference-indices", label: "Ref Indices", description: "Reference list index color map", isInfoOverlay: true },
  { fKey: null, mode: "psnr-overlay", label: "PSNR", description: "Per-MB PSNR heatmap", isInfoOverlay: true },
] as const;

export const Av1NoneActive = () => {
  const [active, setActive] = useState<Set<string>>(new Set());
  const toggle = (m: string) =>
    setActive((prev) => {
      const next = new Set(prev);
      next.has(m) ? next.delete(m) : next.add(m);
      return next;
    });
  return (
    <div style={previewBg}>
      <OverlayToggleBar
        availableOverlays={AV1_OVERLAYS as never}
        activeOverlays={active as never}
        onToggle={(m) => toggle(m as string)}
      />
    </div>
  );
};

export const Av1TwoActive = () => {
  const [active, setActive] = useState<Set<string>>(
    new Set(["heat-map", "efficiency-map"]),
  );
  const toggle = (m: string) =>
    setActive((prev) => {
      const next = new Set(prev);
      next.has(m) ? next.delete(m) : next.add(m);
      return next;
    });
  return (
    <div style={previewBg}>
      <OverlayToggleBar
        availableOverlays={AV1_OVERLAYS as never}
        activeOverlays={active as never}
        onToggle={(m) => toggle(m as string)}
      />
    </div>
  );
};

export const Hevc = () => {
  const [active, setActive] = useState<Set<string>>(new Set(["qp-map"]));
  const toggle = (m: string) =>
    setActive((prev) => {
      const next = new Set(prev);
      next.has(m) ? next.delete(m) : next.add(m);
      return next;
    });
  return (
    <div style={previewBg}>
      <OverlayToggleBar
        availableOverlays={HEVC_OVERLAYS as never}
        activeOverlays={active as never}
        onToggle={(m) => toggle(m as string)}
      />
    </div>
  );
};

export const AvcAllActive = () => {
  const [active, setActive] = useState<Set<string>>(
    new Set(["qp-map", "heat-map", "mb-type", "reference-indices", "psnr-overlay"]),
  );
  const toggle = (m: string) =>
    setActive((prev) => {
      const next = new Set(prev);
      next.has(m) ? next.delete(m) : next.add(m);
      return next;
    });
  return (
    <div style={previewBg}>
      <OverlayToggleBar
        availableOverlays={AVC_OVERLAYS as never}
        activeOverlays={active as never}
        onToggle={(m) => toggle(m as string)}
      />
    </div>
  );
};
