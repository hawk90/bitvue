/**
 * Whole-stream frame list for the graph panels (Bitrate Graph, Reference Graph). They draw every
 * frame of the open stream at once, while the sidecar only pages (`get_frames_chunk`), so this
 * walks the chunks and maps each `UnitNode` to the small shape those panels read.
 */

import { getFramesChunk, type BridgeUnitNode } from "./stream";
import type { StreamId } from "./core";

/** What the graph panels read of one frame. `null` (not `undefined`) for "the sidecar did not say",
 *  because the panels test `!== null`. */
export interface GraphFrame {
  frame_index: number;
  frame_type: string;
  size: number;
  pts: number | null;
  ref_frames: number[] | null;
  temporal_id: number | null;
}

/** Units per `get_frames_chunk` call. Larger than the 100 `FileStateContext` uses (it flushes
 *  state per chunk for progressive UI; here nothing is shown until the whole list is in), which
 *  cuts the round trips of a long stream by 5x. */
export const GRAPH_FRAMES_CHUNK_SIZE = 500;

/** A unit without a `frame_index` is not a frame (nothing to plot); `frame_type` is `"?"` when the
 *  indexer could not tell, same as `unitNodeToFrameInfo`. */
export function unitNodeToGraphFrame(unit: BridgeUnitNode): GraphFrame | null {
  if (unit.frame_index === null) return null;
  return {
    frame_index: unit.frame_index,
    frame_type: unit.frame_type ?? "?",
    size: unit.size,
    pts: unit.pts,
    ref_frames: unit.ref_frames,
    temporal_id: unit.temporal_id,
  };
}

/**
 * Every frame of `stream`, in decode order. Empty (not an error) before the stream is indexed or
 * when the codec/container is one the indexer does not handle (`indexed: false`). A failed chunk
 * rejects. `signal` stops the walk between chunks.
 */
export async function getAllGraphFrames(
  stream: StreamId,
  signal?: AbortSignal,
): Promise<GraphFrame[]> {
  const frames: GraphFrame[] = [];
  let offset = 0;
  let total = 0;
  do {
    if (signal?.aborted) break;
    const chunk = await getFramesChunk(stream, offset, GRAPH_FRAMES_CHUNK_SIZE);
    if (chunk.units.length === 0) break;
    for (const unit of chunk.units) {
      const frame = unitNodeToGraphFrame(unit);
      if (frame) frames.push(frame);
    }
    offset += chunk.units.length;
    total = chunk.total_count;
  } while (offset < total);
  return frames;
}
