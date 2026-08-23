/**
 * Player pixel/block hover tooltip (INT-01).
 *
 * Contract: docs/UX_PARITY_MATRIX.md §4 Player surface tooltip row -- frame idx, pixel (x,y),
 * luma/chroma values, block id/partition info, active overlays list. "Pin Sample" is explicitly
 * optional/v2 there and not implemented here; "Copy Pixel" is covered separately by the Player
 * context menu's "Copy Selection" item (copies the *selected*, not merely hovered, block).
 */

import { memo } from "react";
import type { PixelValue } from "../../../utils/pixelValueLookup";
import type { SpatialBlockRect } from "../../../utils/spatialBlockHitTest";
import type { VisualizationMode } from "../../../contexts/ModeContext";

interface PlayerPixelTooltipProps {
  clientX: number;
  clientY: number;
  frameIndex: number;
  pixelX: number;
  pixelY: number;
  pixel: PixelValue | null;
  block: SpatialBlockRect | null;
  activeOverlays?: ReadonlySet<VisualizationMode>;
}

export const PlayerPixelTooltip = memo(function PlayerPixelTooltip({
  clientX,
  clientY,
  frameIndex,
  pixelX,
  pixelY,
  pixel,
  block,
  activeOverlays,
}: PlayerPixelTooltipProps) {
  const overlayList =
    activeOverlays && activeOverlays.size > 0
      ? Array.from(activeOverlays).join(", ")
      : "None";

  return (
    <div
      className="player-pixel-tooltip"
      style={{ left: clientX + 14, top: clientY + 14 }}
    >
      <div className="player-pixel-tooltip-row">
        Frame #{frameIndex} · ({pixelX}, {pixelY})
      </div>
      {pixel && (
        <div className="player-pixel-tooltip-row">
          Y {pixel.y} · U {pixel.u} · V {pixel.v}
        </div>
      )}
      {block && (
        <div className="player-pixel-tooltip-row">
          Block {block.w}×{block.h} @ ({block.x}, {block.y})
        </div>
      )}
      <div className="player-pixel-tooltip-row player-pixel-tooltip-overlays">
        Overlays: {overlayList}
      </div>
    </div>
  );
});
