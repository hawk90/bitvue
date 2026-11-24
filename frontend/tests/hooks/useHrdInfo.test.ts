import { describe, it, expect } from "vitest";
import { DEFAULT_FRAME_RATE, hrdPanelParams } from "@/hooks/useHrdInfo";
import type { HrdInfoWireResult } from "@/services/electronBridgeService";

const info = (over: Partial<HrdInfoWireResult> = {}): HrdInfoWireResult => ({
  present: true,
  profile: "Main",
  frame_rate: 25,
  timing: null,
  decoder_model: null,
  operating_points: [
    {
      idc: 0,
      seq_level_idx: 8,
      level: "4.0",
      tier: "main",
      max_bitrate_bps: 12_000_000,
      buffer_size_bits: 12_000_000,
      decoder_model: {
        decoder_buffer_delay: 100,
        encoder_buffer_delay: 50,
        low_delay_mode: false,
      },
    },
  ],
  ...over,
});

describe("hrdPanelParams", () => {
  it("gives the first operating point's level limits (buffer size in bytes) and the stream's frame rate", () => {
    expect(hrdPanelParams(info())).toEqual({
      targetBitrate: 12_000_000,
      bufferSize: 1_500_000,
      frameRate: 25,
    });
  });

  it("has no parameters, only the default frame rate, without HRD information", () => {
    expect(hrdPanelParams(null)).toEqual({ frameRate: DEFAULT_FRAME_RATE });
    expect(hrdPanelParams(info({ present: false }))).toEqual({
      frameRate: 25,
    });
  });

  it("has no parameters when the operating point lacks a decoder model or level limits", () => {
    const op = info().operating_points[0];
    expect(
      hrdPanelParams(
        info({ operating_points: [{ ...op, decoder_model: null }] }),
      ).targetBitrate,
    ).toBeUndefined();
    expect(
      hrdPanelParams(
        info({
          operating_points: [
            { ...op, max_bitrate_bps: null, buffer_size_bits: null },
          ],
        }),
      ).targetBitrate,
    ).toBeUndefined();
    expect(
      hrdPanelParams(info({ operating_points: [] })).targetBitrate,
    ).toBeUndefined();
  });

  it("falls back to the default frame rate for a variable display rate", () => {
    expect(hrdPanelParams(info({ frame_rate: null })).frameRate).toBe(
      DEFAULT_FRAME_RATE,
    );
  });
});
