import { ContextMenu } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = {
  background: "#1e1e1e",
  padding: 24,
  position: "relative",
  transform: "translateZ(0)",
  width: 420,
  height: 320,
};

export const PlayerScopeAllEnabled = () => (
  <div style={previewBg}>
    <ContextMenu
      x={40}
      y={30}
      items={[
        {
          id: "copy-frame-index",
          label: "Copy Frame Index",
          command: "copy_frame_index",
          guard: "has_selection",
          enabled: true,
          disabled_reason: null,
        },
        {
          id: "export-evidence-bundle",
          label: "Export Evidence Bundle",
          command: "export_evidence_bundle",
          guard: "has_selection",
          enabled: true,
          disabled_reason: null,
        },
        {
          id: "jump-to-frame-syntax",
          label: "Jump to Frame Syntax",
          command: "jump_to_frame_syntax",
          guard: "has_selection",
          enabled: true,
          disabled_reason: null,
        },
      ]}
      onSelect={() => {}}
      onClose={() => {}}
    />
  </div>
);

export const HexViewScopeWithDisabledItems = () => (
  <div style={previewBg}>
    <ContextMenu
      x={40}
      y={30}
      items={[
        {
          id: "copy-byte-range",
          label: "Copy Byte Range",
          command: "copy_byte_range",
          guard: "has_byte_range",
          enabled: true,
          disabled_reason: null,
        },
        {
          id: "copy-hex-bytes",
          label: "Copy Hex Bytes",
          command: "copy_hex_bytes",
          guard: "has_byte_range",
          enabled: false,
          disabled_reason: "Select a byte range in the hex view first",
        },
        {
          id: "export-evidence-bundle",
          label: "Export Evidence Bundle",
          command: "export_evidence_bundle",
          guard: "has_selection",
          enabled: false,
          disabled_reason: "No active selection",
        },
      ]}
      onSelect={() => {}}
      onClose={() => {}}
    />
  </div>
);
