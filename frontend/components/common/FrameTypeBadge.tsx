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
import "./FrameTypeBadge.css";

interface FrameTypeBadgeProps {
  frameType: string;
}

export const FrameTypeBadge = memo(function FrameTypeBadge({
  frameType,
}: FrameTypeBadgeProps) {
  return (
    <span className={`frame-type-badge frame-type-${frameType.toLowerCase()}`}>
      {frameType}
    </span>
  );
});
