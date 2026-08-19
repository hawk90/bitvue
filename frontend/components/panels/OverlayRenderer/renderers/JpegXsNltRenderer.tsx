/**
 * JPEG XS NLT (Non-Linear Transform) Overlay Renderer
 *
 * F5 (JPEG XS): Shows the tone-mapping stages applied before wavelet coding.
 * When NLT is present, renders a per-component curve visualisation and type
 * label. When absent, shows a "NLT: not present" notice.
 *
 * NLT types:
 *   QUA   (Quadratic)          — low-complexity perceptual
 *   XQUAD (Extended Quadratic) — extended precision
 *   XEXP  (Extended Exponential) — HDR/WCG
 */

import type { OverlayRendererProps } from "../types";

interface NltInfo {
  present: boolean;
  num_comps: number;
  comp_types: string[];
}

const NLT_BG_COLOR = "rgba(50,30,70,0.60)";

function drawCurve(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  type: string,
  color: string,
): void {
  ctx.strokeStyle = color;
  ctx.lineWidth = 1.5;
  ctx.beginPath();

  const steps = 64;
  for (let i = 0; i <= steps; i++) {
    const t = i / steps;
    let out: number;
    if (type.includes("XEXP")) {
      // Exponential curve
      out = Math.pow(t, 0.45);
    } else {
      // Quadratic approximation (piecewise)
      out = t < 0.5 ? 2 * t * t : 1 - 2 * (1 - t) * (1 - t);
    }
    const px = x + t * w;
    const py = y + h - out * h;
    if (i === 0) ctx.moveTo(px, py);
    else ctx.lineTo(px, py);
  }
  ctx.stroke();

  // Linear reference
  ctx.strokeStyle = "rgba(255,255,255,0.20)";
  ctx.lineWidth = 0.5;
  ctx.setLineDash([3, 3]);
  ctx.beginPath();
  ctx.moveTo(x, y + h);
  ctx.lineTo(x + w, y);
  ctx.stroke();
  ctx.setLineDash([]);
}

const COMP_COLORS_NLT = ["#ff6060", "#60d060", "#6080ff", "#c080ff"];

export function JpegXsNltRenderer({
  ctx,
  width,
  height,
  frame,
}: OverlayRendererProps): void {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const nlt = (frame as any).nlt_info as NltInfo | undefined;

  if (!nlt?.present) {
    const msg = "NLT: not present";
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

  const nc = Math.max(1, nlt.num_comps);
  const margin = 30;
  const curveW = Math.min(100, (width - margin * 2) / nc - 10);
  const curveH = curveW;
  const startY = (height - curveH) / 2;

  // Background tint
  ctx.fillStyle = NLT_BG_COLOR;
  ctx.fillRect(0, 0, width, height);

  // Per-component curve
  const totalW = nc * (curveW + 10);
  const startX = (width - totalW) / 2;

  for (let i = 0; i < nc; i++) {
    const x = startX + i * (curveW + 10);
    const color =
      COMP_COLORS_NLT[i] ?? COMP_COLORS_NLT[COMP_COLORS_NLT.length - 1];
    const type = nlt.comp_types[i] ?? "QUA";

    // Background box
    ctx.fillStyle = "rgba(0,0,0,0.45)";
    ctx.fillRect(x - 4, startY - 4, curveW + 8, curveH + 8);

    drawCurve(ctx, x, startY, curveW, curveH, type, color);

    // Axis labels
    ctx.fillStyle = "rgba(255,255,255,0.60)";
    ctx.font = "8px sans-serif";
    ctx.fillText("0", x - 8, startY + curveH + 2);
    ctx.fillText("1", x + curveW, startY + curveH + 2);

    // Component label
    ctx.fillStyle = color;
    ctx.font = "10px sans-serif";
    ctx.fillText(`C${i}`, x + curveW / 2 - 8, startY - 8);
  }

  // ── Info box ───────────────────────────────────────────────────────────────
  const boxX = 10;
  const boxY = 10;
  const boxW = 150;
  const boxH = 30 + nc * 14;

  ctx.fillStyle = "rgba(0,0,0,0.72)";
  ctx.fillRect(boxX, boxY, boxW, boxH);
  ctx.font = "11px sans-serif";
  ctx.fillStyle = "#aaddff";
  ctx.fillText("NLT: present", boxX + 8, boxY + 14);

  nlt.comp_types.forEach((t, i) => {
    ctx.fillStyle = COMP_COLORS_NLT[i] ?? "#ffffff";
    ctx.fillText(`C${i}: ${t}`, boxX + 8, boxY + 28 + i * 14);
  });
}
