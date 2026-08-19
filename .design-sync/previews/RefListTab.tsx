import { RefListTab } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

// RefListTab fetches via `getCodecExtendedInfo()` (electronBridgeService). Unlike `getFrameSyntax`,
// that helper is a plain (non-async) function wrapping `requireBridge()` -- with no
// `window.bitvue`, `requireBridge()` throws *synchronously* inside the component's effect, which
// would bypass its own `.catch()` entirely. Mocking `window.bitvue` up front (same approach as
// FrameSyntaxTab.tsx) avoids ever hitting that path and lets the story show real reference-list
// data instead.
if (typeof window !== "undefined") {
  (window as unknown as { bitvue: Record<string, unknown> }).bitvue = {
    getCodecExtendedInfo: async (frameIndex: number) => ({
      frame_index: frameIndex,
      l0_refs: [
        { list_idx: 0, slot: 0, poc: -1, frame_index: 89, frame_type: "P", long_term: false, weight: null, offset: null },
        { list_idx: 0, slot: 1, poc: -4, frame_index: 86, frame_type: "P", long_term: false, weight: 48, offset: 2 },
        { list_idx: 0, slot: 2, poc: -8, frame_index: 82, frame_type: "I", long_term: true, weight: null, offset: null },
      ],
      // Only a B-frame (frameIndex 90 in BFrameBothLists) gets an L1 list -- a P-frame
      // is single-reference-list by definition, which is the whole point of that story.
      l1_refs:
        frameIndex === 90
          ? [
              { list_idx: 1, slot: 0, poc: 2, frame_index: 92, frame_type: "B", long_term: false, weight: null, offset: null },
            ]
          : [],
      qp_histogram: [],
    }),
  };
}

export const PFrameSingleList = () => (
  <div style={previewBg}>
    <RefListTab
      filePath="/Users/hawk/clips/sample.ivf"
      frameIndex={87}
      frameType="P"
    />
  </div>
);

export const BFrameBothLists = () => (
  <div style={previewBg}>
    <RefListTab
      filePath="/Users/hawk/clips/sample.ivf"
      frameIndex={90}
      frameType="B"
    />
  </div>
);
