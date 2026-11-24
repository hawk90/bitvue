/**
 * Info overlay text badges sit in the top-right so they do not print over the mode labels
 * (Coding Flow's "Blocks: N", Prediction, Transform) in the top-left (#451).
 */

import { describe, it, expect, vi } from "vitest";
import { drawInfoBadge } from "@/components/panels/OverlayRenderer/utils/helpers";
import { QPMapOverlay } from "@/components/panels/OverlayRenderer/renderers/QPMapRenderer";
import { MVFieldOverlay } from "@/components/panels/OverlayRenderer/renderers/MVFieldRenderer";
import type { FrameInfo } from "@/types/video";

function mockCtx() {
  const texts: Array<{ text: string; x: number; y: number }> = [];
  const rects: Array<{ x: number; y: number; w: number; h: number }> = [];
  const ctx = {
    font: "",
    fillStyle: "",
    textAlign: "left",
    save: vi.fn(),
    restore: vi.fn(),
    measureText: (t: string) => ({ width: t.length * 7 }),
    fillText: (text: string, x: number, y: number) =>
      texts.push({ text, x, y }),
    fillRect: (x: number, y: number, w: number, h: number) =>
      rects.push({ x, y, w, h }),
  } as unknown as CanvasRenderingContext2D;
  return { ctx, texts, rects };
}

describe("drawInfoBadge", () => {
  it("anchors the box to the right edge and writes every line inside it", () => {
    const { ctx, texts, rects } = mockCtx();
    drawInfoBadge(ctx, 800, ["QP: 13 - 17", "Blocks: 5x4"]);
    const box = rects[0];
    expect(box.x + box.w).toBe(790);
    for (const t of texts) {
      expect(t.x).toBeGreaterThan(box.x);
      expect(t.x).toBeLessThan(box.x + box.w);
    }
    expect(texts.map((t) => t.text)).toEqual(["QP: 13 - 17", "Blocks: 5x4"]);
  });

  it("never starts left of the canvas on a narrow frame", () => {
    const { ctx, rects } = mockCtx();
    drawInfoBadge(ctx, 40, ["a long line of text here"]);
    expect(rects[0].x).toBe(0);
  });
});

describe("info overlays keep the top-left free", () => {
  const frame = (extra: Partial<FrameInfo>): FrameInfo => ({
    frame_index: 0,
    frame_type: "P",
    size: 1,
    ...extra,
  });

  it("QP Map labels start right of the Coding Flow label column (x=20)", () => {
    const { ctx, texts } = mockCtx();
    QPMapOverlay({
      ctx,
      width: 640,
      height: 360,
      frame: frame({
        qp_grid: {
          grid_w: 1,
          grid_h: 1,
          block_w: 64,
          block_h: 64,
          qp: [20],
          qp_min: 13,
          qp_max: 17,
        },
      }),
    });
    expect(texts.length).toBeGreaterThan(0);
    expect(texts.every((t) => t.x > 400)).toBe(true);
  });

  it("the MV field's intra-frame notice is not at the top-left either", () => {
    const { ctx, texts } = mockCtx();
    MVFieldOverlay({
      ctx,
      width: 640,
      height: 360,
      frame: frame({ frame_type: "I" }),
    });
    expect(texts).toHaveLength(1);
    expect(texts[0].x).toBeGreaterThan(300);
  });
});
