/**
 * Selection Types
 *
 * Type definitions for the selection system.
 * Extracted from SelectionContext for better modularity.
 */

export type StreamId = 'A' | 'B';

export interface SpatialBlock {
  x: number;
  y: number;
  w: number;
  h: number;
}

export type TemporalSelectionType = 'block' | 'point' | 'range' | 'marker';

export interface TemporalSelection {
  type: TemporalSelectionType;
  frameIndex: number;
  block?: SpatialBlock;
  rangeStart?: number;
  rangeEnd?: number;
}

export interface FrameKey {
  stream: StreamId;
  frameIndex: number;
  pts?: number;
}

export interface UnitKey {
  stream: StreamId;
  unitType: string;
  offset: number;
  size: number;
}

export interface SyntaxNodeId {
  path: string[];
  fieldType?: string;
  offset?: number;
}

export interface BitRange {
  startBit: number;
  endBit: number;
}

export type SelectionPanel =
  | 'syntax'
  | 'hex'
  | 'main'
  | 'timeline'
  | 'filmstrip'
  | 'reference-lists'
  | 'keyboard'
  | 'minimap'
  | 'bookmarks';

export interface SelectionSource {
  panel: SelectionPanel;
  timestamp: number;
}

export interface SelectionState {
  streamId: StreamId;
  temporal: TemporalSelection | null;
  frame: FrameKey | null;
  unit: UnitKey | null;
  syntaxNode: SyntaxNodeId | null;
  bitRange: BitRange | null;
  source: SelectionSource;
}

export interface SelectionChangeEvent {
  selection: SelectionState;
  source: SelectionSource;
}

export interface SelectionContextType {
  selection: SelectionState | null;
  setTemporalSelection: (selection: TemporalSelection, source: SelectionPanel) => void;
  setFrameSelection: (frame: FrameKey, source: SelectionPanel) => void;
  setUnitSelection: (unit: UnitKey, source: SelectionPanel) => void;
  setSyntaxSelection: (node: SyntaxNodeId, source: SelectionPanel) => void;
  setBitRangeSelection: (range: BitRange, source: SelectionPanel) => void;
  clearTemporal: () => void;
  clearAll: () => void;
  subscribe: (callback: (event: SelectionChangeEvent) => void) => () => void;
}

export const DEFAULT_SELECTION: SelectionState = {
  streamId: 'A',
  temporal: null,
  frame: null,
  unit: null,
  syntaxNode: null,
  bitRange: null,
  source: {
    panel: 'timeline',
    timestamp: Date.now(),
  },
};
