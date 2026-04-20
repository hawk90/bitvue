/**
 * AVC Reference Frame Index Overlay Renderer
 *
 * Info overlay: colors each 16×16 macroblock by its L0 reference frame index.
 * Intra macroblocks (ref_idx_l0 = null) are shown as dark gray.
 *
 * Color scheme uses a hue-based palette for up to 8 reference frames:
 *   ref 0 → green   (most common reference, previous frame)
 *   ref 1 → blue
 *   ref 2 → orange
 *   ref 3 → magenta
 *   ref 4 → cyan
 *   ref 5 → red
 *   ref 6 → yellow
 *   ref 7 → purple
 *   intra → dark gray (null)
 */

import type { OverlayRendererProps } from "../types";

const REF_COLORS: string[] = [
  "rgba( 52,168, 83,0.60)", // 0 – green
  "rgba( 66,135,245,0.60)", // 1 – blue
  "rgba(255,143,  0,0.60)", // 2 – orange
  "rgba(234, 67,244,0.60)", // 3 – magenta
  "rgba(  0,200,200,0.60)", // 4 – cyan
  "rgba(234, 67, 53,0.60)", // 5 – red
  "rgba(251,192, 45,0.60)", // 6 – yellow
  "rgba(153,  0,255,0.60)", // 7 – purple
];

const INTRA_COLOR = "rgba(60,60,60,0.50)";

export function AvcRefIdxOverlay({
  ctx,
  width,
  height,
  frame,
}: OverlayRendererProps): void {
  const grid = frame.ref_idx_grid;
  if (!grid) return;

  const { grid_w, grid_h, block_w, block_h, ref_idx_l0 } = grid;
  if (!ref_idx_l0 || ref_idx_l0.length === 0) return;

  const scaleX = width / (grid_w * block_w);
  const scaleY = height / (grid_h * block_h);
  const bw = block_w * scaleX;
  const bh = block_h * scaleY;

  const presentRefs = new Set<number | null>();

  for (let row = 0; row < grid_h; row++) {
    for (let col = 0; col < grid_w; col++) {
      const idx = row * grid_w + col;
      const refIdx = ref_idx_l0[idx] ?? null;
      presentRefs.add(refIdx);

      const color =
        refIdx == null
          ? INTRA_COLOR
          : (REF_COLORS[refIdx] ??
            REF_COLORS[refIdx % REF_COLORS.length] ??
            INTRA_COLOR);
      ctx.fillStyle = color;
      ctx.fillRect(col * bw, row * bh, bw, bh);
    }
  }

  // ── Legend ────────────────────────────────────────────────────────────────
  const legendItems: { color: string; label: string }[] = [];
  if (presentRefs.has(null)) {
    legendItems.push({ color: "rgba(60,60,60,0.85)", label: "INTRA" });
  }
  Array.from(presentRefs)
    .filter((r): r is number => r !== null)
    .sort((a, b) => a - b)
    .forEach((r) => {
      legendItems.push({
        color: (REF_COLORS[r % REF_COLORS.length] ?? INTRA_COLOR).replace(
          /[\d.]+\)$/,
          "0.85)",
        ),
        label: `Ref L0[${r}]`,
      });
    });

  if (legendItems.length === 0) return;

  const swatchSize = 10;
  const itemH = 16;
  const padX = 8;
  const padY = 6;
  const boxW = 100;
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
