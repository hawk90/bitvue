import { describe, it, expect } from "vitest";
import { hrdPanelModel } from "@/hooks/useHrdInfo";
import type {
  HrdInfoWireResult,
  HrdModelWire,
} from "@/services/electronBridgeService";

const model: HrdModelWire = {
  mode: "decoding_schedule",
  bit_rate_bps: 1_500_000,
  buffer_size_bits: 1_500_000,
  decoder_buffer_delay: 45_000,
  encoder_buffer_delay: 45_000,
  low_delay_mode: false,
  dec_ct_seconds: 1 / 30,
  peak_bits: 22_000,
  frames: [],
  curve: [],
  violations: [],
};

const info = (over: Partial<HrdInfoWireResult> = {}): HrdInfoWireResult => ({
  present: true,
  model,
  model_unavailable: null,
  profile: "Main",
  frame_rate: 25,
  timing: null,
  decoder_model: null,
  operating_points: [],
  ...over,
});

describe("hrdPanelModel", () => {
  it("passes the decoder model the sidecar ran", () => {
    expect(hrdPanelModel(info())).toEqual({
      model,
      unavailableReason: null,
    });
  });

  it("has no model, and the reason the sidecar gave, when the stream signals no timing", () => {
    expect(
      hrdPanelModel(
        info({
          present: false,
          model: null,
          model_unavailable: "no decoder model: ...",
        }),
      ),
    ).toEqual({ model: null, unavailableReason: "no decoder model: ..." });
  });

  it("has no model before the answer arrives or when the stream is not AV1", () => {
    expect(hrdPanelModel(null)).toEqual({
      model: null,
      unavailableReason: null,
    });
  });
});
