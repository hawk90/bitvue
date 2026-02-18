/**
 * Video Type Utilities Tests
 * Tests video type helper functions and utilities
 */

import { describe, it, expect } from "vitest";
import {
  FrameType,
  CodecType,
  ColorSpace,
  YUVFormat,
  getFrameTypeColorClass,
  getFrameTypeColor,
  isIntraFrame,
  isInterFrame,
  isKeyframe,
  type FrameInfo,
  type FileInfo,
  type StreamStats,
  type ReferenceSlot,
} from "../video";

describe("FrameType enum", () => {
  it("should have I frame type", () => {
    expect(FrameType.I).toBe("I");
  });

  it("should have P frame type", () => {
    expect(FrameType.P).toBe("P");
  });

  it("should have B frame type", () => {
    expect(FrameType.B).toBe("B");
  });

  it("should have KEY frame type", () => {
    expect(FrameType.KEY).toBe("KEY");
  });

  it("should have INTER frame type", () => {
    expect(FrameType.INTER).toBe("INTER");
  });

  it("should have INTRA frame type", () => {
    expect(FrameType.INTRA).toBe("INTRA");
  });

  it("should have SWITCH frame type", () => {
    expect(FrameType.SWITCH).toBe("SWITCH");
  });

  it("should have UNKNOWN frame type", () => {
    expect(FrameType.UNKNOWN).toBe("UNKNOWN");
  });
});

describe("CodecType enum", () => {
  it("should have VVC codec", () => {
    expect(CodecType.VVC).toBe("VVC");
  });

  it("should have HEVC codec", () => {
    expect(CodecType.HEVC).toBe("HEVC");
  });

  it("should have AV1 codec", () => {
    expect(CodecType.AV1).toBe("AV1");
  });

  it("should have VP9 codec", () => {
    expect(CodecType.VP9).toBe("VP9");
  });

  it("should have AVC codec", () => {
    expect(CodecType.AVC).toBe("AVC");
  });

  it("should have MPEG2 codec", () => {
    expect(CodecType.MPEG2).toBe("MPEG2");
  });

  it("should have UNKNOWN codec", () => {
    expect(CodecType.UNKNOWN).toBe("UNKNOWN");
  });
});

describe("ColorSpace enum", () => {
  it("should have BT601 color space", () => {
    expect(ColorSpace.BT601).toBe("BT601");
  });

  it("should have BT709 color space", () => {
    expect(ColorSpace.BT709).toBe("BT709");
  });

  it("should have BT2020 color space", () => {
    expect(ColorSpace.BT2020).toBe("BT2020");
  });

  it("should have SMPTE170M color space", () => {
    expect(ColorSpace.SMPTE170M).toBe("SMPTE170M");
  });
});

describe("YUVFormat enum", () => {
  it("should have YUV420 format", () => {
    expect(YUVFormat.YUV420).toBe("YUV420");
  });

  it("should have YUV422 format", () => {
    expect(YUVFormat.YUV422).toBe("YUV422");
  });

  it("should have YUV444 format", () => {
    expect(YUVFormat.YUV444).toBe("YUV444");
  });

  it("should have NV12 format", () => {
    expect(YUVFormat.NV12).toBe("NV12");
  });

  it("should have P010 format", () => {
    expect(YUVFormat.P010).toBe("P010");
  });
});

