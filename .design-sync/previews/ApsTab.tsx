import { ApsTab } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

// ApsTab fetches its data via the Tauri `invoke("get_codec_extended_info", ...)` bridge -- there's
// no prop that accepts the APS list directly. `invoke()` is declared `async function`, so a
// missing `window.__TAURI_INTERNALS__` rejects safely rather than throwing synchronously; we mock
// it here with realistic VVC APS entries (same pattern this repo's own vitest suite uses for
// electronBridgeService, see AV1FeaturesView.test.tsx) so the "real content" story actually shows
// data instead of just the fallback empty state.
if (typeof window !== "undefined") {
  (window as unknown as { __TAURI_INTERNALS__: { invoke: (cmd: string, args?: unknown) => Promise<unknown> } }).__TAURI_INTERNALS__ = {
    invoke: async (cmd: string) => {
      if (cmd === "get_codec_extended_info") {
        return {
          vvc_aps: {
            aps_list: [
              { aps_id: 0, aps_type: "ALF", enabled: true, summary: "Luma+Chroma ALF, 4 filter sets" },
              { aps_id: 1, aps_type: "ALF", enabled: false, summary: "Cross-component ALF, unused this frame" },
              { aps_id: 0, aps_type: "LMCS", enabled: true, summary: "16-piece piecewise linear mapping" },
              { aps_id: 0, aps_type: "SCALING_LIST", enabled: false, summary: "Flat scaling (disabled)" },
            ],
          },
        };
      }
      throw new Error(`unmocked command: ${cmd}`);
    },
  };
}

export const WithApsEntries = () => (
  <div style={previewBg}>
    <ApsTab filePath="/Users/hawk/clips/clip_vvc.mp4" frameIndex={12} />
  </div>
);

export const NoFileLoaded = () => (
  <div style={previewBg}>
    <ApsTab frameIndex={0} />
  </div>
);
