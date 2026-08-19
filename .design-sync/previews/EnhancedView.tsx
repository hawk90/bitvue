import { EnhancedView } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

// 1x1 transparent PNG -- stand-in thumbnail data URI (EnhancedView's `thumbnails` prop values feed
// straight into `<img src>` inside its child ThumbnailsView, per the task brief).
const STUB_THUMB =
  "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=";

interface PreviewFrame {
  frame_index: number;
  frame_type: string;
  size: number;
  pts: number;
  poc: number;
  key_frame: boolean;
  ref_frames: number[];
}

// GOP boundaries at 0/12/22 (three GOPs); a deliberate size spike across frames 10-16 exercises
// EnhancedView's own scene-change variance heuristic (SCENE_CHANGE_WINDOW_SIZE=5,
// SCENE_CHANGE_VARIANCE_THRESHOLD=0.5) and its >100KB "large-frame" diagnostic; one B-frame with
// no `ref_frames` exercises the "no-reference" diagnostic.
function buildFrames(count = 32): PreviewFrame[] {
  const gopStarts = [0, 12, 22];
  const frames: PreviewFrame[] = [];
  for (let i = 0; i < count; i++) {
    const isGopStart = gopStarts.includes(i);
    const gopBase = [...gopStarts].reverse().find((g) => g <= i) ?? 0;
    const posInGop = i - gopBase;
    let frame_type: string;
    let size: number;
    let ref_frames: number[];

    if (isGopStart) {
      frame_type = "I";
      size = i === 0 ? 172000 : i === 12 ? 149000 : 97000;
      ref_frames = [];
    } else if (posInGop % 3 === 0) {
      frame_type = "P";
      size = 58000 + Math.round(20000 * Math.abs(Math.sin(i * 7.13)));
      ref_frames = [i - 1];
    } else {
      const inSceneSpike = i >= 10 && i <= 16;
      const inflate = inSceneSpike ? 3.4 : 1;
      size = Math.round(
        (21000 + 9000 * Math.abs(Math.sin(i * 3.77))) * inflate,
      );
      frame_type = "B";
      ref_frames =
        i === 15 ? [] : [i - 1, i + 1].filter((r) => r >= 0 && r < count);
    }

    frames.push({
      frame_index: i,
      frame_type,
      size,
      pts: i * 33,
      poc: i,
      key_frame: frame_type === "I",
      ref_frames,
    });
  }
  return frames;
}

const frames = buildFrames(32);

const loadedThumbnails = new Map<number, string>(
  frames.slice(0, 14).map((f) => [f.frame_index, STUB_THUMB]),
);

export const Default = () => (
  <div style={previewBg}>
    <EnhancedView
      frames={frames}
      currentFrameIndex={13}
      thumbnails={loadedThumbnails}
      loadingThumbnails={new Set()}
      onFrameClick={() => {}}
      onHoverFrame={() => {}}
    />
  </div>
);

export const ThumbnailsLoading = () => (
  <div style={previewBg}>
    <EnhancedView
      frames={frames}
      currentFrameIndex={2}
      thumbnails={new Map(frames.slice(0, 4).map((f) => [f.frame_index, STUB_THUMB]))}
      loadingThumbnails={new Set([4, 5, 6, 7, 8])}
      onFrameClick={() => {}}
      onHoverFrame={() => {}}
    />
  </div>
);
