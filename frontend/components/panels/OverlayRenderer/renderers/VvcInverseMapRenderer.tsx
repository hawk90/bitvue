/**
 * VVC Inverse Map (LMCS) Renderer
 *
 * F6 mode for VVC: visualizes the LMCS (Luma Mapping with Chroma Scaling)
 * inverse-mapping function applied to each CTU.
 *
 * The LMCS mapping reshapes luma values before coding. The inverse map shows
 * how much each region's luma was shifted by the mapping — bright = large
 * positive shift, dark = large negative shift, neutral grey = no change.
 *
 * Approximation: uses QP deviation from average as a proxy for LMCS delta
 * until a dedicated `lmcs_grid` field is added to FrameInfo.  Blocks with
 * lower QP (high quality) are brightened; higher QP blocks are darkened —
 * matching typical LMCS behavior where low-QP regions receive positive bias.
 *
 * Color map (VQ Analyzer parity):
 *   Large positive delta → warm yellow/red  (#FF8800)
 *   Near zero            → neutral grey     (#888888)
 *   Large negative delta → cool blue/purple (#4444CC)
 */

import type { OverlayRendererProps } from "../types";

/** Interpolate between two RGB colors by t ∈ [0, 1] */
function lerp3(
  r0: number,
  g0: number,
  b0: number,
  r1: number,
  g1: number,
  b1: number,
  t: number,
): [number, number, number] {
  const r = Math.round(r0 + (r1 - r0) * t);
  const g = Math.round(g0 + (g1 - g0) * t);
  const b = Math.round(b0 + (b1 - b0) * t);
  return [r, g, b];
}

/** Map normalized delta [-1..1] to an RGBA string via blue→grey→orange colormap */
function deltaToColor(norm: number): string {
  // norm < 0 → blue/purple; norm > 0 → orange/red
  if (norm >= 0) {
    const t = Math.min(norm, 1);
    const [r, g, b] = lerp3(136, 136, 136, 255, 136, 0, t);
    return `rgba(${r},${g},${b},0.55)`;
  } else {
    const t = Math.min(-norm, 1);
    const [r, g, b] = lerp3(136, 136, 136, 68, 68, 200, t);
    return `rgba(${r},${g},${b},0.55)`;
  }
}

export function VvcInverseMapRenderer({
  ctx,
  width,
  height,
  frame,
}: OverlayRendererProps) {
  const qpGrid = frame.qp_grid;
  if (!qpGrid) {
    ctx.fillStyle = "rgba(0,0,0,0.7)";
    ctx.fillRect(10, 10, 295, 30);
    ctx.fillStyle = "#fff";
    ctx.font = "12px sans-serif";
    ctx.fillText("LMCS data requires QP grid — not available", 20, 30);
    return;
  }

  const { grid_w, grid_h, qp } = qpGrid;

  // Compute average and std dev for normalization
  let sum = 0;
  for (const v of qp) sum += v;
  const avgQp = qp.length > 0 ? sum / qp.length : 32;
  let variance = 0;
  for (const v of qp) variance += (v - avgQp) ** 2;
  const stdDev = Math.sqrt(variance / (qp.length || 1)) || 1;

  const cellW = width / grid_w;
  const cellH = height / grid_h;

  for (let row = 0; row < grid_h; row++) {
    for (let col = 0; col < grid_w; col++) {
      const idx = row * grid_w + col;
      const blockQp = qp[idx] ?? avgQp;
      // LMCS typically brightens low-QP (high quality) blocks
      // Positive norm = block is below average QP = positive luma shift
      const norm = Math.max(-1, Math.min(1, (avgQp - blockQp) / (2 * stdDev)));

      ctx.fillStyle = deltaToColor(norm);
      ctx.fillRect(col * cellW, row * cellH, cellW, cellH);
    }
  }

  // Legend bar
  const barX = 10;
  const barY = height - 24;
  const barW = 180;
  const barH = 10;
  const grad = ctx.createLinearGradient(barX, 0, barX + barW, 0);
  grad.addColorStop(0, "rgba(68,68,200,0.8)");
  grad.addColorStop(0.5, "rgba(136,136,136,0.8)");
  grad.addColorStop(1, "rgba(255,136,0,0.8)");
  ctx.fillStyle = grad;
  ctx.fillRect(barX, barY, barW, barH);

  ctx.fillStyle = "#fff";
  ctx.font = "10px sans-serif";
  ctx.fillText("−Δ luma (dark)", barX, barY - 2);
  ctx.fillText("+Δ luma (light)", barX + barW - 80, barY - 2);

  // Note
  ctx.fillStyle = "rgba(0,0,0,0.55)";
  ctx.fillRect(10, height - 42, 290, 15);
  ctx.fillStyle = "#ffdd88";
  ctx.fillText("⚠ LMCS inverse map approximation (QP proxy)", 14, height - 31);
}
