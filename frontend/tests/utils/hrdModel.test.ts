import { describe, it, expect } from "vitest";
import {
  countViolations,
  formatSeconds,
  frameOfPacket,
  nearestFrame,
  occupancyBefore,
  violationsAt,
} from "@/utils/hrdModel";
import type {
  HrdModelFrameWire,
  HrdModelWire,
} from "@/services/electronBridgeService";

// 100 bits arrive over 0.0-0.1 s, 100 more over 0.1-0.2 s; the first 100 leave at 0.2 s.
const curve: [number, number][] = [
  [0, 0],
  [0.1, 100],
  [0.2, 200],
  [0.2, 100],
];

const frame = (over: Partial<HrdModelFrameWire>): HrdModelFrameWire => ({
  dfg: 0,
  packet: 0,
  frame_type: "I",
  show_frame: true,
  coded_bits: 100,
  buffer_removal_time: null,
  frame_presentation_time: null,
  first_bit_arrival: 0,
  last_bit_arrival: 0.1,
  scheduled_removal: 0.2,
  removal: 0.2,
  ...over,
});

describe("occupancyBefore", () => {
  it("interpolates between breakpoints", () => {
    expect(occupancyBefore(curve, 0.05)).toBeCloseTo(50);
    expect(occupancyBefore(curve, 0.15)).toBeCloseTo(150);
  });

  it("gives the level before a removal at the removal's own time, not after it", () => {
    expect(occupancyBefore(curve, 0.2)).toBe(200);
  });

  it("holds the first and last level outside the curve and is 0 for none", () => {
    expect(occupancyBefore(curve, -1)).toBe(0);
    expect(occupancyBefore(curve, 9)).toBe(100);
    expect(occupancyBefore([], 1)).toBe(0);
  });
});

describe("frame lookup", () => {
  const frames = [
    frame({ dfg: 0, packet: 0, removal: 0.5 }),
    frame({ dfg: 1, packet: 1, removal: 0.6, show_frame: false }),
    frame({ dfg: 2, packet: 1, removal: 0.7 }),
  ];

  it("takes the last decoded frame of a packet, the one the frame list shows", () => {
    expect(frameOfPacket(frames, 1)?.dfg).toBe(2);
    expect(frameOfPacket(frames, 0)?.dfg).toBe(0);
    expect(frameOfPacket(frames, 5)).toBeUndefined();
  });

  it("finds the frame removed nearest a time", () => {
    expect(nearestFrame(frames, 0.54)?.dfg).toBe(0);
    expect(nearestFrame(frames, 0.66)?.dfg).toBe(2);
    expect(nearestFrame([], 1)).toBeUndefined();
  });
});

describe("violations", () => {
  const violations: HrdModelWire["violations"] = [
    { kind: "smoothing_buffer_underflow", dfg: 3, packet: 3 },
    { kind: "smoothing_buffer_underflow", dfg: 4, packet: 4 },
    { kind: "smoothing_buffer_overflow", dfg: 4, packet: 4 },
    { kind: "display_frame_late", dfg: null, packet: 9 },
    { kind: "decoder_buffer_delay_level", dfg: null, packet: null },
  ];

  it("counts overflow, underflow and everything else apart", () => {
    expect(countViolations(violations)).toEqual({
      overflow: 1,
      underflow: 2,
      other: 2,
    });
    expect(countViolations([])).toEqual({
      overflow: 0,
      underflow: 0,
      other: 0,
    });
  });

  it("lists those found at one frame group", () => {
    const model = { violations } as HrdModelWire;
    expect(violationsAt(model, 4).map((v) => v.kind)).toEqual([
      "smoothing_buffer_underflow",
      "smoothing_buffer_overflow",
    ]);
    expect(violationsAt(model, 0)).toEqual([]);
  });
});

describe("formatSeconds", () => {
  it("uses milliseconds under a second", () => {
    expect(formatSeconds(0.0095)).toBe("9.5 ms");
    expect(formatSeconds(1.5)).toBe("1.50 s");
  });
});
