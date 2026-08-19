/**
 * Tests for AV1FeaturesView component
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { AV1FeaturesView } from "../AV1FeaturesView";
import type { FrameInfo } from "@/types/video";
import type { Av1FeaturesWireResult } from "../../services/electronBridgeService";

// AV1FeaturesView fetches real data via getAv1Features (real Electron bridge call, not a Tauri
// invoke) -- unmocked, that call throws synchronously in jsdom (`window.bitvue` doesn't exist),
// which the component's .catch() only resets cdefBlocks/restorationUnits for, leaving
// filmGrain/superRes permanently null -- so their sections (guarded by `showX && stateX`, unlike
// CDEF/LoopRestoration which render unconditionally once `showX` is true) could never render
// under any test in this file, real bug or not. Mock the bridge module directly with a
// realistic resolved response instead of faking `window.bitvue`.
const mockGetAv1Features =
  vi.fn<(frameIndex: number) => Promise<Av1FeaturesWireResult>>();

vi.mock("../../services/electronBridgeService", () => ({
  getAv1Features: (frameIndex: number) => mockGetAv1Features(frameIndex),
}));

const mockWireResult: Av1FeaturesWireResult = {
  frame_index: 100,
  cdef: {
    width: 64,
    height: 64,
    block_size: 8,
    blocks: [{ x: 0, y: 0, size: 8, direction: 2, strength: 4 }],
    damping: 3,
    y_primary_strength: 4,
    y_secondary_strength: 1,
  },
  loop_restoration: {
    width: 64,
    height: 64,
    unit_size: 32,
    y_type: 1,
    units: [{ x: 0, y: 0, size: 32, restoration_type: 1 }],
  },
  film_grain: {
    enabled: true,
    seed: 42,
    scaling_shift: 8,
    ar_coeff_lag: 3,
    chroma_scaling_from_luma: false,
    overlap: true,
  },
  super_resolution: {
    enabled: true,
    scale_denominator: 12,
    upscaled_width: 128,
    upscaled_height: 128,
  },
};

describe("AV1FeaturesView", () => {
  const mockFrame: FrameInfo = {
    frame_index: 100,
    frame_type: "P",
    poc: 100,
    pts: 100,
    size: 25000,
    temporal_id: 1,
    spatial_id: 0,
    ref_frames: [99, 101],
  };

  beforeEach(() => {
    mockGetAv1Features.mockReset();
    mockGetAv1Features.mockResolvedValue(mockWireResult);
  });

  it("renders without crashing", () => {
    render(<AV1FeaturesView frame={mockFrame} width={64} height={64} />);
    expect(screen.getByText("AV1 Advanced Features")).toBeInTheDocument();
  });

  it("displays frame information", () => {
    render(<AV1FeaturesView frame={mockFrame} width={64} height={64} />);

    expect(screen.getByText("Frame 100")).toBeInTheDocument();
    expect(screen.getByText("P")).toBeInTheDocument();
  });

  it("displays CDEF section when enabled", () => {
    render(
      <AV1FeaturesView
        frame={mockFrame}
        width={64}
        height={64}
        showCdef={true}
      />,
    );

    expect(
      screen.getByText("CDEF (Constrained Directional Enhancement Filter)"),
    ).toBeInTheDocument();
    expect(screen.getByText(/Blocks:/)).toBeInTheDocument();
  });

  it("displays Loop Restoration section when enabled", () => {
    render(
      <AV1FeaturesView
        frame={mockFrame}
        width={64}
        height={64}
        showLoopRestoration={true}
      />,
    );

    expect(screen.getByText("Loop Restoration")).toBeInTheDocument();
  });

  it("displays Film Grain section when enabled", async () => {
    render(
      <AV1FeaturesView
        frame={mockFrame}
        width={64}
        height={64}
        showFilmGrain={true}
      />,
    );

    await waitFor(() =>
      expect(screen.getByText("Film Grain Synthesis")).toBeInTheDocument(),
    );
  });

  it("displays Super Resolution section when enabled", async () => {
    render(
      <AV1FeaturesView
        frame={mockFrame}
        width={64}
        height={64}
        showSuperRes={true}
      />,
    );

    await waitFor(() =>
      expect(screen.getByText("Super Resolution")).toBeInTheDocument(),
    );
  });

  it("displays film grain parameters", async () => {
    render(
      <AV1FeaturesView
        frame={mockFrame}
        width={64}
        height={64}
        showFilmGrain={true}
      />,
    );

    await waitFor(() => expect(screen.getByText(/Seed:/)).toBeInTheDocument());
    expect(screen.getByText(/AR Coeff Lag:/)).toBeInTheDocument();
  });

  it("displays legend", () => {
    render(<AV1FeaturesView frame={mockFrame} width={64} height={64} />);

    expect(screen.getByText(/CDEF Direction/)).toBeInTheDocument();
    expect(screen.getByText(/Wiener Filter/)).toBeInTheDocument();
    expect(screen.getByText(/SgrProj Filter/)).toBeInTheDocument();
  });

  it("handles null frame gracefully", () => {
    render(<AV1FeaturesView frame={null} width={64} height={64} />);

    expect(screen.getByText("No frame selected")).toBeInTheDocument();
  });
});
