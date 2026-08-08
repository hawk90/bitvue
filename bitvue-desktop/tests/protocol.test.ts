import { describe, expect, it } from "vitest";
import { FRAME_HEADER_LEN, FrameDecoder, FrameKind, encodeFrame } from "../src/protocol.js";

describe("encodeFrame", () => {
  it("writes the 9-byte header in the documented little-endian layout", () => {
    const payload = Buffer.from("hi", "utf8");
    const frame = encodeFrame(FrameKind.Control, 0x01020304, payload);

    expect(frame.length).toBe(FRAME_HEADER_LEN + payload.length);
    expect(frame[0]).toBe(FrameKind.Control); // kind byte
    expect(frame.readUInt32LE(1)).toBe(0x01020304); // correlation_id LE
    expect(frame.readUInt32LE(5)).toBe(payload.length); // payload_len LE
    expect(frame.subarray(FRAME_HEADER_LEN)).toEqual(payload);
  });

  it("round-trips through the decoder for each frame kind", () => {
    for (const kind of [FrameKind.Control, FrameKind.Data, FrameKind.Event]) {
      const payload = Buffer.from(`payload-for-kind-${kind}`, "utf8");
      const frame = encodeFrame(kind, 7, payload);
      const decoder = new FrameDecoder();
      const [decoded] = decoder.push(frame);
      expect(decoded.kind).toBe(kind);
      expect(decoded.correlationId).toBe(7);
      expect(decoded.payload).toEqual(payload);
    }
  });

  it("handles an empty payload (payload_len = 0)", () => {
    const frame = encodeFrame(FrameKind.Event, 0, Buffer.alloc(0));
    expect(frame.length).toBe(FRAME_HEADER_LEN);
    const decoder = new FrameDecoder();
    const [decoded] = decoder.push(frame);
    expect(decoded.payload.length).toBe(0);
  });
});

describe("FrameDecoder fragmentation handling", () => {
  it("parses a single frame delivered in one chunk", () => {
    const decoder = new FrameDecoder();
    const payload = Buffer.from("hello", "utf8");
    const frames = decoder.push(encodeFrame(FrameKind.Control, 1, payload));
    expect(frames).toHaveLength(1);
    expect(frames[0].payload.toString("utf8")).toBe("hello");
    expect(decoder.pendingByteCount).toBe(0);
  });

  it("parses multiple frames concatenated into a single chunk", () => {
    const decoder = new FrameDecoder();
    const a = encodeFrame(FrameKind.Control, 1, Buffer.from("first", "utf8"));
    const b = encodeFrame(FrameKind.Data, 2, Buffer.from("second", "utf8"));
    const c = encodeFrame(FrameKind.Event, 0, Buffer.from("third", "utf8"));

    const frames = decoder.push(Buffer.concat([a, b, c]));

    expect(frames).toHaveLength(3);
    expect(frames.map((f) => f.correlationId)).toEqual([1, 2, 0]);
    expect(frames.map((f) => f.payload.toString("utf8"))).toEqual(["first", "second", "third"]);
  });

  it("reassembles a frame split mid-header", () => {
    const decoder = new FrameDecoder();
    const frame = encodeFrame(FrameKind.Control, 99, Buffer.from("split-header", "utf8"));

    // Split after 3 of the 9 header bytes — kind + partial correlation_id only.
    const part1 = frame.subarray(0, 3);
    const part2 = frame.subarray(3);

    expect(decoder.push(part1)).toHaveLength(0);
    expect(decoder.pendingByteCount).toBe(3);
    const frames = decoder.push(part2);

    expect(frames).toHaveLength(1);
    expect(frames[0].correlationId).toBe(99);
    expect(frames[0].payload.toString("utf8")).toBe("split-header");
  });

  it("reassembles a frame split mid-payload", () => {
    const decoder = new FrameDecoder();
    const payload = Buffer.from("this payload is split across two chunks", "utf8");
    const frame = encodeFrame(FrameKind.Data, 5, payload);

    // Split partway through the payload, well past the header.
    const splitAt = FRAME_HEADER_LEN + 10;
    const part1 = frame.subarray(0, splitAt);
    const part2 = frame.subarray(splitAt);

    expect(decoder.push(part1)).toHaveLength(0);
    const frames = decoder.push(part2);

    expect(frames).toHaveLength(1);
    expect(frames[0].payload).toEqual(payload);
  });

  it("reassembles correctly when fed one byte at a time", () => {
    const decoder = new FrameDecoder();
    const payload = Buffer.from("byte-by-byte", "utf8");
    const frame = encodeFrame(FrameKind.Control, 42, payload);

    let collected: ReturnType<FrameDecoder["push"]> = [];
    for (let i = 0; i < frame.length; i++) {
      collected = collected.concat(decoder.push(frame.subarray(i, i + 1)));
    }

    expect(collected).toHaveLength(1);
    expect(collected[0].correlationId).toBe(42);
    expect(collected[0].payload.toString("utf8")).toBe("byte-by-byte");
  });

  it("handles a frame boundary that falls exactly at a chunk boundary, followed by another frame split across chunks", () => {
    const decoder = new FrameDecoder();
    const a = encodeFrame(FrameKind.Control, 1, Buffer.from("aaa", "utf8"));
    const b = encodeFrame(FrameKind.Control, 2, Buffer.from("bbbbbbbbbb", "utf8"));

    // First chunk: all of `a`, plus the header and a few payload bytes of `b`.
    const chunk1 = Buffer.concat([a, b.subarray(0, FRAME_HEADER_LEN + 4)]);
    const chunk2 = b.subarray(FRAME_HEADER_LEN + 4);

    const firstBatch = decoder.push(chunk1);
    expect(firstBatch).toHaveLength(1);
    expect(firstBatch[0].correlationId).toBe(1);

    const secondBatch = decoder.push(chunk2);
    expect(secondBatch).toHaveLength(1);
    expect(secondBatch[0].correlationId).toBe(2);
    expect(secondBatch[0].payload.toString("utf8")).toBe("bbbbbbbbbb");
  });

  it("throws on an unknown frame kind byte", () => {
    const decoder = new FrameDecoder();
    const bogus = Buffer.alloc(FRAME_HEADER_LEN);
    bogus.writeUInt8(255, 0);
    expect(() => decoder.push(bogus)).toThrow(/unknown frame kind/i);
  });

  it("handles a large (multi-chunk-typical) payload split at an arbitrary boundary", () => {
    const decoder = new FrameDecoder();
    const payload = Buffer.alloc(64 * 1024);
    for (let i = 0; i < payload.length; i++) payload[i] = i % 256;
    const frame = encodeFrame(FrameKind.Data, 123, payload);

    const splitAt = 37; // arbitrary, well inside the header+payload run
    const frames = [...decoder.push(frame.subarray(0, splitAt)), ...decoder.push(frame.subarray(splitAt))];

    expect(frames).toHaveLength(1);
    expect(frames[0].payload).toEqual(payload);
  });
});
