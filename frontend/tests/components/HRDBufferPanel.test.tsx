/**
 * HRDBufferPanel tests.
 *
 * The panel plots the decoder model the sidecar ran (spec Annex E, #587). A stream with no model
 * must say "no HRD information" rather than simulate from an average-bitrate guess (#461).
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@/test/test-utils";
import { HRDBufferPanel } from "../../components/panels/HRDBufferPanel";
import type {
  HrdModelFrameWire,
  HrdModelWire,
} from "../../services/electronBridgeService";

const frame = (over: Partial<HrdModelFrameWire>): HrdModelFrameWire => ({
  dfg: 0,
  packet: 0,
  frame_type: "I",
  show_frame: true,
  coded_bits: 400_000,
  buffer_removal_time: 1,
  frame_presentation_time: 0,
  first_bit_arrival: 0,
  last_bit_arrival: 0.2,
  scheduled_removal: 0.5,
  removal: 0.5,
  ...over,
});

// Two frames of 400 kbit arriving at 2 Mbit/s into a 1 Mbit buffer: 400 kbit are in it when the
// first is removed at 0.5 s (40% of the buffer), 800 kbit before the second at 1.0 s.
const model = (over: Partial<HrdModelWire> = {}): HrdModelWire => ({
  mode: "decoding_schedule",
  bit_rate_bps: 2_000_000,
  buffer_size_bits: 1_000_000,
  decoder_buffer_delay: 45_000,
  encoder_buffer_delay: 45_000,
  low_delay_mode: false,
  dec_ct_seconds: 1 / 30,
  peak_bits: 800_000,
  frames: [
    frame({}),
    frame({
      dfg: 1,
      packet: 1,
      frame_type: "P",
      first_bit_arrival: 0.2,
      last_bit_arrival: 0.4,
      scheduled_removal: 1.0,
      removal: 1.0,
    }),
  ],
  curve: [
    [0, 0],
    [0.2, 400_000],
    [0.4, 800_000],
    [0.5, 800_000],
    [0.5, 400_000],
    [1.0, 400_000],
    [1.0, 0],
  ],
  violations: [],
  ...over,
});

const bufferStat = (container: HTMLElement) =>
  Array.from(container.querySelectorAll(".hrd-stat"))
    .find((el) => el.querySelector(".stat-label")?.textContent === "Buffer:")
    ?.querySelector(".stat-value")?.textContent;

describe("HRDBufferPanel", () => {
  it("shows 'No HRD information' and draws nothing without a model", () => {
    const { container } = render(
      <HRDBufferPanel model={null} currentFrameIndex={0} />,
    );

    expect(screen.getByText(/No HRD information/)).toBeInTheDocument();
    expect(container.querySelector("canvas")).toBeNull();
    expect(screen.queryByText(/Buffer:/)).not.toBeInTheDocument();
    expect(screen.queryByText(/Mbps/)).not.toBeInTheDocument();
  });

  it("gives the reason the stream has no model", () => {
    render(
      <HRDBufferPanel
        model={null}
        unavailableReason="no decoder model: the first operating point has none"
        currentFrameIndex={0}
      />,
    );
    expect(screen.getByText(/No HRD information/)).toBeInTheDocument();
    expect(
      screen.getByText(/the first operating point has none/),
    ).toBeInTheDocument();
  });

  it("shows the mode, the level's bit rate and the buffer level at the selected frame", () => {
    const { container } = render(
      <HRDBufferPanel model={model()} currentFrameIndex={0} />,
    );

    expect(container.querySelector("canvas")).not.toBeNull();
    expect(screen.getByText("Decoding schedule")).toBeInTheDocument();
    expect(screen.getByText("2.00 Mbps")).toBeInTheDocument();
    // 800 kbit in a 1 Mbit buffer just before frame 0 is removed
    expect(bufferStat(container)).toBe("80.0%");
    expect(screen.queryByText(/Overflow:/)).not.toBeInTheDocument();
    expect(screen.queryByText(/Underflow:/)).not.toBeInTheDocument();
  });

  it("follows the selected frame, not the end of the stream", () => {
    const { container } = render(
      <HRDBufferPanel model={model()} currentFrameIndex={1} />,
    );
    // 400 kbit left in the buffer before frame 1
    expect(bufferStat(container)).toBe("40.0%");
  });

  it("names the resource availability mode and low delay", () => {
    render(
      <HRDBufferPanel
        model={model({ mode: "resource_availability", low_delay_mode: true })}
        currentFrameIndex={0}
      />,
    );
    expect(
      screen.getByText("Resource availability (low delay)"),
    ).toBeInTheDocument();
  });

  it("counts overflow, underflow and other violations", () => {
    render(
      <HRDBufferPanel
        model={model({
          violations: [
            { kind: "smoothing_buffer_underflow", dfg: 0, packet: 0 },
            { kind: "smoothing_buffer_underflow", dfg: 1, packet: 1 },
            { kind: "smoothing_buffer_overflow", dfg: 1, packet: 1 },
            { kind: "display_frame_late", dfg: 1, packet: 1 },
          ],
        })}
        currentFrameIndex={0}
      />,
    );
    const stat = (label: string) =>
      screen.getByText(label).parentElement?.querySelector(".stat-value")
        ?.textContent;
    expect(stat("Underflow:")).toBe("2");
    expect(stat("Overflow:")).toBe("1");
    expect(stat("Other violations:")).toBe("1");
  });

  it("says so when the stream as a whole breaks a requirement", () => {
    render(
      <HRDBufferPanel
        model={model({
          violations: [
            { kind: "decoder_buffer_delay_level", dfg: null, packet: null },
          ],
        })}
        currentFrameIndex={0}
      />,
    );
    expect(screen.getByRole("alert")).toHaveTextContent(
      /decoder_buffer_delay is 0 or longer/,
    );
  });

  it("renders a model with no frames and a selected frame outside it", () => {
    const { container } = render(
      <HRDBufferPanel
        model={model({ frames: [], curve: [], peak_bits: 0 })}
        currentFrameIndex={7}
      />,
    );
    expect(container.querySelector("canvas")).not.toBeNull();
    expect(bufferStat(container)).toBeUndefined();
  });
});
