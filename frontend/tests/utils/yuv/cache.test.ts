/**
 * Regression test for a real cross-frame cache collision bug (found 2026-08-19 while
 * investigating a user report of a black/wrong screen for stream B in the new dual-stream
 * compare view -- docs/DEVELOPMENT_PHASES.md Phase 7.5). `createYUVCacheKey` used to derive its
 * key ONLY from width/height/chromaSubsampling/colorspace/plane lengths -- never from actual
 * pixel content. Since `yuvCache` is a single global singleton shared by every `YUVRenderer`
 * instance in the app (`frontend/utils/yuv/renderer.ts`), any two frames sharing those
 * properties -- the *common* case, not an edge case: every frame within one video shares them,
 * and two different streams at the same resolution/format (the usual dual-stream compare case)
 * do too -- silently rendered whichever content got cached first, regardless of which frame was
 * actually requested.
 */
import { describe, it, expect } from "vitest";
import { yuvToImageData } from "../../../utils/yuv/renderer";
import { YUVCache } from "../../../utils/yuv/cache";
import { Colorspace } from "../../../types/yuv";
import type { YUVFrame } from "../../../types/yuv";

function makeFrame(fill: number): YUVFrame {
  const w = 4,
    h = 4;
  return {
    width: w,
    height: h,
    yStride: w,
    uStride: w / 2,
    vStride: w / 2,
    chromaSubsampling: "420",
    y: new Uint8Array(w * h).fill(fill),
    u: new Uint8Array((w / 2) * (h / 2)).fill(128),
    v: new Uint8Array((w / 2) * (h / 2)).fill(128),
  };
}

describe("YUV conversion cache", () => {
  it("does not reuse another frame's cached pixels just because dims/format match", () => {
    YUVCache.clear();
    const dark = makeFrame(16);
    const bright = makeFrame(235);

    const imgDark = yuvToImageData(dark, Colorspace.BT709);
    const imgBright = yuvToImageData(bright, Colorspace.BT709);

    // Same dims/format/plane-lengths (both 4x4 420), genuinely different luma content -- must
    // produce genuinely different RGB output, not a stale cache hit from the first call.
    expect(imgBright.data[0]).not.toBe(imgDark.data[0]);
  });

  it("still reuses the cache for the exact same frame object (the cache's real purpose)", () => {
    YUVCache.clear();
    const frame = makeFrame(100);

    const first = yuvToImageData(frame, Colorspace.BT709);
    const second = yuvToImageData(frame, Colorspace.BT709);

    // Same object reference back out of the cache -- confirms the fix didn't just disable
    // caching altogether (that would "fix" the bug but defeat the cache's purpose).
    expect(second).toBe(first);
  });

  it("distinguishes frames whose bulk content matches but differ only in a sampled region", () => {
    YUVCache.clear();
    const base = makeFrame(100);
    const modified = makeFrame(100);
    // Flip a handful of scattered bytes -- content sampling must still catch this, not just
    // full-content differences like the dark/bright case above.
    modified.y[1] = 250;
    modified.y[7] = 250;
    modified.y[13] = 250;

    const imgBase = yuvToImageData(base, Colorspace.BT709);
    const imgModified = yuvToImageData(modified, Colorspace.BT709);

    expect(imgModified.data).not.toEqual(imgBase.data);
  });
});