describe("getFrameTypeColorClass", () => {
  it("should return frame-i for I frames", () => {
    expect(getFrameTypeColorClass("I")).toBe("frame-i");
    expect(getFrameTypeColorClass("i")).toBe("frame-i");
  });

  it("should return frame-i for KEY frames", () => {
    expect(getFrameTypeColorClass("KEY")).toBe("frame-i");
    expect(getFrameTypeColorClass("key")).toBe("frame-i");
  });

  it("should return frame-i for INTRA frames", () => {
    expect(getFrameTypeColorClass("INTRA")).toBe("frame-i");
    expect(getFrameTypeColorClass("intra")).toBe("frame-i");
  });

  it("should return frame-p for P frames", () => {
    expect(getFrameTypeColorClass("P")).toBe("frame-p");
    expect(getFrameTypeColorClass("p")).toBe("frame-p");
  });

  it("should return frame-p for INTER frames", () => {
    expect(getFrameTypeColorClass("INTER")).toBe("frame-p");
    expect(getFrameTypeColorClass("inter")).toBe("frame-p");
  });

  it("should return frame-b for B frames", () => {
    expect(getFrameTypeColorClass("B")).toBe("frame-b");
    expect(getFrameTypeColorClass("b")).toBe("frame-b");
  });

  it("should return frame-unknown for unknown frame types", () => {
    expect(getFrameTypeColorClass("UNKNOWN")).toBe("frame-unknown");
    expect(getFrameTypeColorClass("X")).toBe("frame-unknown");
    expect(getFrameTypeColorClass("")).toBe("frame-unknown");
  });

  it("should handle mixed case frame types", () => {
    expect(getFrameTypeColorClass("Intra")).toBe("frame-i");
    expect(getFrameTypeColorClass("Inter")).toBe("frame-p");
  });
});

describe("getFrameTypeColor", () => {
  it("should return CSS variable for I frames", () => {
    expect(getFrameTypeColor("I")).toBe("var(--frame-i)");
    expect(getFrameTypeColor("i")).toBe("var(--frame-i)");
  });

  it("should return CSS variable for KEY frames", () => {
    expect(getFrameTypeColor("KEY")).toBe("var(--frame-i)");
    expect(getFrameTypeColor("key")).toBe("var(--frame-i)");
  });

  it("should return CSS variable for P frames", () => {
    expect(getFrameTypeColor("P")).toBe("var(--frame-p)");
    expect(getFrameTypeColor("p")).toBe("var(--frame-p)");
  });

  it("should return CSS variable for INTER frames", () => {
    expect(getFrameTypeColor("INTER")).toBe("var(--frame-p)");
    expect(getFrameTypeColor("inter")).toBe("var(--frame-p)");
  });

  it("should return CSS variable for B frames", () => {
    expect(getFrameTypeColor("B")).toBe("var(--frame-b)");
    expect(getFrameTypeColor("b")).toBe("var(--frame-b)");
  });

  it("should return CSS variable for frame types starting with B", () => {
    expect(getFrameTypeColor("BLA")).toBe("var(--frame-b)");
    expect(getFrameTypeColor("bframe")).toBe("var(--frame-b)");
  });

  it("should return text-secondary color for unknown frame types", () => {
    expect(getFrameTypeColor("UNKNOWN")).toBe("var(--text-secondary)");
    expect(getFrameTypeColor("X")).toBe("var(--text-secondary)");
    expect(getFrameTypeColor("")).toBe("var(--text-secondary)");
  });
});

describe("isIntraFrame", () => {
  it("should return true for I frames", () => {
    expect(isIntraFrame("I")).toBe(true);
    expect(isIntraFrame("i")).toBe(true);
  });

  it("should return true for KEY frames", () => {
    expect(isIntraFrame("KEY")).toBe(true);
    expect(isIntraFrame("key")).toBe(true);
  });

  it("should return true for INTRA frames", () => {
    expect(isIntraFrame("INTRA")).toBe(true);
    expect(isIntraFrame("intra")).toBe(true);
  });

  it("should return false for P frames", () => {
    expect(isIntraFrame("P")).toBe(false);
    expect(isIntraFrame("p")).toBe(false);
  });

  it("should return false for B frames", () => {
    expect(isIntraFrame("B")).toBe(false);
    expect(isIntraFrame("b")).toBe(false);
  });

  it("should return false for INTER frames", () => {
    expect(isIntraFrame("INTER")).toBe(false);
    expect(isIntraFrame("inter")).toBe(false);
  });

  it("should return false for unknown frame types", () => {
    expect(isIntraFrame("UNKNOWN")).toBe(false);
    expect(isIntraFrame("X")).toBe(false);
    expect(isIntraFrame("")).toBe(false);
  });
});

