/**
 * Frame decode domain — real decoded YUV pixel planes for one frame. See
 * `services/bridge/core.ts`'s module doc for how this file fits into the overall bridge split.
 */

import { requireBridge, type StreamId } from "./core";
import type { YUVFrame } from "../../types/yuv";

/** Mirrors `get_decoded_frame_yuv`'s Control-frame metadata (`bitvue-sidecar`'s `decode_bridge`
 *  module) -- the `Data` frame that follows is the concatenated Y+U+V raw bytes, sliced using
 *  `y_len`/`u_len`/`v_len` below. Same no-base64, no-JSON-array approach as `getHexRange`. Also
 *  the return shape of `compareDiff.ts`'s `getDebugYuvFrame` -- decoded/reference/diff/amplified
 *  pixel data is the same wire shape regardless of source. */
export interface BridgeDecodedYuvFrame {
  width: number;
  height: number;
  bitDepth: number;
  chromaSubsampling: "420" | "422" | "444";
  yStride: number;
  uStride: number;
  vStride: number;
  yLen: number;
  uLen: number;
  vLen: number;
  bytes: Uint8Array;
}

/** Decodes (dav1d, via `bitvue-decode`) the stream from its start up to and including
 *  `frameIndex`, returning that frame's real YUV planes. AV1/IVF only, no session caching yet --
 *  see `bitvue-sidecar`'s `decode_bridge` module doc for the current O(frameIndex) perf caveat. */
export async function getDecodedFrameYuv(
  stream: StreamId,
  frameIndex: number,
): Promise<BridgeDecodedYuvFrame> {
  return requireBridge().getDecodedFrameYuv(stream, frameIndex);
}

/** Slices a `BridgeDecodedYuvFrame`'s single concatenated `bytes` buffer into the
 *  y/u/v `Uint8Array` views `YUVFrame` (and thus `VideoCanvas`) expects. */
export function bridgeYuvToFrame(frame: BridgeDecodedYuvFrame): YUVFrame {
  const { bytes, yLen, uLen, vLen } = frame;
  return {
    y: bytes.subarray(0, yLen),
    u: uLen > 0 ? bytes.subarray(yLen, yLen + uLen) : new Uint8Array(0),
    v:
      vLen > 0
        ? bytes.subarray(yLen + uLen, yLen + uLen + vLen)
        : new Uint8Array(0),
    width: frame.width,
    height: frame.height,
    yStride: frame.yStride,
    uStride: frame.uStride,
    vStride: frame.vStride,
    chromaSubsampling: frame.chromaSubsampling,
  };
}
