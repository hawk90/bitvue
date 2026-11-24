/**
 * Frame Type Badge
 *
 * Single source of truth for the small color-coded I/P/B pill used anywhere a frame's type is
 * shown next to its value (Selection Info, Unit HEX, Syntax Detail). Previously each panel
 * re-implemented this inline with its own `<span>` + ad-hoc class name, and only
 * SelectionInfoPanel's ever included the `frame-type-badge` class that gives the badge its
 * padding/border/background -- the other two rendered a barely-visible sliver (text present, but
 * with no badge sizing around it). One component now backs all three.
 */

import { memo } from "react";
import { frameKindClass } from "../../utils/frameType";
import "./FrameTypeBadge.css";

interface FrameTypeBadgeProps {
  frameType: string;
}

/** What the short labels mean, shown as the badge's tooltip. */
const FRAME_TYPE_TITLES: Record<string, string> = {
  I: "Key frame",
  IO: "Intra-only frame (intra coded, not a random access point)",
  SW: "Switch frame (inter coded, stream switching point)",
  P: "Predicted frame",
  B: "Bi-predicted frame",
};

export const FrameTypeBadge = memo(function FrameTypeBadge({
  frameType,
}: FrameTypeBadgeProps) {
  return (
    <span
      className={`frame-type-badge ${frameKindClass("frame-type", frameType)}`}
      title={FRAME_TYPE_TITLES[frameType.toUpperCase()]}
    >
      {frameType}
    </span>
  );
});
