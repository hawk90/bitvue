/**
 * JPEG XS Transform Overlay Renderer
 *
 * F3 (JPEG XS): Wavelet decomposition tree visualisation.
 * Renders the quadtree partition of the frame into wavelet sub-bands,
 * colour-coded by decomposition level (finest→coarsest).
 */

import type { OverlayRendererProps } from "../types";

interface SubbandNode {
  name: string;
  x: number; // [0,1]
  y: number; // [0,1]
  w: number; // [0,1]
  h: number; // [0,1]
  level: number;
}

interface TransformMap {
  decomp_h: number;
  decomp_v: number;
  nodes: SubbandNode[];
}

/** Level colours: deepest (finest detail) → brightest */
const LEVEL_COLORS = [
  "rgba(220, 80,  80, 0.65)", // level 0 — finest HH
  "rgba(220,160,  40, 0.65)", // level 1
  "rgba(100,200,  80, 0.65)", // level 2
  "rgba( 40,160, 220, 0.65)", // level 3
  "rgba(160, 60, 220, 0.65)", // level 4 (coarsest non-LL)
];

const LL_COLOR = "rgba(255,255,255,0.25)";

export function JpegXsTransformRenderer({
  ctx,
  width,
  height,
  frame,
}: OverlayRendererProps): void {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const map = (frame as any).transform_map as TransformMap | undefined;

  if (!map?.nodes?.length) {
    const msg = "Transform: no decomp data";
    ctx.font = "13px monospace";
    const bw = ctx.measureText(msg).width + 24;
    const bx = (width - bw) / 2;
    const by = (height - 30) / 2;
    ctx.fillStyle = "rgba(0,0,0,0.70)";
    ctx.fillRect(bx, by, bw, 30);
    ctx.fillStyle = "#888888";
    ctx.fillText(msg, bx + 12, by + 20);
    return;
  }

  const { nodes, decomp_h } = map;

  for (const node of nodes) {
    const px = node.x * width;
    const py = node.y * height;
    const pw = node.w * width;
    const ph = node.h * height;

    const color =
      node.name === "LL"
        ? LL_COLOR
        : (LEVEL_COLORS[node.level] ?? LEVEL_COLORS[LEVEL_COLORS.length - 1]);

    ctx.fillStyle = color;
    ctx.fillRect(px, py, pw, ph);

    // Border
    ctx.strokeStyle = "rgba(255,255,255,0.30)";
    ctx.lineWidth = 0.5;
    ctx.strokeRect(px, py, pw, ph);

    // Label
    if (pw > 24 && ph > 12) {
      ctx.fillStyle = "rgba(255,255,255,0.90)";
      ctx.font = `${Math.min(12, ph * 0.35)}px monospace`;
      ctx.fillText(node.name, px + 4, py + ph * 0.6);
    }
  }

  // ── Legend ─────────────────────────────────────────────────────────────────
  const padX = 8;
  const padY = 8;
  const swatchSize = 10;
  const itemH = 16;
  const levels = Math.min(decomp_h, LEVEL_COLORS.length);
  const boxH = padY * 2 + (levels + 1) * itemH;
  const boxW = 110;
  const boxX = 10;
  const boxY = 10;

  ctx.fillStyle = "rgba(0,0,0,0.68)";
  ctx.fillRect(boxX, boxY, boxW, boxH);
  ctx.font = "10px sans-serif";

  ctx.fillStyle = LL_COLOR;
  ctx.fillRect(boxX + padX, boxY + padY, swatchSize, swatchSize);
  ctx.fillStyle = "#ffffff";
  ctx.fillText("LL (coarse)", boxX + padX + swatchSize + 4, boxY + padY + 9);

  for (let l = 0; l < levels; l++) {
    const y = boxY + padY + (l + 1) * itemH;
    ctx.fillStyle = LEVEL_COLORS[l];
    ctx.fillRect(boxX + padX, y, swatchSize, swatchSize);
    ctx.fillStyle = "#ffffff";
    ctx.fillText(`Level ${l + 1}`, boxX + padX + swatchSize + 4, y + 9);
  }
}
