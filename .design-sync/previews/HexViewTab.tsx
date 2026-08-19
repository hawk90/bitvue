import { HexViewTab } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = {
  background: "#1e1e1e",
  padding: 24,
  width: 640,
};

// HexViewTab fetches its bytes via `getHexRange()` (electronBridgeService -> `window.bitvue.getHexRange`)
// keyed off `frames[frameIndex].offset` -- there's no props path for raw bytes, so the bridge must be
// mocked (same pattern as .design-sync/previews/FrameSyntaxTab.tsx). `getHexRange` is declared `async`,
// so a missing `window.bitvue` would reject safely, but we want real hex content in the catalog rather
// than the component's error state.
//
// The mock synthesizes a plausible AV1 OBU chunk: a 00 00 01 start code followed by an OBU header byte,
// then deterministic-but-varied "compressed" bytes (a cheap hash of the byte index) so every offset/line
// looks like real bitstream data rather than a flat repeating pattern.
function synthesizeObuChunk(offset: number, len: number): Uint8Array {
  const bytes = new Uint8Array(len);
  if (len > 0) bytes[0] = 0x00;
  if (len > 1) bytes[1] = 0x00;
  if (len > 2) bytes[2] = 0x01;
  if (len > 3) bytes[3] = 0x32; // OBU header: obu_type=FRAME, has_size_field=1
  for (let i = 4; i < len; i++) {
    const h = ((offset + i) * 2654435761) >>> 0;
    bytes[i] = (h >>> 21) & 0xff;
  }
  return bytes;
}

if (typeof window !== "undefined") {
  (window as unknown as { bitvue: Record<string, unknown> }).bitvue = {
    getHexRange: async (_stream: string, offset: number, len: number) => ({
      offset,
      len,
      bytes: synthesizeObuChunk(offset, len),
    }),
    getContextMenuItems: async (
      _scope: string,
      _hasSelection: boolean,
      hasByteRange: boolean,
    ) => ({
      items: [
        {
          id: "copy-bytes",
          label: "Copy Byte",
          command: "Copy.Bytes",
          guard: "hasByteRange",
          enabled: hasByteRange,
          disabled_reason: hasByteRange ? null : "No byte selected",
        },
        {
          id: "export-evidence",
          label: "Export Evidence Bundle...",
          command: "Export.EvidenceBundle",
          guard: "always",
          enabled: true,
          disabled_reason: null,
        },
      ],
    }),
  };
}

export const Default = () => (
  <div style={previewBg}>
    <HexViewTab
      frameIndex={0}
      frames={[{ frame_index: 0, size: 512, offset: 1_048_576 }]}
    />
  </div>
);

export const TruncatedLargeFrame = () => (
  <div style={previewBg}>
    <HexViewTab
      frameIndex={0}
      frames={[{ frame_index: 0, size: 184_220, offset: 0 }]}
    />
  </div>
);

export const DeepFileOffset = () => (
  <div style={previewBg}>
    <HexViewTab
      frameIndex={958}
      frames={Array.from({ length: 959 }, (_, i) => ({
        frame_index: i,
        size: 1024,
        offset: i === 958 ? 52_428_800 : undefined,
      }))}
    />
  </div>
);

export const MissingOffset = () => (
  <div style={previewBg}>
    <HexViewTab
      frameIndex={0}
      frames={[{ frame_index: 0, size: 2048 }]}
    />
  </div>
);