describe("isInterFrame", () => {
  it("should return true for P frames", () => {
    expect(isInterFrame("P")).toBe(true);
    expect(isInterFrame("p")).toBe(true);
  });

  it("should return true for B frames", () => {
    expect(isInterFrame("B")).toBe(true);
    expect(isInterFrame("b")).toBe(true);
  });

  it("should return true for INTER frames", () => {
    expect(isInterFrame("INTER")).toBe(true);
    expect(isInterFrame("inter")).toBe(true);
  });

  it("should return false for I frames", () => {
    expect(isInterFrame("I")).toBe(false);
    expect(isInterFrame("i")).toBe(false);
  });

  it("should return false for KEY frames", () => {
    expect(isInterFrame("KEY")).toBe(false);
    expect(isInterFrame("key")).toBe(false);
  });

  it("should return false for INTRA frames", () => {
    expect(isInterFrame("INTRA")).toBe(false);
    expect(isInterFrame("intra")).toBe(false);
  });

  it("should return false for unknown frame types", () => {
    expect(isInterFrame("UNKNOWN")).toBe(false);
    expect(isInterFrame("X")).toBe(false);
    expect(isInterFrame("")).toBe(false);
  });
});

describe("isKeyframe", () => {
  it("should return true when keyFrame property is true", () => {
    expect(isKeyframe("P", true)).toBe(true);
    expect(isKeyframe("B", true)).toBe(true);
  });

  it("should return false when keyFrame property is false", () => {
    expect(isKeyframe("I", false)).toBe(false);
    expect(isKeyframe("KEY", false)).toBe(false);
  });

  it("should return true for I frames when keyFrame is undefined", () => {
    expect(isKeyframe("I")).toBe(true);
    expect(isKeyframe("i")).toBe(true);
  });

  it("should return true for KEY frames when keyFrame is undefined", () => {
    expect(isKeyframe("KEY")).toBe(true);
    expect(isKeyframe("key")).toBe(true);
  });

  it("should return true for INTRA frames when keyFrame is undefined", () => {
    expect(isKeyframe("INTRA")).toBe(true);
    expect(isKeyframe("intra")).toBe(true);
  });

  it("should return false for P frames when keyFrame is undefined", () => {
    expect(isKeyframe("P")).toBe(false);
    expect(isKeyframe("p")).toBe(false);
  });

  it("should return false for B frames when keyFrame is undefined", () => {
    expect(isKeyframe("B")).toBe(false);
    expect(isKeyframe("b")).toBe(false);
  });

  it("should return false for unknown frame types when keyFrame is undefined", () => {
    expect(isKeyframe("UNKNOWN")).toBe(false);
    expect(isKeyframe("X")).toBe(false);
  });
});

describe("FrameInfo interface", () => {
  it("should accept valid frame info object", () => {
    const frame: FrameInfo = {
      frame_index: 0,
      frame_type: "I",
      size: 50000,
      poc: 0,
      key_frame: true,
    };

    expect(frame.frame_index).toBe(0);
    expect(frame.frame_type).toBe("I");
    expect(frame.size).toBe(50000);
  });

  it("should accept frame with optional properties", () => {
    const frame: FrameInfo = {
      frame_index: 1,
      frame_type: "P",
      size: 30000,
      poc: 1,
      pts: 100,
      display_order: 1,
      coding_order: 1,
      key_frame: false,
      temporal_id: 0,
      spatial_id: 0,
      ref_frames: [0],
      ref_slots: [0],
      duration: 1,
    };

    expect(frame.ref_frames).toEqual([0]);
    expect(frame.ref_slots).toEqual([0]);
  });

  it("should accept frame with ref_slot_info", () => {
    const refSlot: ReferenceSlot = {
      index: 0,
      name: "LAST",
      frameIndex: 5,
    };

    const frame: FrameInfo = {
      frame_index: 10,
      frame_type: "P",
      size: 30000,
      ref_slot_info: [refSlot],
    };

    expect(frame.ref_slot_info).toEqual([refSlot]);
  });

  it("should accept frame with thumbnail", () => {
    const frame: FrameInfo = {
      frame_index: 0,
      frame_type: "I",
      size: 50000,
      thumbnail: "data:image/png;base64,mockdata",
    };

    expect(frame.thumbnail).toBe("data:image/png;base64,mockdata");
  });
});

