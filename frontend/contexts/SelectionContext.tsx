/**
 * Tri-Sync Selection Context
 *
 * Core navigation system:
 * - Single source of truth for selection state
 * - Broadcasts selection changes to all panels
 * - Synchronizes: Syntax Tree ↔ Hex View ↔ Main Panel ↔ Timeline
 */

import {
  createContext,
  useContext,
  useState,
  useCallback,
  useEffect,
  ReactNode,
  useRef,
} from "react";

// Import extracted types and utilities
import type {
  SelectionState,
  SelectionContextType,
  SelectionChangeEvent,
  DEFAULT_SELECTION,
} from "../types/selection";
import {
  applyTriSyncRules,
  mergeSelectionUpdates,
} from "../utils/selectionSync";

// Re-export commonly used types for convenience
export type {
  StreamId,
  SpatialBlock,
  TemporalSelectionType,
  TemporalSelection,
  FrameKey,
  UnitKey,
  SyntaxNodeId,
  BitRange,
  SelectionSource,
  SelectionChangeEvent,
} from "../types/selection";

// ════════════════════════════════════════════════════════════════════════════════
// Selection Context
// ════════════════════════════════════════════════════════════════════════════════

const SelectionContext = createContext<SelectionContextType | null>(null);

// ════════════════════════════════════════════════════════════════════════════════
// Provider
// ════════════════════════════════════════════════════════════════════════════════

interface SelectionProviderProps {
  children: ReactNode;
}

export function SelectionProvider({ children }: SelectionProviderProps) {
  const [selection, setSelection] = useState<SelectionState | null>(null);
  const [listeners] = useState<Set<(event: SelectionChangeEvent) => void>>(
    new Set(),
  );
  // Keep track of all listener subscriptions for cleanup on unmount
  const unsubscribeFunctionsRef = useRef<Array<() => void>>([]);

  // Cleanup all listeners when provider unmounts
  useEffect(() => {
    return () => {
      // Call all unsubscribe functions to prevent memory leaks
      unsubscribeFunctionsRef.current.forEach((unsubscribe) => unsubscribe());
      unsubscribeFunctionsRef.current = [];
    };
  }, []);

  const notifyListeners = useCallback(
    (newSelection: SelectionState) => {
      const event: SelectionChangeEvent = {
        selection: newSelection,
        source: newSelection.source,
      };
      listeners.forEach((callback) => callback(event));
    },
    [listeners],
  );

  const updateSelection = useCallback(
    (
      updates: Partial<SelectionState>,
      sourcePanel: SelectionState["source"]["panel"],
    ) => {
      setSelection((prev) => {
        // Merge updates with current selection
        const mergedSelection = mergeSelectionUpdates(
          prev,
          updates,
          sourcePanel,
        );

        // Apply Tri-Sync propagation rules
        const syncedSelection = applyTriSyncRules(mergedSelection);

        notifyListeners(syncedSelection);
        return syncedSelection;
      });
    },
    [notifyListeners],
  );

  const setTemporalSelection = useCallback(
    (
      temporal: SelectionState["temporal"],
      source: SelectionState["source"]["panel"],
    ) => {
      updateSelection({ temporal }, source);
    },
    [updateSelection],
  );

  const setFrameSelection = useCallback(
    (
      frame: SelectionState["frame"],
      source: SelectionState["source"]["panel"],
    ) => {
      if (!frame) return;
      updateSelection(
        {
          frame,
          streamId: frame.stream,
          temporal: {
            type: "point",
            frameIndex: frame.frameIndex,
          },
        },
        source,
      );
    },
    [updateSelection],
  );

  const setUnitSelection = useCallback(
    (
      unit: SelectionState["unit"],
      source: SelectionState["source"]["panel"],
    ) => {
      if (!unit) return;
      updateSelection({ unit, streamId: unit.stream }, source);
    },
    [updateSelection],
  );

  const setSyntaxSelection = useCallback(
    (
      node: SelectionState["syntaxNode"],
      source: SelectionState["source"]["panel"],
    ) => {
      updateSelection({ syntaxNode: node }, source);
    },
    [updateSelection],
  );

  const setBitRangeSelection = useCallback(
    (
      range: SelectionState["bitRange"],
      source: SelectionState["source"]["panel"],
    ) => {
      updateSelection({ bitRange: range }, source);
    },
    [updateSelection],
  );

  const clearTemporal = useCallback(() => {
    setSelection((prev) => {
      if (!prev) return null;
      const newSelection: SelectionState = {
        ...prev,
        temporal: null,
        source: {
          panel: "syntax",
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

  const subscribe = useCallback(
    (callback: (event: SelectionChangeEvent) => void) => {
      listeners.add(callback);
      const unsubscribe = () => {
        listeners.delete(callback);
      };
      // Track unsubscribe function for cleanup on unmount
      unsubscribeFunctionsRef.current.push(unsubscribe);
      return unsubscribe;
    },
    [listeners],
  );

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
    throw new Error("useSelection must be used within SelectionProvider");
  }
  return context;
}

// Helper hook for panels that need to react to selection changes
export function useSelectionSubscribe(
  callback: (event: SelectionChangeEvent) => void,
  deps: unknown[] = [],
) {
  const { subscribe } = useSelection();

  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(() => {
    const unsubscribe = subscribe(callback);
    return unsubscribe;
  }, deps); // Re-subscribe when deps change
}
