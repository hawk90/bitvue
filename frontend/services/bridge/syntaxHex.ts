/**
 * Syntax/hex domain — raw byte ranges, lazy per-unit syntax trees, and the four structural
 * (multi-sync) selection commands. See `services/bridge/core.ts`'s module doc for how this file
 * fits into the overall bridge split.
 */

import { requireBridge, type StreamId, type BridgeEvent } from "./core";

/** Mirrors `bitvue-sidecar`'s `syntax_node_to_json` -- a nested tree built from
 *  `bitvue_engine::SyntaxModel`'s flat node map. `value` is a plain string (or null for
 *  container/non-leaf fields), not a discriminated union -- unlike `bitvue_engine::UnitNode`,
 *  `SyntaxNode`'s `value` field is already just a display string in the Rust type. */
export interface BridgeSyntaxNode {
  type: string;
  name: string;
  value: string | null;
  bit_range: { start_bit: number; end_bit: number };
  children: BridgeSyntaxNode[];
}

export async function getHexRange(
  stream: StreamId,
  offset: number,
  len: number,
): Promise<{ offset: number; len: number; bytes: Uint8Array }> {
  return requireBridge().getHexRange(stream, offset, len);
}

/** Lazy, per-unit syntax tree (AV1 only so far -- see `bitvue-indexer`'s module doc). Unlike
 *  `getStreamInfo`/`getFramesChunk`, this throws on failure (no unit for that frame_index, index
 *  hasn't run yet, wrong codec) rather than returning an `{indexed: false}`-style payload -- the
 *  sidecar reports it as a real wire error since there's no meaningful partial syntax tree. */
export async function getFrameSyntax(
  stream: StreamId,
  frameIndex: number,
): Promise<BridgeSyntaxNode> {
  return requireBridge().getFrameSyntax(stream, frameIndex);
}

// -- Structural (multi-sync) selection ---------------------------------------------------------
//
// These four map onto bitvue_engine::Command's "Tri-sync" selection variants -- independent of
// selectFrame's temporal cursor (a unit/syntax-node/bit-range/spatial-block selection can exist
// without a frame selection, and vice versa). No frontend component calls these yet as of
// 2026-08-08 -- they're wired here because the sidecar-side capability is real and tested
// (bitvue-sidecar's select_unit/select_syntax/select_bit_range/select_spatial_block, wired
// earlier this migration), not because a specific UI interaction needs them right now. Don't
// treat their existence as proof any cross-view sync feature is wired up end to end.

export async function selectUnit(
  stream: StreamId,
  unitType: string,
  offset: number,
  size: number,
): Promise<BridgeEvent[]> {
  const { events } = await requireBridge().selectUnit(
    stream,
    unitType,
    offset,
    size,
  );
  return events;
}

export async function selectSyntax(
  stream: StreamId,
  nodeId: string,
  startBit: number,
  endBit: number,
): Promise<BridgeEvent[]> {
  const { events } = await requireBridge().selectSyntax(
    stream,
    nodeId,
    startBit,
    endBit,
  );
  return events;
}

export async function selectBitRange(
  stream: StreamId,
  startBit: number,
  endBit: number,
): Promise<BridgeEvent[]> {
  const { events } = await requireBridge().selectBitRange(
    stream,
    startBit,
    endBit,
  );
  return events;
}

export async function selectSpatialBlock(
  stream: StreamId,
  x: number,
  y: number,
  w: number,
  h: number,
): Promise<BridgeEvent[]> {
  const { events } = await requireBridge().selectSpatialBlock(
    stream,
    x,
    y,
    w,
    h,
  );
  return events;
}
