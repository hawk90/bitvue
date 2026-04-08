/**
 * Stream Player - Single stream player for compare view
 *
 * Displays a single video stream with frame navigation.
 */

import { memo, useMemo, useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  type FrameInfo,
  type YUVFrameData,
  AlignmentQuality,
} from "../../types/video";
import { VideoCanvas } from "../panels/YuvViewerPanel/VideoCanvas";
import { FrameNavigationControls } from "../panels/YuvViewerPanel/FrameNavigationControls";
import type { YUVFrame } from "../../types/yuv";
import "./StreamPlayer.css";

interface StreamPlayerProps {
  frames: FrameInfo[];
  currentFrame: number;
  onFrameChange: (index: number) => void;
  streamLabel: "A" | "B";
  alignedFrame?: number | null;
  alignmentQuality?: AlignmentQuality;
}

function StreamPlayer({
  frames,
  currentFrame,
  onFrameChange,
  streamLabel,
  alignedFrame: _alignedFrame,
  alignmentQuality,
}: StreamPlayerProps) {
  const currentFrameData = frames[currentFrame] || null;
  const [yuvFrame, setYuvFrame] = useState<YUVFrame | null>(null);

  useEffect(() => {
    if (!currentFrameData) {
      setYuvFrame(null);
      return;
    }
    let cancelled = false;
    invoke<YUVFrameData>("get_decoded_frame_yuv", {
      frameIndex: currentFrame,
      streamId: streamLabel,
    })
      .then((data) => {
        if (cancelled || !data.success || !data.y_plane) return;
        const b64 = (s: string) => {
          const bin = atob(s);
          const arr = new Uint8Array(bin.length);
          for (let i = 0; i < bin.length; i++) arr[i] = bin.charCodeAt(i);
          return arr;
        };
        setYuvFrame({
          width: data.width,
          height: data.height,
          y: b64(data.y_plane),
          u: data.u_plane ? b64(data.u_plane) : new Uint8Array(0),
          v: data.v_plane ? b64(data.v_plane) : new Uint8Array(0),
          yStride: data.y_stride,
          uStride: data.u_stride,
          vStride: data.v_stride,
          chromaSubsampling: "420",
        });
      })
      .catch(() => setYuvFrame(null));
    return () => {
      cancelled = true;
    };
  }, [currentFrame, currentFrameData, streamLabel]);

  // Memoize alignment color function - it's recreated on every render otherwise
  const getAlignmentColor = useMemo(
    () =>
      (quality?: AlignmentQuality): string => {
        switch (quality) {
          case AlignmentQuality.Exact:
            return "var(--color-success)";
          case AlignmentQuality.Nearest:
            return "var(--color-warning)";
          case AlignmentQuality.Gap:
            return "var(--color-error)";
          default:
            return "var(--color-text-secondary)";
        }
      },
    [],
  );

  return (
    <div className={`stream-player stream-${streamLabel.toLowerCase()}`}>
      {/* Frame display */}
      <div className="player-viewport">
        {currentFrameData ? (
          <VideoCanvas
            width={currentFrameData.width || 1920}
            height={currentFrameData.height || 1080}
            frameData={currentFrameData}
            yuvData={yuvFrame ?? undefined}
          />
        ) : (
          <div className="player-placeholder">
            <span>No frame data</span>
          </div>
        )}

        {/* Alignment indicator for stream B */}
        {streamLabel === "B" && alignmentQuality !== undefined && (
          <div
            className="alignment-indicator"
            style={{ borderColor: getAlignmentColor(alignmentQuality) }}
            title={`Alignment: ${alignmentQuality}`}
          >
            <span
              className="alignment-dot"
              style={{ backgroundColor: getAlignmentColor(alignmentQuality) }}
            />
            {alignmentQuality}
          </div>
        )}

        {/* Frame info overlay */}
        {currentFrameData && (
          <div className="frame-overlay">
            <span className="frame-number">
              {streamLabel}: {currentFrame + 1} / {frames.length}
            </span>
            {currentFrameData.frame_type && (
              <span
                className={`frame-type frame-${currentFrameData.frame_type.toLowerCase()}`}
              >
                {currentFrameData.frame_type}
              </span>
            )}
            {currentFrameData.size && (
              <span className="frame-size">
                {(currentFrameData.size / 1024).toFixed(1)} KB
              </span>
            )}
          </div>
        )}
      </div>

      {/* Frame navigation */}
      <div className="player-controls">
        <FrameNavigationControls
          currentFrame={currentFrame}
          totalFrames={frames.length}
          onFrameChange={onFrameChange}
          compact
        />
      </div>
    </div>
  );
}

export default memo(StreamPlayer);
