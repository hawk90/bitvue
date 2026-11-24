/**
 * Av1HeatMapOverlay: the Heat Map shows measured bits per pixel, not MV magnitude (#458).
 */

import { describe, it, expect, vi } from "vitest";
import { Av1HeatMapOverlay } from "@/components/panels/OverlayRenderer/renderers/Av1HeatMapRenderer";
import type { FrameInfo } from "@/types/video";

function mockCtx() {
  const rects: Array<{ fillStyle: string; x: number; y: number }> = [];
  const ctx = {
    _fillStyle: "",
    get fillStyle() {
      return this._fillStyle;
    },
    set fillStyle(v: string) {
      this._fillStyle = v;
    },
    fillRect(x: number, y: number) {
      rects.push({ fillStyle: this._fillStyle, x, y });
    },
    font: "",
    fillText: vi.fn(),
    createLinearGradient: () => ({ addColorStop: vi.fn() }),
  } as unknown as CanvasRenderingContext2D;
  return { ctx, rects };
}

const frame = (bits: number[]): FrameInfo => ({
  frame_index: 0,
  frame_type: "I",
  size: 1,
  bit_grid: {
    grid_w: bits.length,
    grid_h: 1,
    block_w: 64,
    block_h: 64,
    bits_per_pixel: bits,
    symbols_per_bit: bits.map(() => 0.1),
  },
});

describe("Av1HeatMapOverlay", () => {
  it("paints cells by bits/pixel and skips cells that coded nothing", () => {
    const { ctx, rects } = mockCtx();
    Av1HeatMapOverlay({
      ctx,
      width: 192,
      height: 64,
      frame: frame([0, 0.5, 2]),
    });
    expect(rects).toHaveLength(3); // 2 cells + legend bar
    expect(rects[0].x).toBe(64);
    expect(rects[1].x).toBe(128);
    expect(rects[0].fillStyle).not.toBe(rects[1].fillStyle);
  });

  it("labels the scale with the measured maximum", () => {
    const { ctx } = mockCtx();
    Av1HeatMapOverlay({ ctx, width: 128, height: 64, frame: frame([1, 2.5]) });
    const texts = (ctx.fillText as ReturnType<typeof vi.fn>).mock.calls.map(
      (c) => c[0],
    );
    expect(texts).toEqual(["0 bits/px", "2.50 bits/px"]);
  });

  it("draws nothing without a bit grid", () => {
    const { ctx, rects } = mockCtx();
    Av1HeatMapOverlay({
      ctx,
      width: 64,
      height: 64,
      frame: { frame_index: 0, frame_type: "I", size: 1 },
    });
    expect(rects).toHaveLength(0);
  });
});
