/**
 * Reference Overlay Renderer
 *
 * F7: Shows reference frame relationships with visual graph
 */

import { memo } from "react";
import type { OverlayRendererProps } from "../types";
import { getCssVar } from "../../../../utils/css";
import { drawArrowBetweenPoints } from "../utils/drawing";

export const ReferenceOverlay = memo(function ReferenceOverlay({
  ctx,
  width,
  height,
  frame,
}: OverlayRendererProps) {
  const refFrames = frame.ref_frames || [];
  const frameType = frame.frame_type || "UNKNOWN";
  const isKeyFrame =
    frameType === "I" || frameType === "KEY" || frameType === "INTRA";

  // Graph settings
  const nodeRadius = 20;
  const centerX = width / 2;
  const centerY = height / 2;
  const spacing = Math.min(width, height) / 4;

  // Get frame type colors
  const keyColor = getCssVar("--frame-i") || "#e03131";
  const intraColor = keyColor;
  const interColor = getCssVar("--frame-p") || "#2da44e";
  const currentFrameColor = isKeyFrame ? intraColor : interColor;

  // Draw current frame (center)
  ctx.fillStyle = currentFrameColor + "cc"; // 80% opacity
  ctx.strokeStyle = "#ffffff";
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.arc(centerX, centerY, nodeRadius, 0, Math.PI * 2);
  ctx.fill();
  ctx.stroke();

  // Current frame label
  ctx.fillStyle = "#ffffff";
  ctx.font = "bold 12px sans-serif";
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText(frame.frame_index.toString(), centerX, centerY);

  // Frame type label below current frame
  ctx.font = "11px sans-serif";
  ctx.fillText(frameType, centerX, centerY + nodeRadius + 15);

  // Draw reference frames
  if (refFrames.length > 0) {
    const angleStep = (Math.PI * 2) / refFrames.length;

    refFrames.forEach((refIndex, i) => {
      const angle = angleStep * i - Math.PI / 2; // Start from top
      const refX = centerX + Math.cos(angle) * spacing;
      const refY = centerY + Math.sin(angle) * spacing;

      // Draw arrow from ref to current (flow of dependency)
      drawArrowBetweenPoints(
        ctx,
        refX,
        refY,
        centerX,
        centerY,
        nodeRadius,
        getCssVar("--text-secondary") || "#888888",
      );

      // Draw reference frame node
      ctx.fillStyle = (getCssVar("--frame-p") || "#2da44e") + "cc";
      ctx.strokeStyle = "#ffffff";
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.arc(refX, refY, nodeRadius * 0.85, 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();

      // Reference frame label
      ctx.fillStyle = "#ffffff";
      ctx.font = "bold 11px sans-serif";
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillText(refIndex.toString(), refX, refY);
    });
  }

  // Draw info panel
  const infoX = 10;
  const infoY = 10;
  const infoWidth = 220;
  const infoHeight = isKeyFrame ? 60 : 80;

  ctx.fillStyle = "rgba(0, 0, 0, 0.8)";
  ctx.fillRect(infoX, infoY, infoWidth, infoHeight);
  ctx.strokeStyle = currentFrameColor;
  ctx.lineWidth = 2;
  ctx.strokeRect(infoX, infoY, infoWidth, infoHeight);

  ctx.fillStyle = getCssVar("--text-bright") || "#fff";
  ctx.font = "12px sans-serif";
  ctx.textAlign = "left";
  ctx.textBaseline = "top";

  const textY = infoY + 12;
  const lineHeight = 18;

  ctx.fillText(`Current Frame: ${frame.frame_index}`, infoX + 10, textY);
  ctx.fillText(`Frame Type: ${frameType}`, infoX + 10, textY + lineHeight);

  if (isKeyFrame) {
    ctx.fillStyle = getCssVar("--text-secondary") || "#aaaaaa";
    ctx.fillText(
      "Key frame - no references",
      infoX + 10,
      textY + lineHeight * 2,
    );
  } else {
    ctx.fillText(
      `References: ${refFrames.length} frame${refFrames.length !== 1 ? "s" : ""}`,
      infoX + 10,
      textY + lineHeight * 2,
    );
    if (refFrames.length > 0) {
      ctx.fillStyle = getCssVar("--text-secondary") || "#aaaaaa";
      ctx.fillText(
        `[${refFrames.join(", ")}]`,
        infoX + 10,
        textY + lineHeight * 3,
      );
    }
  }
});
