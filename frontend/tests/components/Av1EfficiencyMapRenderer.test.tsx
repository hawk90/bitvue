/**
 * Av1EfficiencyMapRenderer Tests
 *
 * Covers the 2026-08-10 rewiring to real per-block residual energy data
 * (`frame.energy_grid`) -- verifies the renderer actually uses it when present, and falls back
 * to the old QP-derived heuristic only when it's absent.
 */

import { describe, it, expect, vi } from "vitest";
import { Av1EfficiencyMapOverlay } from "@/components/panels/OverlayRenderer/renderers/Av1EfficiencyMapRenderer";
import type { FrameInfo } from "@/types/video";

function createMockCtx() {
  const fillRectCalls: Array<{
    fillStyle: string;
    x: number;
    y: number;
    w: number;
    h: number;
  }> = [];
  const ctx = {
    get fillStyle() {
      return this._fillStyle;
    },
    set fillStyle(v: string) {
      this._fillStyle = v;
    },
    _fillStyle: "",
    fillRect(x: number, y: number, w: number, h: number) {
      fillRectCalls.push({ fillStyle: this._fillStyle, x, y, w, h });
    },
    font: "",
    fillText: vi.fn(),
    createLinearGradient: () => ({ addColorStop: vi.fn() }),
  } as unknown as CanvasRenderingContext2D;
  return { ctx, fillRectCalls };
}

function baseFrame(): FrameInfo {
  return {
    frame_index: 0,
    frame_type: "I",
    size: 1000,
  };
}

describe("Av1EfficiencyMapOverlay", () => {
  it("uses real energy_grid data when present, not the QP proxy", () => {
    const { ctx, fillRectCalls } = createMockCtx();
    const frame: FrameInfo = {
      ...baseFrame(),
      energy_grid: {
        grid_w: 2,
        grid_h: 1,
        block_w: 64,
        block_h: 64,
        energy_bpp: [0, 100], // second cell has real residual energy, first is skip
      },
      // Deliberately different QP data -- if the renderer used this instead, cell colors
      // would come out identical (uniform QP) rather than reflecting the energy split above.
      qp_grid: {
        grid_w: 2,
        grid_h: 1,
        block_w: 64,
        block_h: 64,
        qp: [32, 32],
        qp_min: 32,
        qp_max: 32,
      },
    };

    Av1EfficiencyMapOverlay({ ctx, width: 128, height: 64, frame });

    // 2 grid cells + 1 colorbar-legend rect
    expect(fillRectCalls).toHaveLength(3);
    // Cell 0 (energy=0) should be the coldest color; cell 1 (energy=max) the hottest --
    // they must differ, which the QP proxy (uniform qp=32) would not have produced.
    expect(fillRectCalls[0].fillStyle).not.toBe(fillRectCalls[1].fillStyle);
  });

  it("falls back to the QP proxy when energy_grid is absent", () => {
    const { ctx, fillRectCalls } = createMockCtx();
    const frame: FrameInfo = {
      ...baseFrame(),
      qp_grid: {
        grid_w: 2,
        grid_h: 1,
        block_w: 64,
        block_h: 64,
        qp: [10, 50],
        qp_min: 10,
        qp_max: 50,
      },
    };

    Av1EfficiencyMapOverlay({ ctx, width: 128, height: 64, frame });

    // 2 grid cells + 1 colorbar-legend rect
    expect(fillRectCalls).toHaveLength(3);
    expect(fillRectCalls[0].fillStyle).not.toBe(fillRectCalls[1].fillStyle);
  });

  it("is a no-op when neither energy_grid nor qp_grid is present", () => {
    const { ctx, fillRectCalls } = createMockCtx();
    Av1EfficiencyMapOverlay({
      ctx,
      width: 128,
      height: 64,
      frame: baseFrame(),
    });
    expect(fillRectCalls).toHaveLength(0);
  });
});
