/**
 * PTS Quality Badge (EDGE-03)
 *
 * Small stream-wide indicator mirroring `bitvue_engine::frame_identity::PtsQuality`'s
 * `badge_text()`/`tooltip()` -- computed once per stream (not per-frame) by
 * `FrameIndexMap::assess_pts_quality`: Bad = duplicate PTS or >50% missing, Warn = some missing
 * or VFR detected, Ok = otherwise. Decode-order non-monotonicity (B-frames) is expected and
 * deliberately NOT a quality issue -- see that Rust doc for the full rule set.
 *
 * Renders nothing for `null` (not yet loaded) or `"Ok"` (nothing to flag) -- only Warn/Bad are
 * worth a visible badge; a permanently-present "OK" pill next to the frame count would just be
 * noise on every stream, which is the overwhelming majority case for real fixtures.
 */

import { memo } from "react";
import "./PtsQualityBadge.css";

interface PtsQualityBadgeProps {
  ptsQuality: "Ok" | "Warn" | "Bad" | null;
}

const BADGE_TEXT: Record<"Warn" | "Bad", string> = {
  Warn: "PTS: WARN",
  Bad: "PTS: BAD",
};

const TOOLTIP: Record<"Warn" | "Bad", string> = {
  Warn: "Some PTS values missing or variable frame rate detected. Timeline uses frame index fallback.",
  Bad: "Major PTS issues detected (duplicates, or >50% missing). Timeline uses frame index.",
};

export const PtsQualityBadge = memo(function PtsQualityBadge({
  ptsQuality,
}: PtsQualityBadgeProps) {
  if (ptsQuality === null || ptsQuality === "Ok") return null;

  return (
    <span
      className={`pts-quality-badge pts-quality-${ptsQuality.toLowerCase()}`}
      title={TOOLTIP[ptsQuality]}
    >
      {BADGE_TEXT[ptsQuality]}
    </span>
  );
});