describe("FileInfo interface", () => {
  it("should accept valid file info object", () => {
    const fileInfo: FileInfo = {
      success: true,
      path: "/path/to/file.ivf",
      codec: "AV1",
      frameCount: 1000,
      width: 1920,
      height: 1080,
      bitrate: 5000000,
      duration: 1000,
      fps: 30,
      profile: "Main",
      level: "5.1",
      bitDepth: 10,
      chromaFormat: "YUV420",
    };

    expect(fileInfo.success).toBe(true);
    expect(fileInfo.codec).toBe("AV1");
    expect(fileInfo.frameCount).toBe(1000);
  });

  it("should accept file info with error", () => {
    const fileInfo: FileInfo = {
      success: false,
      error: "File not found",
    };

    expect(fileInfo.success).toBe(false);
    expect(fileInfo.error).toBe("File not found");
  });

  it("should handle missing optional properties", () => {
    const fileInfo: FileInfo = {
      success: true,
      path: "/path/to/file.ivf",
    };

    expect(fileInfo.codec).toBeUndefined();
    expect(fileInfo.frameCount).toBeUndefined();
  });
});

describe("StreamStats interface", () => {
  it("should accept valid stream stats object", () => {
    const stats: StreamStats = {
      totalFrames: 1000,
      keyFrames: 50,
      totalSize: 50000000,
      avgSize: 50000,
      frameTypes: { I: 50, P: 300, B: 650 },
    };

    expect(stats.totalFrames).toBe(1000);
    expect(stats.keyFrames).toBe(50);
    expect(stats.frameTypes.I).toBe(50);
  });

  it("should calculate correct frame type distribution", () => {
    const stats: StreamStats = {
      totalFrames: 100,
      keyFrames: 10,
      totalSize: 1000000,
      avgSize: 10000,
      frameTypes: { I: 10, P: 40, B: 50 },
    };

    const totalByType = Object.values(stats.frameTypes).reduce(
      (a, b) => a + b,
      0,
    );
    expect(totalByType).toBe(100);
  });
});

describe("ReferenceSlot interface", () => {
  it("should accept valid reference slot", () => {
    const slot: ReferenceSlot = {
      index: 0,
      name: "LAST",
      frameIndex: 5,
    };

    expect(slot.index).toBe(0);
    expect(slot.name).toBe("LAST");
    expect(slot.frameIndex).toBe(5);
  });

  it("should accept reference slot without frameIndex", () => {
    const slot: ReferenceSlot = {
      index: 1,
      name: "LAST2",
    };

    expect(slot.index).toBe(1);
    expect(slot.frameIndex).toBeUndefined();
  });
});

describe("video type edge cases", () => {
  it("should handle empty string in getFrameTypeColorClass", () => {
    expect(getFrameTypeColorClass("")).toBe("frame-unknown");
  });

  it("should handle special characters in frame type", () => {
    expect(getFrameTypeColorClass("I-FRAME")).toBe("frame-unknown");
    expect(getFrameTypeColorClass("P_FRAME")).toBe("frame-unknown");
  });

  it("should handle numeric frame types", () => {
    expect(getFrameTypeColorClass("1")).toBe("frame-unknown");
    expect(getFrameTypeColorClass("0")).toBe("frame-unknown");
  });

  it("should handle undefined frame type color with startsWith check", () => {
    expect(getFrameTypeColor("b")).toBe("var(--frame-b)");
    expect(getFrameTypeColor("Bla")).toBe("var(--frame-b)");
    expect(getFrameTypeColor("BLUE")).toBe("var(--frame-b)");
  });

  it("should handle case sensitivity consistently", () => {
    expect(isIntraFrame("I")).toBe(isIntraFrame("i"));
    expect(isIntraFrame("KEY")).toBe(isIntraFrame("key"));
    expect(isInterFrame("P")).toBe(isInterFrame("p"));
    expect(isInterFrame("B")).toBe(isInterFrame("b"));
  });
});
