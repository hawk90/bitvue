/**
 * Color Utilities for Overlay Rendering
 *
 * Color palettes and interpolation functions for visualization modes
 */

import type { ColorStop } from "../types";
import { getCssVar } from "../../../../utils/css";

/**
 * 4-stop RGB ramp for QP heatmap
 * Blue → Cyan → Yellow → Red
 */
export const QP_COLOR_STOPS: ColorStop[] = [
  { t: 0.0, r: 0, g: 70, b: 255 }, // Blue
  { t: 0.35, r: 0, g: 200, b: 180 }, // Cyan
  { t: 0.65, r: 255, g: 190, b: 0 }, // Yellow
  { t: 1.0, r: 255, g: 40, b: 40 }, // Red
];

/**
 * Prediction mode color palette
 * Based on AV1 prediction modes from bitvue_av1::tile::coding_unit::PredictionMode
 *
 * INTRA modes (0-13): Blue/Purple tones
 * INTER modes (14+): Green/Yellow tones
 */
export const PREDICTION_MODE_COLORS: Record<number, string> = {
  // INTRA modes
  0: "#4a90d9", // DcPred - DC prediction
  1: "#5a9fe8", // VPred - Vertical prediction
  2: "#6aaff7", // HPred - Horizontal prediction
  3: "#7abfd0", // D45Pred - Diagonal 45°
  4: "#8acff9", // D135Pred - Diagonal 135°
  5: "#9adf08", // D113Pred - Diagonal 113°
  6: "#aaef17", // D157Pred - Diagonal 157°
  7: "#baff26", // D203Pred - Diagonal 203°
  8: "#caff35", // SmoothPred - Smooth prediction
  9: "#daff44", // SmoothV - Vertical smooth
  10: "#eaff53", // SmoothH - Horizontal smooth
  11: "#faff62", // PaethPred - Paeth predictor
  12: "#0a9f71", // PredCFL - Chroma from Luma
  13: "#1aaf80", // Other INTRA modes

  // INTER modes
  64: "#2da44e", // NewMv - New motion vector
  65: "#3db45d", // NearestMv - Nearest reference
  66: "#4dc46c", // NearMv - Near reference
  67: "#5dd47b", // ZeroMv - Zero motion vector
  68: "#6de48a", // GlobalMv - Global motion
  69: "#7df499", // Compound modes
};

/**
 * Transform size color palette
 * Based on AV1 transform sizes (4x4 to 64x64)
 */
export const TX_SIZE_COLORS: Record<number, string> = {
  0: "#ffb86c", // Tx4x4 - Orange
  1: "#75beff", // Tx8x8 - Blue
  2: "#89d185", // Tx16x16 - Green
  3: "#007acc", // Tx32x32 - Dark blue
  4: "#e03131", // Tx64x64 - Red
};

/**
 * Get frame type color from CSS variables
 * Note: Canvas rendering needs hex fallbacks since CSS variables don't resolve in canvas
 */
export function getFrameTypeColor(frameType: string): string {
  switch (frameType) {
    case "I":
    case "KEY":
      return getCssVar("--frame-i") || "#e03131";
    case "P":
      return getCssVar("--frame-p") || "#2da44e";
    case "B":
      return getCssVar("--frame-b") || "#1f7ad9";
    default:
      return getCssVar("--text-secondary") || "#cccccc";
  }
}

/**
 * Get prediction mode name for display
 */
export function getPredictionModeName(mode: number): string {
  // INTRA modes
  if (mode <= 13) {
    const intraNames = [
      "DC",
      "V",
      "H",
      "D45",
      "D135",
      "D113",
      "D157",
      "D203",
      "Smooth",
      "SmoothV",
      "SmoothH",
      "Paeth",
      "CFL",
      "Intra",
    ];
    return intraNames[mode] || `Intra${mode}`;
  }
  // INTER modes
  if (mode === 64) return "NewMV";
  if (mode === 65) return "NearMV";
  if (mode === 66) return "Near";
  if (mode === 67) return "ZeroMV";
  if (mode === 68) return "Global";
  return `Inter${mode}`;
}

/**
 * Get transform size name for display
 */
export function getTxSizeName(txSize: number): string {
  const names = ["4x4", "8x8", "16x16", "32x32", "64x64"];
  return names[txSize] || `Tx${txSize}`;
}

/**
 * Get transform size in pixels from tx value
 */
export function getTxSizePixels(txSize: number): number {
  const sizes = [4, 8, 16, 32, 64];
  return sizes[txSize] || 16;
}
