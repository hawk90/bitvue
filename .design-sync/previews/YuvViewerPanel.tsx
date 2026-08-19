import { YuvViewerPanel } from "bitvue";
import { Component, type CSSProperties, type ReactNode } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };
const panelWrapper: CSSProperties = { ...previewBg, width: 960, height: 620 };

// ── Bridge mocks ─────────────────────────────────────────────────────────────────────────────
// YuvViewerPanel fetches decoded pixels (`getDecodedFrameYuv`) and analysis grids
// (`getFrameAnalysis`) directly from `window.bitvue` on mount/frame-change (both plain async
// functions in electronBridgeService.ts, so a missing window.bitvue rejects the promise safely,
// but we mock it to show real pixel content rather than the "No frame loaded" placeholder).
// Wire shapes match `BridgeDecodedYuvFrame`/`FrameAnalysisData` in electronBridgeService.ts.
if (typeof window !== "undefined") {
  const W = 64;
  const H = 64;
  const ySize = W * H;
  const cW = W / 2;
  const cH = H / 2;
  const cSize = cW * cH;

  function makeYuvBytes(): Uint8Array {
    const bytes = new Uint8Array(ySize + cSize + cSize);
    // Y plane: diagonal gradient
    for (let y = 0; y < H; y++) {
      for (let x = 0; x < W; x++) {
        bytes[y * W + x] = Math.min(255, ((x + y) / (W + H)) * 255);
      }
    }
    // U/V planes: flat mid-gray chroma
    bytes.fill(128, ySize, ySize + cSize);
    bytes.fill(140, ySize + cSize, ySize + cSize + cSize);
    return bytes;
  }

  const decodedFrame = {
    width: W,
    height: H,
    bitDepth: 8,
    chromaSubsampling: "420" as const,
    yStride: W,
    uStride: cW,
    vStride: cW,
    yLen: ySize,
    uLen: cSize,
    vLen: cSize,
    bytes: makeYuvBytes(),
  };

  const frameAnalysis = {
    frame_index: 86,
    width: W,
    height: H,
    qp_grid: {
      grid_w: 8,
      grid_h: 8,
      block_w: 8,
      block_h: 8,
      qp: Array.from({ length: 64 }, (_, i) => 60 + (i % 8) * 4),
      qp_min: 60,
      qp_max: 92,
    },
  };

  (window as unknown as { bitvue: Record<string, unknown> }).bitvue = {
    getDecodedFrameYuv: async () => decodedFrame,
    getDebugYuvFrame: async () => decodedFrame,
    getFrameAnalysis: async () => frameAnalysis,
    getContextMenuItems: async () => ({
      items: [
        {
          id: "export-evidence",
          label: "Export Evidence Bundle…",
          command: "Export.EvidenceBundle",
          guard: "always",
          enabled: true,
          disabled_reason: null,
        },
      ],
    }),
  };
}

// ── Known blocker (flagged, not fixed) ──────────────────────────────────────────────────────
// YuvViewerPanel also calls `useYuvDiff()` (frontend/contexts/YuvDiffContext.tsx) unconditionally
// near the top of its body (`const { isLoaded: debugYuvLoaded, ... } = useYuvDiff();`). That
// context is neither one of the 7 providers the DS build harness auto-wraps every card in
// (.design-sync/config.json's `provider` chain) nor listed in `extraEntries`, so it isn't
// reachable from a preview file that (per convention) must import only from the "bitvue" bare
// specifier -- there is no `YuvDiffProvider` to wrap this component in. `useYuvDiff()` throws
// synchronously ("useYuvDiff must be used inside YuvDiffProvider") on every render regardless of
// props or the window.bitvue mocks above. Same root cause as the YuvDiffPanel preview's blocker;
// fix needs a config.json change (add "./contexts/YuvDiffContext.tsx" to extraEntries + the
// provider chain), out of scope for a previews-only pass.
//
// This preview mocks the bridge correctly and is otherwise ready to show real decoded pixels the
// moment that config change lands; meanwhile a small local error boundary (plain React, no
// relative frontend/ import) turns the crash into a legible in-card explanation.
class MissingProviderBoundary extends Component<
  { children: ReactNode },
  { failed: boolean }
> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  render() {
    if (this.state.failed) {
      return (
        <div
          style={{
            border: "1px dashed #666",
            borderRadius: 4,
            padding: 16,
            color: "#aaa",
            fontSize: 12,
            fontFamily: "monospace",
            lineHeight: 1.6,
          }}
        >
          YuvViewerPanel requires YuvDiffProvider, which isn't wired into the design-sync
          harness's provider chain (.design-sync/config.json) or extraEntries yet -- see comment
          in YuvViewerPanel.tsx preview source for details.
        </div>
      );
    }
    return this.props.children;
  }
}

export const LoadedFrame = () => (
  <div style={panelWrapper}>
    <MissingProviderBoundary>
      <YuvViewerPanel
        currentFrameIndex={86}
        totalFrames={250}
        onFrameChange={() => {}}
      />
    </MissingProviderBoundary>
  </div>
);

export const NoFileLoaded = () => (
  <div style={panelWrapper}>
    <MissingProviderBoundary>
      <YuvViewerPanel currentFrameIndex={0} totalFrames={0} onFrameChange={() => {}} />
    </MissingProviderBoundary>
  </div>
);
