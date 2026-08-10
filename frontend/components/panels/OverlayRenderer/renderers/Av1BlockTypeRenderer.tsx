/**
 * AV1 / VP9 Block Type Overlay Renderer
 *
 * Info overlay: colors each block by prediction mode category.
 *
 * Color scheme (VQ Analyzer parity):
 *   INTRA        → blue   rgba(66,135,245,0.50)
 *   INTER        → green  rgba(52,168,83,0.50)
 *   SKIP         → yellow rgba(251,188,5,0.50)
 *   INTRA_BC     → purple rgba(153,0,255,0.50)
 *   COMPOUND     → orange rgba(255,140,0,0.50)
 *
 * Uses `mv_grid.mode` array. Values match `bitvue_engine::mv_overlay::BlockMode`'s real
 * `#[repr(u8)]` discriminants exactly (frame_analysis.rs's `mv_grid_to_json` sends
 * `m as u8` -- the enum's own values, not a remapped wire schema): 0 = None (no CU found for
 * this cell, e.g. off frame edge), 1 = Inter, 2 = Intra, 3 = Skip, 4 = IntraBc, 5 = Compound.
 *
 * (2026-08-11: this file previously assumed 0=Inter/1=Intra/2=Skip, an off-by-one mismatch
 * against the real enum -- every block was shown one category off. Fixed alongside adding real
 * IntraBc/Compound categorization, see ref_frame() parsing in `parse_coding_unit`.)
 */

import type { OverlayRendererProps } from "../types";

// BlockMode values -- must match bitvue_engine::mv_overlay::BlockMode's real #[repr(u8)]
// discriminants exactly (see this file's doc comment for why that's worth calling out).
const MODE_NONE = 0;
const MODE_INTER = 1;
const MODE_INTRA = 2;
const MODE_SKIP = 3;
const MODE_INTRA_BC = 4;
const MODE_COMPOUND = 5;

const MODE_COLORS: Record<number, string> = {
  [MODE_INTRA]: "rgba(66,  135, 245, 0.48)", // blue
  [MODE_INTER]: "rgba(52,  168,  83, 0.45)", // green
  [MODE_SKIP]: "rgba(251, 188,   5, 0.48)", // yellow
  [MODE_INTRA_BC]: "rgba(153,  0, 255, 0.48)", // purple
  [MODE_COMPOUND]: "rgba(255, 140,   0, 0.48)", // orange
};
const LEGEND_ITEMS = [
  { color: "rgba(66,135,245,0.80)", label: "INTRA" },
  { color: "rgba(52,168,83,0.80)", label: "INTER" },
  { color: "rgba(251,188,5,0.80)", label: "SKIP" },
  { color: "rgba(153,0,255,0.80)", label: "INTRA_BC" },
  { color: "rgba(255,140,0,0.80)", label: "COMPOUND" },
];

export function Av1BlockTypeOverlay({
  ctx,
  width,
  height,
  frame,
}: OverlayRendererProps): void {
  const mvGrid = frame.mv_grid;
  if (!mvGrid?.mode) return; // no-op if no mode data

  const { grid_w, grid_h, block_w, block_h, mode } = mvGrid;

  // Scale block dimensions to canvas size
  const scaleX = width / (grid_w * block_w);
  const scaleY = height / (grid_h * block_h);
  const bw = block_w * scaleX;
  const bh = block_h * scaleY;

  for (let row = 0; row < grid_h; row++) {
    for (let col = 0; col < grid_w; col++) {
      const idx = row * grid_w + col;
      const blockMode = mode[idx] ?? MODE_NONE;
      const color = MODE_COLORS[blockMode] ?? "rgba(128,128,128,0.30)";

      ctx.fillStyle = color;
      ctx.fillRect(col * bw, row * bh, bw, bh);
    }
  }

  // ── Legend ────────────────────────────────────────────────────────────────
  const swatchSize = 10;
  const itemH = 16;
  const padX = 8;
  const padY = 8;
  const boxW = 105;
  const boxH = padY * 2 + LEGEND_ITEMS.length * itemH;
  const boxX = 10;
  const boxY = 10;

  ctx.fillStyle = "rgba(0,0,0,0.65)";
  ctx.fillRect(boxX, boxY, boxW, boxH);
  ctx.font = "10px sans-serif";

  LEGEND_ITEMS.forEach((item, i) => {
    const y = boxY + padY + i * itemH;
    ctx.fillStyle = item.color;
    ctx.fillRect(boxX + padX, y, swatchSize, swatchSize);
    ctx.fillStyle = "#ffffff";
    ctx.fillText(item.label, boxX + padX + swatchSize + 5, y + 9);
  });
}
