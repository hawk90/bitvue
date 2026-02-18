/**
 * Reference Lines Overlay Component
 *
 * SVG overlay showing reference frame dependencies
 */

import { memo } from "react";

interface ExpansionInfo {
  from: number;
  fromPos: { x: number; y: number };
  to: number;
  toPos: { x: number; y: number };
  color: string;
  arrowIndex: number;
  arrowTotal: number;
}

interface ReferenceLinesOverlayProps {
  expansionInfo: ExpansionInfo[];
  containerRef: React.RefObject<HTMLDivElement>;
}

export const ReferenceLinesOverlay = memo(function ReferenceLinesOverlay({
  expansionInfo,
  containerRef,
}: ReferenceLinesOverlayProps) {
  return (
    <svg
      className="reference-lines-overlay"
      style={{
        position: "absolute",
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        pointerEvents: "none",
        zIndex: 100,
      }}
    >
      {expansionInfo.map((info, idx) => {
        const targetEl = containerRef.current?.querySelector(
          `[data-frame-index="${info.to}"]`,
        ) as HTMLElement;
        let targetWidth = 100;
        if (targetEl) {
          targetWidth = targetEl.getBoundingClientRect().width;
        }

        const fromX = info.fromPos.x;
        const fromY = info.fromPos.y;

        const segmentWidth = targetWidth / (info.arrowTotal + 1);
        const toX =
          info.toPos.x - targetWidth / 2 + segmentWidth * (info.arrowIndex + 1);
        const toY = info.toPos.y;

        const arrowSize = 6;
        const arrowWidth = 6;
        const horizontalDistance = Math.abs(toX - fromX);
        const verticalDrop = Math.min(
          15 + Math.floor(horizontalDistance / 10),
          50,
        );

        return (
          <g key={`${info.from}-${info.to}-${idx}`}>
            <path
              d={`M ${fromX} ${fromY} L ${fromX} ${fromY + verticalDrop} L ${toX} ${fromY + verticalDrop} L ${toX} ${toY}`}
              fill="none"
              stroke={info.color}
              strokeWidth="1.5"
              strokeOpacity="0.6"
            />

            <path
              d={`M ${toX} ${toY} L ${toX - arrowWidth} ${toY + arrowSize} L ${toX + arrowWidth} ${toY + arrowSize} Z`}
              fill={info.color}
              fillOpacity="0.9"
              stroke="#ffffff"
              strokeWidth="1"
              strokeOpacity="0.3"
            />

            <circle
              cx={toX}
              cy={toY}
              r="4"
              fill={info.color}
              fillOpacity="0.15"
              stroke={info.color}
              strokeWidth="2"
              strokeOpacity="0.8"
            />

            <circle
              cx={toX}
              cy={toY}
              r="1.5"
              fill={info.color}
              fillOpacity="1"
            />
          </g>
        );
      })}
    </svg>
  );
});
