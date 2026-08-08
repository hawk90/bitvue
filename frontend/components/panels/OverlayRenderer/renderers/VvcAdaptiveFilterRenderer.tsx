/**
 * VVC Adaptive Filter (ALF) Renderer
 *
 * F9 mode for VVC: shows which CTU-sized blocks have ALF applied.
 *
 * Color scheme (VQ Analyzer parity):
 *   ALF applied:     blue (#4488FF, 40% opacity fill)
 *   ALF not applied: grey (#888888, 15% opacity fill)
 *
 * ALF decision per CTU is derived from the QP grid — blocks with lower QP
 * relative to the frame average are more likely to have ALF applied (high
 * quality blocks benefit most). This is a heuristic approximation until
 * full per-CTU ALF flag extraction is implemented.
 *
 * When full ALF flag data is available via a dedicated `alf_grid` field on
 * FrameInfo, this renderer will use that directly.
 */

import type { OverlayRendererProps } from "../types";

export function VvcAdaptiveFilterRenderer({
  ctx,
  width,
  height,
  frame,
}: OverlayRendererProps) {
  const qpGrid = frame.qp_grid;
  if (!qpGrid) {
    ctx.fillStyle = "rgba(0,0,0,0.7)";
    ctx.fillRect(10, 10, 310, 30);
    ctx.fillStyle = "#fff";
    ctx.font = "12px sans-serif";
    ctx.fillText(
      "ALF data requires QP grid — not available for this frame",
      20,
      30,
    );
    return;
  }

  const { grid_w, grid_h, qp } = qpGrid;

  // Compute average QP for threshold
  let sum = 0;
  for (const v of qp) sum += v;
  const avgQp = qp.length > 0 ? sum / qp.length : 32;

  const cellW = width / grid_w;
  const cellH = height / grid_h;

  for (let row = 0; row < grid_h; row++) {
    for (let col = 0; col < grid_w; col++) {
      const idx = row * grid_w + col;
      const blockQp = qp[idx] ?? avgQp;
      // Heuristic: blocks with QP below average benefit from ALF
      const alfApplied = blockQp <= avgQp;

      const px = col * cellW;
      const py = row * cellH;

      if (alfApplied) {
        ctx.fillStyle = "rgba(68, 136, 255, 0.40)";
        ctx.strokeStyle = "rgba(68, 136, 255, 0.70)";
      } else {
        ctx.fillStyle = "rgba(136, 136, 136, 0.15)";
        ctx.strokeStyle = "rgba(136, 136, 136, 0.40)";
      }

      ctx.fillRect(px, py, cellW, cellH);
      ctx.lineWidth = 0.5;
      ctx.strokeRect(px, py, cellW, cellH);
    }
  }

  // Legend
  const legendY = 12;
  ctx.font = "11px sans-serif";
  const items: Array<{ color: string; label: string }> = [
    { color: "rgba(68,136,255,0.7)", label: "ALF applied (est.)" },
    { color: "rgba(136,136,136,0.5)", label: "No ALF" },
  ];
  let lx = 10;
  for (const item of items) {
    ctx.fillStyle = "rgba(0,0,0,0.6)";
    ctx.fillRect(lx, legendY, 10, 10);
    ctx.fillStyle = item.color;
    ctx.fillRect(lx, legendY, 10, 10);
    ctx.fillStyle = "#fff";
    ctx.fillText(item.label, lx + 14, legendY + 9);
    lx += 130;
  }

  // Note about estimation
  ctx.fillStyle = "rgba(0,0,0,0.55)";
  ctx.fillRect(10, height - 26, 295, 18);
  ctx.fillStyle = "#ffdd88";
  ctx.font = "10px sans-serif";
  ctx.fillText(
    "⚠ ALF per-CTU estimation (full parsing pending)",
    14,
    height - 13,
  );
}
