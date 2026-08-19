import { ProbsTab } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

// Same pattern as ApsTab.tsx: ProbsTab fetches through the Tauri `invoke("get_codec_extended_info")`
// bridge internally rather than accepting the probability table as a prop, so we mock the bridge
// with realistic VP9 probability data instead of trying to pass mock data as props.
if (typeof window !== "undefined") {
  (window as unknown as { __TAURI_INTERNALS__: { invoke: (cmd: string, args?: unknown) => Promise<unknown> } }).__TAURI_INTERNALS__ = {
    invoke: async (cmd: string) => {
      if (cmd === "get_codec_extended_info") {
        return {
          vp9_probs: {
            frame_index: 42,
            is_key_frame: false,
            entries: [
              { group: "Coefficient", label: "coef_probs[TX_4X4][Y][0]", probs: [214, 158, 92] },
              { group: "Coefficient", label: "coef_probs[TX_8X8][UV][1]", probs: [201, 133, 77] },
              { group: "Mode", label: "y_mode_probs", probs: [128, 96, 172, 64] },
              { group: "Mode", label: "partition_probs", probs: [199, 122, 41] },
              { group: "MV", label: "mv_joint_probs", probs: [32, 64, 96] },
            ],
          },
        };
      }
      throw new Error(`unmocked command: ${cmd}`);
    },
  };
}

export const WithProbabilityData = () => (
  <div style={previewBg}>
    <ProbsTab filePath="/Users/hawk/clips/clip_vp9.webm" frameIndex={42} />
  </div>
);

export const NoFileLoaded = () => (
  <div style={previewBg}>
    <ProbsTab frameIndex={0} />
  </div>
);
