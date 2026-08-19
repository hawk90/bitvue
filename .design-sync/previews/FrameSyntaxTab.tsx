import { FrameSyntaxTab } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

// FrameSyntaxTab fetches its syntax tree via `getFrameSyntax()` (electronBridgeService ->
// `window.bitvue.getFrameSyntax`). That function is declared `async`, so a missing
// `window.bitvue` rejects safely -- but to show real content rather than the frame-info fallback
// tree, we mock `window.bitvue` directly with a realistic AV1 frame_header syntax tree (same
// mocking approach documented in .design-sync/NOTES.md for components touching the bridge).
if (typeof window !== "undefined") {
  (window as unknown as { bitvue: Record<string, unknown> }).bitvue = {
    getFrameSyntax: async () => ({
      type: "obu",
      name: "frame_header_obu",
      value: null,
      bit_range: { start_bit: 0, end_bit: 640 },
      children: [
        { type: "field", name: "show_existing_frame", value: "0", bit_range: { start_bit: 0, end_bit: 1 }, children: [] },
        { type: "field", name: "frame_type", value: "INTER_FRAME", bit_range: { start_bit: 1, end_bit: 3 }, children: [] },
        { type: "field", name: "show_frame", value: "1", bit_range: { start_bit: 3, end_bit: 4 }, children: [] },
        { type: "field", name: "base_q_idx", value: "96", bit_range: { start_bit: 4, end_bit: 12 }, children: [] },
        { type: "field", name: "primary_ref_frame", value: "2", bit_range: { start_bit: 12, end_bit: 15 }, children: [] },
      ],
    }),
  };
}

export const WithSyntaxTree = () => (
  <div style={previewBg}>
    <FrameSyntaxTab
      frame={{
        frame_index: 86,
        frame_type: "P",
        size: 184220,
        pts: 86,
        temporal_id: 1,
        display_order: 86,
        coding_order: 83,
        ref_frames: [80, 84, 85],
      }}
      expandedNodes={new Set(["frame_header_obu"])}
      onToggleNode={() => {}}
      filePath="/Users/hawk/clips/sample.ivf"
    />
  </div>
);

export const NoFrameSelected = () => (
  <div style={previewBg}>
    <FrameSyntaxTab
      frame={null}
      expandedNodes={new Set()}
      onToggleNode={() => {}}
    />
  </div>
);
