/**
 * Tri-Sync Selection Context
 *
 * VQAnalyzer's core navigation system:
 * - Single source of truth for selection state
 * - Broadcasts selection changes to all panels
 * - Synchronizes: Syntax Tree ↔ Hex View ↔ Main Panel ↔ Timeline
 */

import { createContext, useContext, useState, useCallback, useEffect, ReactNode } from 'react';

// ════════════════════════════════════════════════════════════════════════════════
// Selection Types
// ════════════════════════════════════════════════════════════════════════════════

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

export interface SelectionSource {
  panel: 'syntax' | 'hex' | 'main' | 'timeline' | 'filmstrip' | 'reference-lists' | 'keyboard' | 'minimap' | 'bookmarks';
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

// ════════════════════════════════════════════════════════════════════════════════
// Selection Context
// ════════════════════════════════════════════════════════════════════════════════

interface SelectionContextType {
  selection: SelectionState | null;
  setTemporalSelection: (selection: TemporalSelection, source: SelectionSource['panel']) => void;
  setFrameSelection: (frame: FrameKey, source: SelectionSource['panel']) => void;
  setUnitSelection: (unit: UnitKey, source: SelectionSource['panel']) => void;
  setSyntaxSelection: (node: SyntaxNodeId, source: SelectionSource['panel']) => void;
  setBitRangeSelection: (range: BitRange, source: SelectionSource['panel']) => void;
  clearTemporal: () => void;
  clearAll: () => void;
  subscribe: (callback: (event: SelectionChangeEvent) => void) => () => void;
}

const SelectionContext = createContext<SelectionContextType | null>(null);

// ════════════════════════════════════════════════════════════════════════════════
// Provider
// ════════════════════════════════════════════════════════════════════════════════

const DEFAULT_SELECTION: SelectionState = {
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

interface SelectionProviderProps {
  children: ReactNode;
}

export function SelectionProvider({ children }: SelectionProviderProps) {
  const [selection, setSelection] = useState<SelectionState | null>(null);
  const [listeners] = useState<Set<(event: SelectionChangeEvent) => void>>(new Set());

  const notifyListeners = useCallback((newSelection: SelectionState) => {
    const event: SelectionChangeEvent = {
      selection: newSelection,
      source: newSelection.source,
    };
    listeners.forEach(callback => callback(event));
  }, [listeners]);

  const updateSelection = useCallback((updates: Partial<SelectionState>, sourcePanel: SelectionSource['panel']) => {
    setSelection(prev => {
      const newSelection: SelectionState = {
        ...(prev || DEFAULT_SELECTION),
        ...updates,
        source: {
          panel: sourcePanel,
          timestamp: Date.now(),
        },
      };

      // Tri-Sync: Update related selections based on authority
      const syncedSelection = applyTriSyncRules(newSelection);

      notifyListeners(syncedSelection);
      return syncedSelection;
    });
  }, [notifyListeners]);

  // Apply Tri-Sync propagation rules
  const applyTriSyncRules = useCallback((sel: SelectionState): SelectionState => {
    // Rule 1: Temporal selection → Frame selection
    if (sel.temporal && !sel.frame) {
      return {
        ...sel,
        frame: {
          stream: sel.streamId,
          frameIndex: sel.temporal.frameIndex,
        },
      };
    }

    // Rule 2: Unit selection → BitRange selection
    if (sel.unit && !sel.bitRange) {
      return {
        ...sel,
        bitRange: {
          startBit: sel.unit.offset * 8,
          endBit: (sel.unit.offset + sel.unit.size) * 8,
        },
      };
    }

    // Rule 3: SyntaxNode selection → BitRange selection
    if (sel.syntaxNode?.offset !== undefined && !sel.bitRange) {
      const size = sel.syntaxNode.fieldType ? estimateBitSize(sel.syntaxNode.fieldType) : 32;
      return {
        ...sel,
        bitRange: {
          startBit: sel.syntaxNode.offset,
          endBit: sel.syntaxNode.offset + size,
        },
      };
    }

    return sel;
  }, []);

  const setTemporalSelection = useCallback((temporal: TemporalSelection, source: SelectionSource['panel']) => {
    updateSelection({ temporal }, source);
  }, [updateSelection]);

  const setFrameSelection = useCallback((frame: FrameKey, source: SelectionSource['panel']) => {
    updateSelection({
      frame,
      streamId: frame.stream,
      temporal: {
        type: 'point',
        frameIndex: frame.frameIndex,
      },
    }, source);
  }, [updateSelection]);

  const setUnitSelection = useCallback((unit: UnitKey, source: SelectionSource['panel']) => {
    updateSelection({ unit, streamId: unit.stream }, source);
  }, [updateSelection]);

  const setSyntaxSelection = useCallback((node: SyntaxNodeId, source: SelectionSource['panel']) => {
    updateSelection({ syntaxNode: node }, source);
  }, [updateSelection]);

  const setBitRangeSelection = useCallback((range: BitRange, source: SelectionSource['panel']) => {
    updateSelection({ bitRange: range }, source);
  }, [updateSelection]);

  const clearTemporal = useCallback(() => {
    setSelection(prev => {
      if (!prev) return null;
      const newSelection: SelectionState = {
        ...prev,
        temporal: null,
        source: {
          panel: 'syntax',
          timestamp: Date.now(),
        },
      };
      notifyListeners(newSelection);
      return newSelection;
    });
  }, [notifyListeners]);

  const clearAll = useCallback(() => {
    setSelection(null);
  }, []);

  const subscribe = useCallback((callback: (event: SelectionChangeEvent) => void) => {
    listeners.add(callback);
    return () => {
      listeners.delete(callback);
    };
  }, [listeners]);

  const value: SelectionContextType = {
    selection,
    setTemporalSelection,
    setFrameSelection,
    setUnitSelection,
    setSyntaxSelection,
    setBitRangeSelection,
    clearTemporal,
    clearAll,
    subscribe,
  };

  return (
    <SelectionContext.Provider value={value}>
      {children}
    </SelectionContext.Provider>
  );
}

// ════════════════════════════════════════════════════════════════════════════════
// Hook
// ════════════════════════════════════════════════════════════════════════════════

export function useSelection(): SelectionContextType {
  const context = useContext(SelectionContext);
  if (!context) {
    throw new Error('useSelection must be used within SelectionProvider');
  }
  return context;
}

// Helper hook for panels that need to react to selection changes
export function useSelectionSubscribe(callback: (event: SelectionChangeEvent) => void, deps: unknown[] = []) {
  const { subscribe } = useSelection();

  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(() => {
    const unsubscribe = subscribe(callback);
    return unsubscribe;
  }, deps); // Re-subscribe when deps change
}

// ════════════════════════════════════════════════════════════════════════════════
// Utilities
// ════════════════════════════════════════════════════════════════════════════════

function estimateBitSize(fieldType: string): number {
  // Rough estimate for common field types
  const bits: Record<string, number> = {
    'u1': 1,
    'u2': 2,
    'u3': 3,
    'u4': 4,
    'u5': 5,
    'u6': 6,
    'u7': 7,
    'u8': 8,
    'ue(v)': 0, // Variable, need actual parsing
    'leb128': 0,
    'bytes': 32, // Default
  };

  return bits[fieldType] ?? 32;
}
