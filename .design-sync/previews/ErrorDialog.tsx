import { ErrorDialog } from "bitvue";
import type { CSSProperties } from "react";

// `.error-dialog-overlay` is `position: fixed` -- give the wrapper a containing block (transform)
// plus a concrete box so it renders inside the visible card instead of escaping it.
const dialogWrapper: CSSProperties = {
  background: "#1e1e1e",
  padding: 24,
  position: "relative",
  transform: "translateZ(0)",
  width: 600,
  height: 460,
};

export const ParseError = () => (
  <div style={dialogWrapper}>
    <ErrorDialog
      isOpen={true}
      title="Failed to parse bitstream"
      message="The decoder hit an unrecoverable OBU parse error while reading sample_1080p.ivf."
      errorCode="AV1_OBU_PARSE_FAILED"
      details={`thread 'decode' panicked at 'desync in SymbolDecoder at frame 132, tile 0'
  at bitvue-av1-codec/src/entropy/decoder.rs:214:9
  at bitvue-av1-codec/src/cu_parser.rs:88:5
  at bitvue-sidecar/src/commands/frame.rs:41:13`}
      onClose={() => {}}
      onDismiss={() => {}}
    />
  </div>
);

export const NoDetails = () => (
  <div style={dialogWrapper}>
    <ErrorDialog
      isOpen={true}
      title="File not found"
      message="Could not open /Users/hawk/clips/missing_clip.ivf — the file may have been moved or deleted."
      onClose={() => {}}
    />
  </div>
);
