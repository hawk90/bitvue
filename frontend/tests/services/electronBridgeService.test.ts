import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  closeStream,
  getFrameSyntax,
  getFramesChunk,
  getHexRange,
  getStreamInfo,
  getTimeline,
  hasElectronBridge,
  indexStream,
  openStream,
  selectBitRange,
  selectFrame,
  selectSpatialBlock,
  selectSyntax,
  selectUnit,
  showOpenDialog,
} from "@/services/electronBridgeService";

function installMockBridge(
  overrides: Partial<NonNullable<Window["bitvue"]>> = {},
) {
  window.bitvue = {
    hello: vi
      .fn()
      .mockResolvedValue({ protocol_version: "0.1.0", capabilities: [] }),
    openStream: vi
      .fn()
      .mockResolvedValue({ events: [{ type: "ModelUpdated", stream: "A" }] }),
    closeStream: vi
      .fn()
      .mockResolvedValue({ events: [{ type: "ModelUpdated", stream: "A" }] }),
    selectFrame: vi.fn().mockResolvedValue({
      events: [{ type: "SelectionUpdated", stream: "A" }],
    }),
    getHexRange: vi.fn().mockResolvedValue({
      offset: 0,
      len: 4,
      bytes: new Uint8Array([1, 2, 3, 4]),
    }),
    showOpenDialog: vi.fn().mockResolvedValue("/tmp/fake.ivf"),
    indexStream: vi
      .fn()
      .mockResolvedValue({ events: [{ type: "ModelUpdated", stream: "A" }] }),
    getStreamInfo: vi
      .fn()
      .mockResolvedValue({ indexed: false, container: null }),
    getFramesChunk: vi
      .fn()
      .mockResolvedValue({ indexed: false, units: [], total_count: 0 }),
    getFrameSyntax: vi.fn().mockResolvedValue({
      type: "obu_0",
      name: "obu_0",
      value: null,
      bit_range: { start_bit: 352, end_bit: 368 },
      children: [],
    }),
    getTimeline: vi.fn().mockResolvedValue({
      stream_id: "A",
      frames: [],
      current_frame: null,
      scrub_mode: "Idle",
      viewport: [0, 0],
      vertical_viewport: [0, 0],
    }),
    onSidecarRestarted: vi.fn().mockReturnValue(() => {}),
    ...overrides,
  };
}

