/**
 * JPEG XS MCT (Multiple Component Transform) Overlay Renderer
 *
 * F4 (JPEG XS): Shows the inter-component decorrelation transform applied
 * before wavelet coding. Renders a per-component channel diagram with MCT
 * type labelling.
 *
 *   None  → grey channels (independent)
 *   RCT   → YCgCo (reversible, lossless)
 *   ICT   → YCbCr (irreversible, perceptual weighting)
 *   Custom → generic matrix
 */

import type { OverlayRendererProps } from "../types";

interface MctInfo {
  mct_type: string;
  num_comps: number;
  comp_labels: string[];
  reversible: boolean;
}

const MCT_TYPE_COLORS: Record<string, string> = {
  None: "rgba(120,120,120,0.55)",
  Rct: "rgba( 40,180,100,0.65)",
  Ict: "rgba( 60,120,220,0.65)",
  Custom: "rgba(200,120, 40,0.65)",
};

const COMP_COLORS = [
  "rgba(220, 60, 60, 0.75)", // Y / R
  "rgba( 60,180, 60, 0.75)", // Cb / G / Co / Cg
  "rgba( 60, 60,220, 0.75)", // Cr / B
  "rgba(180,120,220, 0.75)", // 4th component
];

export function JpegXsMctRenderer({
  ctx,
  width,
  height,
  frame,
}: OverlayRendererProps): void {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const mct = (frame as any).mct_info as MctInfo | undefined;

  if (!mct) {
    const msg = "MCT: no data";
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

  const nc = Math.max(1, mct.num_comps);
  const barW = Math.floor(width / nc);
  const bgColor = MCT_TYPE_COLORS[mct.mct_type] ?? MCT_TYPE_COLORS["None"];

  // Full-frame background tint
  ctx.fillStyle = bgColor;
  ctx.fillRect(0, 0, width, height);

  // Per-component vertical strips
  for (let i = 0; i < nc; i++) {
    const x = i * barW;
    const compColor = COMP_COLORS[i] ?? COMP_COLORS[COMP_COLORS.length - 1];
    ctx.fillStyle = compColor;
    ctx.fillRect(x, height * 0.3, barW - 2, height * 0.4);

    // Component label
    const label = mct.comp_labels[i] ?? `C${i}`;
    ctx.fillStyle = "rgba(255,255,255,0.95)";
    ctx.font = `bold ${Math.min(20, barW * 0.3)}px sans-serif`;
    const tw = ctx.measureText(label).width;
    ctx.fillText(label, x + (barW - tw) / 2, height * 0.55);
  }

  // ── Info box ───────────────────────────────────────────────────────────────
  const lines = [
    `MCT: ${mct.mct_type}`,
    `Components: ${nc}`,
    `Reversible: ${mct.reversible ? "Yes" : "No"}`,
  ];
  const boxW = 160;
  const boxH = 16 + lines.length * 16;
  const boxX = 10;
  const boxY = 10;

  ctx.fillStyle = "rgba(0,0,0,0.72)";
  ctx.fillRect(boxX, boxY, boxW, boxH);
  ctx.font = "11px sans-serif";

  lines.forEach((line, i) => {
    ctx.fillStyle = i === 0 ? "#aaddff" : "#ffffff";
    ctx.fillText(line, boxX + 8, boxY + 14 + i * 16);
  });
}
