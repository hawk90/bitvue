import { describe, it, expect } from "vitest";
import { frameKind, frameKindClass } from "@/utils/frameType";
import { getFrameTypeColorClass } from "@/types/video";

describe("frameKind", () => {
  it.each([
    ["I", "i"],
    ["i", "i"],
    ["KEY", "i"],
    ["Intra", "i"],
    ["IDR", "i"],
    ["SI", "i"],
    ["IRAP", "i"],
    ["BLA", "unknown"],
    ["P", "p"],
    ["INTER", "p"],
    ["SP", "p"],
    ["P_FRAME", "p"],
    ["B", "b"],
    ["B_REF", "b"],
    ["BFrame", "b"],
    ["b-pyramid", "b"],
    ["SWITCH", "switch"],
    ["SW", "switch"],
    ["IO", "i"],
    ["INTRA_ONLY", "i"],
    ["UNKNOWN", "unknown"],
    ["", "unknown"],
    ["Bogus", "unknown"],
  ])("%s -> %s", (input, expected) => {
    expect(frameKind(input)).toBe(expected);
  });

  it("treats a missing type as unknown", () => {
    expect(frameKind(undefined)).toBe("unknown");
    expect(frameKind(null)).toBe("unknown");
  });
});

describe("one mapping for every widget", () => {
  it("gives the same kind to a frame type in every class family", () => {
    for (const type of ["I", "KEY", "INTRA", "P", "INTER", "B", "B_REF", "X"]) {
      const kind = frameKind(type);
      expect(frameKindClass("timeline-bar", type)).toBe(`timeline-bar-${kind}`);
      expect(frameKindClass("frame-type", type)).toBe(`frame-type-${kind}`);
      expect(getFrameTypeColorClass(type)).toBe(`frame-${kind}`);
    }
  });
});