describe("electronBridgeService", () => {
  afterEach(() => {
    delete (window as { bitvue?: unknown }).bitvue;
  });

  describe("hasElectronBridge", () => {
    it("returns false when window.bitvue is absent", () => {
      expect(hasElectronBridge()).toBe(false);
    });

    it("returns true when window.bitvue is present", () => {
      installMockBridge();
      expect(hasElectronBridge()).toBe(true);
    });
  });

  describe("when window.bitvue is unavailable", () => {
    it("openStream throws a clear error instead of a cryptic 'undefined is not a function'", async () => {
      await expect(openStream("A", "/tmp/x.ivf")).rejects.toThrow(
        /window\.bitvue is unavailable/,
      );
    });
  });

  describe("openStream", () => {
    beforeEach(() => installMockBridge());

    it("reports success and passes through events when only ModelUpdated comes back", async () => {
      const result = await openStream("A", "/tmp/x.ivf");
      expect(result).toEqual({
        success: true,
        path: "/tmp/x.ivf",
        events: [{ type: "ModelUpdated", stream: "A" }],
        error: undefined,
      });
    });

    it("reports failure when a DiagnosticAdded event comes back (Core's error-as-event design)", async () => {
      installMockBridge({
        openStream: vi.fn().mockResolvedValue({
          events: [
            {
              type: "DiagnosticAdded",
              diagnostic: "Failed to open file: not found",
            },
          ],
        }),
      });
      const result = await openStream("A", "/tmp/missing.ivf");
      expect(result.success).toBe(false);
      expect(result.error).toContain("Failed to open file");
    });

    it("forwards the exact stream and path arguments to the bridge", async () => {
      await openStream("B", "/some/path.mkv");
      expect(window.bitvue!.openStream).toHaveBeenCalledWith(
        "B",
        "/some/path.mkv",
      );
    });
  });

  describe("closeStream / selectFrame", () => {
    beforeEach(() => installMockBridge());

    it("closeStream returns the raw events array", async () => {
      const events = await closeStream("A");
      expect(events).toEqual([{ type: "ModelUpdated", stream: "A" }]);
    });

    it("selectFrame forwards frameIndex and returns events", async () => {
      const events = await selectFrame("A", 42);
      expect(window.bitvue!.selectFrame).toHaveBeenCalledWith("A", 42);
      expect(events).toEqual([{ type: "SelectionUpdated", stream: "A" }]);
    });
  });

  describe("getHexRange / showOpenDialog", () => {
    beforeEach(() => installMockBridge());

    it("getHexRange passes through the raw result", async () => {
      const result = await getHexRange("A", 10, 4);
      expect(result.bytes).toEqual(new Uint8Array([1, 2, 3, 4]));
    });

    it("showOpenDialog returns the selected path", async () => {
      const path = await showOpenDialog([
        { name: "Video", extensions: ["ivf"] },
      ]);
      expect(path).toBe("/tmp/fake.ivf");
    });

    it("showOpenDialog returns null when the user cancels", async () => {
      installMockBridge({ showOpenDialog: vi.fn().mockResolvedValue(null) });
      const path = await showOpenDialog();
      expect(path).toBeNull();
    });
  });

  describe("indexStream / getStreamInfo / getFramesChunk", () => {
    beforeEach(() => installMockBridge());

    it("indexStream forwards the stream and returns events", async () => {
      const events = await indexStream("A");
      expect(window.bitvue!.indexStream).toHaveBeenCalledWith("A");
      expect(events).toEqual([{ type: "ModelUpdated", stream: "A" }]);
    });

    it("getStreamInfo passes through indexed:false before anything's been indexed", async () => {
      const result = await getStreamInfo("A");
      expect(result).toEqual({ indexed: false, container: null });
    });

    it("getStreamInfo passes through a populated container", async () => {
      installMockBridge({
        getStreamInfo: vi.fn().mockResolvedValue({
          indexed: true,
          container: {
            format: "Ivf",
            codec: "av1",
            track_count: 1,
            width: 352,
            height: 288,
          },
        }),
      });
      const result = await getStreamInfo("A");
      expect(result.indexed).toBe(true);
      expect(result.container?.codec).toBe("av1");
    });

    it("getFramesChunk forwards stream/offset/limit and returns the raw result", async () => {
      installMockBridge({
        getFramesChunk: vi.fn().mockResolvedValue({
          indexed: true,
          units: [{ frame_index: 0, frame_type: "I" }],
          total_count: 42,
        }),
      });
      const result = await getFramesChunk("A", 10, 5);
      expect(window.bitvue!.getFramesChunk).toHaveBeenCalledWith("A", 10, 5);
      expect(result.total_count).toBe(42);
      expect(result.units).toHaveLength(1);
    });
  });

  describe("getFrameSyntax", () => {
    beforeEach(() => installMockBridge());

    it("forwards stream/frameIndex and returns the real nested tree", async () => {
      installMockBridge({
        getFrameSyntax: vi.fn().mockResolvedValue({
          type: "obu_0",
          name: "obu_0",
          value: null,
          bit_range: { start_bit: 352, end_bit: 368 },
          children: [
            {
              type: "obu_forbidden_bit",
              name: "obu_forbidden_bit",
              value: "0",
              bit_range: { start_bit: 352, end_bit: 353 },
              children: [],
            },
          ],
        }),
      });
      const tree = await getFrameSyntax("A", 3);
      expect(window.bitvue!.getFrameSyntax).toHaveBeenCalledWith("A", 3);
      expect(tree.children).toHaveLength(1);
      expect(tree.children[0].name).toBe("obu_forbidden_bit");
      expect(tree.children[0].bit_range.start_bit).toBe(352);
    });

    it("rejects when the bridge rejects (a real wire error, not an {indexed:false} payload)", async () => {
      installMockBridge({
        getFrameSyntax: vi.fn().mockRejectedValue(new Error("frame not found")),
      });
      await expect(getFrameSyntax("A", 999)).rejects.toThrow("frame not found");
    });
  });

  describe("getTimeline", () => {
    beforeEach(() => installMockBridge());

    it("forwards the stream and returns the real display-order timeline", async () => {
      installMockBridge({
        getTimeline: vi.fn().mockResolvedValue({
          stream_id: "A",
          frames: [
            {
              display_idx: 0,
              size_bytes: 5488,
              frame_type: "KEY_FRAME",
              marker: "Key",
              pts: 0,
              dts: null,
              is_selected: false,
            },
          ],
          current_frame: null,
          scrub_mode: "Idle",
          viewport: [0, 0],
          vertical_viewport: [0, 0],
        }),
      });
      const timeline = await getTimeline("A");
      expect(window.bitvue!.getTimeline).toHaveBeenCalledWith("A");
      expect(timeline.frames).toHaveLength(1);
      expect(timeline.frames[0].marker).toBe("Key");
    });

    it("rejects when the bridge rejects (no units indexed yet)", async () => {
      installMockBridge({
        getTimeline: vi
          .fn()
          .mockRejectedValue(new Error("No units indexed for this stream")),
      });
      await expect(getTimeline("A")).rejects.toThrow("No units indexed");
    });
  });

  describe("structural (multi-sync) selection", () => {
    beforeEach(() =>
      installMockBridge({
        selectUnit: vi.fn().mockResolvedValue({
          events: [{ type: "SelectionUpdated", stream: "A" }],
        }),
        selectSyntax: vi.fn().mockResolvedValue({
          events: [{ type: "SelectionUpdated", stream: "A" }],
        }),
        selectBitRange: vi.fn().mockResolvedValue({
          events: [{ type: "SelectionUpdated", stream: "A" }],
        }),
        selectSpatialBlock: vi.fn().mockResolvedValue({
          events: [{ type: "SelectionUpdated", stream: "A" }],
        }),
      }),
    );

    it("selectUnit forwards unitType/offset/size and returns events", async () => {
      const events = await selectUnit("A", "OBU_FRAME_HEADER", 100, 50);
      expect(window.bitvue!.selectUnit).toHaveBeenCalledWith(
        "A",
        "OBU_FRAME_HEADER",
        100,
        50,
      );
      expect(events).toEqual([{ type: "SelectionUpdated", stream: "A" }]);
    });

    it("selectSyntax forwards nodeId/startBit/endBit and returns events", async () => {
      const events = await selectSyntax("A", "node-42", 352, 360);
      expect(window.bitvue!.selectSyntax).toHaveBeenCalledWith(
        "A",
        "node-42",
        352,
        360,
      );
      expect(events).toEqual([{ type: "SelectionUpdated", stream: "A" }]);
    });

    it("selectBitRange forwards startBit/endBit and returns events", async () => {
      const events = await selectBitRange("A", 100, 200);
      expect(window.bitvue!.selectBitRange).toHaveBeenCalledWith("A", 100, 200);
      expect(events).toEqual([{ type: "SelectionUpdated", stream: "A" }]);
    });

    it("selectSpatialBlock forwards x/y/w/h and returns events", async () => {
      const events = await selectSpatialBlock("A", 16, 32, 8, 8);
      expect(window.bitvue!.selectSpatialBlock).toHaveBeenCalledWith(
        "A",
        16,
        32,
        8,
        8,
      );
      expect(events).toEqual([{ type: "SelectionUpdated", stream: "A" }]);
    });
  });
});
