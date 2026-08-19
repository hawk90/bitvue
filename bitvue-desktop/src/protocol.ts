/**
 * `bitvue-protocol` wire format, reimplemented in TypeScript for the Electron-main side of the
 * bridge. Mirrors `crates/bitvue-protocol/src/lib.rs` byte-for-byte — see
 * `docs/DEVELOPMENT_PHASES.md` § "제품 아키텍처 확정" / "bitvue-protocol wire schema v0" for the
 * authoritative spec this was derived from.
 *
 * Frame header — 9 bytes, little-endian:
 *   byte 0:      kind        (u8)     0 = Control, 1 = Data, 2 = Event
 *   bytes 1-4:   correlation_id (u32 LE)
 *   bytes 5-8:   payload_len (u32 LE)
 * followed immediately by `payload_len` raw bytes (no delimiter).
 */

/** Frame header size in bytes: matches `bitvue_protocol::FRAME_HEADER_LEN`. */
export const FRAME_HEADER_LEN = 9;

/** Protocol version this client speaks — matches `bitvue_protocol::PROTOCOL_VERSION`. */
export const PROTOCOL_VERSION = "0.1.0";

export enum FrameKind {
  /** Small structured JSON (`Request`/`Response`). */
  Control = 0,
  /** Raw bytes — decoded frame planes, QP/MV typed arrays, hex ranges, etc. No wrapper encoding. */
  Data = 1,
  /** Sidecar-initiated push (progress, warnings). `correlation_id` is 0 unless tied to a subscription. */
  Event = 2,
}

function isFrameKind(v: number): v is FrameKind {
  return v === FrameKind.Control || v === FrameKind.Data || v === FrameKind.Event;
}

export interface DecodedFrame {
  kind: FrameKind;
  correlationId: number;
  payload: Buffer;
}

/**
 * Encodes one frame (header + payload) ready to write to the child's stdin.
 * `payload.length` must fit in a u32 (4 GiB) — not checked here, matches the Rust side which
 * also just casts `usize as u32` (`FrameHeader.payload_len`).
 */
export function encodeFrame(kind: FrameKind, correlationId: number, payload: Buffer): Buffer {
  const header = Buffer.alloc(FRAME_HEADER_LEN);
  header.writeUInt8(kind, 0);
  header.writeUInt32LE(correlationId >>> 0, 1);
  header.writeUInt32LE(payload.length >>> 0, 5);
  return Buffer.concat([header, payload]);
}

/**
 * Incrementally parses a stream of bytes into frames. `stdout` data events can deliver partial
 * frames or multiple frames concatenated in one chunk — this buffers everything it's given and
 * only yields frames once a full header + payload has arrived, carrying any leftover bytes
 * (mid-header or mid-payload) forward to the next `push()` call.
 */
export class FrameDecoder {
  private buffer: Buffer = Buffer.alloc(0);

  /** Feed newly-received bytes in; returns zero or more fully-decoded frames. */
  push(chunk: Buffer): DecodedFrame[] {
    this.buffer = this.buffer.length > 0 ? Buffer.concat([this.buffer, chunk]) : chunk;

    const frames: DecodedFrame[] = [];
    for (;;) {
      if (this.buffer.length < FRAME_HEADER_LEN) break;

      const kindByte = this.buffer.readUInt8(0);
      if (!isFrameKind(kindByte)) {
        throw new Error(`bitvue-protocol: unknown frame kind byte: ${kindByte}`);
      }
      const correlationId = this.buffer.readUInt32LE(1);
      const payloadLen = this.buffer.readUInt32LE(5);
      const totalLen = FRAME_HEADER_LEN + payloadLen;

      if (this.buffer.length < totalLen) break; // payload not fully arrived yet

      const payload = Buffer.from(this.buffer.subarray(FRAME_HEADER_LEN, totalLen));
      frames.push({ kind: kindByte, correlationId, payload });
      this.buffer = this.buffer.subarray(totalLen);
    }
    return frames;
  }

  /** Bytes buffered but not yet enough to form a complete frame. For diagnostics/tests only. */
  get pendingByteCount(): number {
    return this.buffer.length;
  }
}

/** Control-plane request envelope (main -> sidecar). Mirrors `bitvue_protocol::Request`. */
export interface WireRequest {
  id: number;
  method: string;
  params: unknown;
}

/** Wire-level error payload. Mirrors `bitvue_protocol::WireError`. */
export interface WireError {
  code: string;
  message: string;
  offset?: number;
}

/** Control-plane response envelope (sidecar -> main). Mirrors `bitvue_protocol::Response`. */
export interface WireResponse {
  id: number;
  ok: boolean;
  result?: unknown;
  error?: WireError;
}

export interface HelloResult {
  protocol_version: string;
  capabilities: string[];
}
