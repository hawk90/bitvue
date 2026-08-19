/**
 * AVC Macroblock Type Overlay Renderer
 *
 * Info overlay: colors each 16×16 macroblock by its MB type.
 *
 * Color scheme (VQ Analyzer parity):
 *   I4x4   → deep blue    rgba(41, 98,255,0.55)
 *   I16x16 → blue         rgba(66,135,245,0.55)
 *   IPCM   → light blue   rgba(100,181,246,0.55)
 *   PLuma  → green        rgba(52,168,83,0.55)
 *   P8x8   → lime         rgba(139,195,74,0.55)
 *   BDirect→ amber        rgba(255,143,0,0.55)
 *   B16x16 → orange       rgba(255,87,34,0.55)
 *   B16x8  → deep orange  rgba(230,74,25,0.55)
 *   B8x16  → red-orange   rgba(213,62,4,0.55)
 *   B8x8   → deep red     rgba(183,28,28,0.55)
 *   PSkip  → yellow       rgba(251,192,45,0.55)
 *   BSkip  → gray         rgba(158,158,158,0.55)
 *
 * Type indices match bitvue_avc::MbType enum discriminants set in
 * extract_mb_type_grid():
 *   0=I4x4, 1=I16x16, 2=IPCM, 3=PLuma, 4=P8x8, 5=BDirect,
 *   6=B16x16, 7=B16x8, 8=B8x16, 9=B8x8, 10=PSkip, 11=BSkip
 */

import type { OverlayRendererProps } from "../types";

const MB_TYPE_COLORS: Record<number, string> = {
  0: "rgba( 41, 98,255,0.55)", // I4x4
  1: "rgba( 66,135,245,0.55)", // I16x16
  2: "rgba(100,181,246,0.55)", // IPCM
  3: "rgba( 52,168, 83,0.55)", // PLuma
  4: "rgba(139,195, 74,0.55)", // P8x8
  5: "rgba(255,143,  0,0.55)", // BDirect
  6: "rgba(255, 87, 34,0.55)", // B16x16
  7: "rgba(230, 74, 25,0.55)", // B16x8
  8: "rgba(213, 62,  4,0.55)", // B8x16
  9: "rgba(183, 28, 28,0.55)", // B8x8
  10: "rgba(251,192, 45,0.55)", // PSkip
  11: "rgba(158,158,158,0.55)", // BSkip
};

const MB_TYPE_LABELS: Record<number, string> = {
  0: "I4×4",
  1: "I16×16",
  2: "IPCM",
  3: "P-Luma",
  4: "P8×8",
  5: "B-Direct",
  6: "B16×16",
  7: "B16×8",
  8: "B8×16",
  9: "B8×8",
  10: "P-Skip",
  11: "B-Skip",
};

export function AvcMbTypeOverlay({
  ctx,
  width,
  height,
  frame,
}: OverlayRendererProps): void {
  const grid = frame.mb_type_grid;
  if (!grid) return;

  const { grid_w, grid_h, block_w, block_h, mb_types } = grid;
  if (!mb_types || mb_types.length === 0) return;

  const scaleX = width / (grid_w * block_w);
  const scaleY = height / (grid_h * block_h);
  const bw = block_w * scaleX;
  const bh = block_h * scaleY;

  // Collect types present for the legend
  const presentTypes = new Set<number>();

  for (let row = 0; row < grid_h; row++) {
    for (let col = 0; col < grid_w; col++) {
      const idx = row * grid_w + col;
      const typeIdx = mb_types[idx];
      if (typeIdx == null) continue;

      const color = MB_TYPE_COLORS[typeIdx] ?? "rgba(128,128,128,0.40)";
      ctx.fillStyle = color;
      ctx.fillRect(col * bw, row * bh, bw, bh);
      presentTypes.add(typeIdx);
    }
  }

  // ── Legend (only types actually used in this frame) ────────────────────
  const legendItems = Array.from(presentTypes)
    .sort((a, b) => a - b)
    .map((t) => ({
      color: (MB_TYPE_COLORS[t] ?? "rgba(128,128,128,0.80)").replace(
        /[\d.]+\)$/,
        "0.85)",
      ),
      label: MB_TYPE_LABELS[t] ?? `Type ${t}`,
    }));

  if (legendItems.length === 0) return;

  const swatchSize = 10;
  const itemH = 16;
  const padX = 8;
  const padY = 6;
  const boxW = 95;
  const boxH = padY * 2 + legendItems.length * itemH;
  const boxX = 10;
  const boxY = 10;

  ctx.fillStyle = "rgba(0,0,0,0.70)";
  ctx.fillRect(boxX, boxY, boxW, boxH);
  ctx.font = "10px sans-serif";

  legendItems.forEach((item, i) => {
    const y = boxY + padY + i * itemH;
    ctx.fillStyle = item.color;
    ctx.fillRect(boxX + padX, y, swatchSize, swatchSize);
    ctx.fillStyle = "#ffffff";
    ctx.fillText(item.label, boxX + padX + swatchSize + 5, y + 9);
  });
}
