import { QmTab } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

// Same pattern as ApsTab.tsx: QmTab fetches through the Tauri `invoke("get_codec_extended_info")`
// bridge internally, so we mock the bridge with realistic HEVC quantization-matrix data.
if (typeof window !== "undefined") {
  (window as unknown as { __TAURI_INTERNALS__: { invoke: (cmd: string, args?: unknown) => Promise<unknown> } }).__TAURI_INTERNALS__ = {
    invoke: async (cmd: string) => {
      if (cmd === "get_codec_extended_info") {
        return {
          codec: "HEVC",
          hevc_qm: {
            scaling_list_enabled: true,
            matrices: [
              {
                name: "Intra 4x4 Luma",
                size: 4,
                pred_type: "Intra",
                plane: "Y",
                values: [16, 16, 17, 21, 16, 17, 20, 25, 17, 20, 25, 30, 21, 25, 30, 36],
              },
              {
                name: "Inter 4x4 Luma",
                size: 4,
                pred_type: "Inter",
                plane: "Y",
                values: new Array(16).fill(16),
              },
              {
                name: "Intra 8x8 Chroma Cb",
                size: 8,
                pred_type: "Intra",
                plane: "Cb",
                values: new Array(64).fill(16),
              },
            ],
          },
        };
      }
      throw new Error(`unmocked command: ${cmd}`);
    },
  };
}

export const WithScalingLists = () => (
  <div style={previewBg}>
    <QmTab filePath="/Users/hawk/clips/clip_hevc.mp4" frameIndex={30} />
  </div>
);

export const NoFileLoaded = () => (
  <div style={previewBg}>
    <QmTab frameIndex={0} />
  </div>
);
