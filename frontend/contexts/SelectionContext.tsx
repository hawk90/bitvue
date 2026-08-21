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
  useMemo,
  ReactNode,
  useRef,
  useSyncExternalStore,
} from "react";

// Import extracted types and utilities
import type {
  SelectionState,
  SelectionContextType,
  SelectionChangeEvent,
  FrameKey,
} from "../types/selection";
import {
  applyTriSyncRules,
  mergeSelectionUpdates,
} from "../utils/selectionSync";
import {
  selectBitRange,
  type SelectionUpdatedEvent,
} from "../services/electronBridgeService";

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
// Active-frame store (useSyncExternalStore) -- see types/selection.ts's SelectionState doc,
// invariant 1
// ════════════════════════════════════════════════════════════════════════════════

// Separate from SelectionContext itself: useContext(SelectionContext) re-renders on ANY
// selection change (it's one useState<SelectionState>, see the module doc above), which is
// exactly the "unit/bitRange selection wakes up the frame preview" problem this store exists to
// avoid. This context's own value never changes reference across renders (see the useMemo below,
// both deps are stable-forever useCallbacks), so subscribing to it costs nothing -- all the real
// re-render gating happens inside useSyncExternalStore itself, via getFrameSnapshot returning a
// referentially stable FrameKey whenever `frame` itself didn't change (mergeSelectionUpdates
// spreads the previous SelectionState, so untouched keys -- including `frame` -- keep their old
// object identity; only an update that actually touches `frame` produces a new one).
interface SelectionStore {
  subscribe: (onStoreChange: () => void) => () => void;
  getFrameSnapshot: () => FrameKey | null;
  // Dispatch, carried on this same stable object rather than a second context: every setter
  // below is already a stable-forever useCallback (empty deps, or deps on another stable-forever
  // callback), so reading them from here -- instead of useSelection(), which forces a re-render
  // on every selection change just by being read via useContext -- costs nothing. This is the
  // "useNavigationDispatch()" half of the design; useActiveFrame is the "useActiveFrame()" half.
  setFrameSelection: SelectionContextType["setFrameSelection"];
}

const SelectionStoreContext = createContext<SelectionStore | null>(null);

/** The active frame -- see types/selection.ts's SelectionState doc, invariant 1. Re-renders only
 *  when `frame` itself changes, not on unrelated unit/syntax/bitRange selection updates (unlike
 *  `useSelection().selection.frame`, which re-renders on every selection change). */
export function useActiveFrame(): FrameKey | null {
  const store = useContext(SelectionStoreContext);
  if (!store) {
    throw new Error("useActiveFrame must be used within SelectionProvider");
  }
  return useSyncExternalStore(store.subscribe, store.getFrameSnapshot);
}

/**
 * Compatibility shim for the old `contexts/CurrentFrameContext.tsx` (removed 2026-08-21,
 * axis-2 Context-ownership cleanup) -- same `{currentFrameIndex, setCurrentFrameIndex}` shape,
 * backed by SelectionContext's `frame` instead of a second independent `useState`. Existing
 * consumers (App.tsx's `*FromContext` wrappers, BitViewPanel, DiagnosticsPanel,
 * SelectionInfoPanel, SyntaxDetailPanel, UnitHexPanel) needed only an import-path change to pick
 * this up -- see types/selection.ts's SelectionState doc, invariant 1, for why this exists
 * instead of two pieces of state kept equal by a bridge (`FrameSyncBridge.tsx`, also removed).
 *
 * Always addresses stream "A" -- matches what the old CurrentFrameContext always implicitly
 * meant (it never tracked a stream at all); stream B's position is tracked separately by
 * CompareContext (`currentFrameB`/`setFrameB`), untouched by this. New code should prefer
 * useActiveFrame() directly instead: unlike this wrapper, it doesn't need useSelection() (whose
 * re-render-on-any-change is exactly what useActiveFrame exists to avoid).
 */
export function useCurrentFrame(): {
  currentFrameIndex: number;
  setCurrentFrameIndex: (frameIndex: number) => void;
} {
  const store = useContext(SelectionStoreContext);
  if (!store) {
    throw new Error("useCurrentFrame must be used within SelectionProvider");
  }
  const activeFrame = useSyncExternalStore(
    store.subscribe,
    store.getFrameSnapshot,
  );
  const setCurrentFrameIndex = useCallback(
    (frameIndex: number) => {
      store.setFrameSelection({ stream: "A", frameIndex }, "main");
    },
    [store],
  );
  return {
    currentFrameIndex: activeFrame?.frameIndex ?? 0,
    setCurrentFrameIndex,
  };
}

// ════════════════════════════════════════════════════════════════════════════════
// Provider
// ════════════════════════════════════════════════════════════════════════════════

interface SelectionProviderProps {
  children: ReactNode;
}

export function SelectionProvider({ children }: SelectionProviderProps) {
  const [selection, setSelection] = useState<SelectionState | null>(null);
  const listenersRef = useRef(new Set<(event: SelectionChangeEvent) => void>());

  // Kept in sync during render (not an effect -- useSyncExternalStore's getFrameSnapshot must be
  // correct synchronously, including on the very render that changed `selection`, not one tick
  // later). Only ever read imperatively by getFrameSnapshot below, never used to drive this
  // component's own render output -- the one case React's docs sanction mutating a ref mid-render.
  const selectionRef = useRef(selection);
  selectionRef.current = selection;

  // Cleanup all listeners when provider unmounts
  useEffect(() => {
    const listeners = listenersRef.current;
    return () => {
      listeners.clear();
    };
  }, []);

  // Notifies listeners (including useActiveFrame's useSyncExternalStore subscribers) from an
  // effect, never from inside a setSelection updater -- an updater function is supposed to be
  // pure, and a subscriber calling setState in response to notifyListeners (useSyncExternalStore
  // does exactly this) triggered a real "Cannot update a component while rendering a different
  // component" warning + a double-fire when it *was* called from inside the updater (caught by
  // this file's useActiveFrame regression tests the same day this store was added -- dormant
  // until then because nothing real subscribed before). Effects run after commit, which is the
  // React-sanctioned place for this.
  useEffect(() => {
    const event: SelectionChangeEvent = {
      selection,
      source: selection?.source ?? { panel: "sync", timestamp: Date.now() },
    };
    listenersRef.current.forEach((callback) => callback(event));
  }, [selection]);

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
        return applyTriSyncRules(mergedSelection);
      });
    },
    [],
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

  // No backend round trip -- the caller (FrameSyntaxTab) already has the real bitRange from the
  // syntax tree data it fetched via getFrameSyntax, so there's nothing to resolve. Contrast with
  // setBitRangeSelection below, which goes the other direction (a bit range with no known node)
  // and genuinely needs the backend's find_nearest_node reverse mapping.
  const setSyntaxSelection = useCallback(
    (
      node: SelectionState["syntaxNode"],
      bitRange: SelectionState["bitRange"],
      source: SelectionState["source"]["panel"],
    ) => {
      updateSelection({ syntaxNode: node, bitRange }, source);
    },
    [updateSelection],
  );

  // Sets `bitRange` immediately (optimistic -- the click itself always has a real bit range to
  // show right away), then calls the real select_bit_range round trip and patches in whatever
  // syntax_node it resolves to (Core::handle_command's find_nearest_node reverse mapping,
  // crates/bitvue-engine/src/core.rs) once it comes back. `null` if nothing resolved (e.g. no
  // syntax tree cached yet for this stream/frame) -- not treated as an error, just "no match".
  const setBitRangeSelection = useCallback(
    (
      range: SelectionState["bitRange"],
      source: SelectionState["source"]["panel"],
    ) => {
      updateSelection({ bitRange: range }, source);
      if (!range) return;
      const stream = selectionRef.current?.streamId ?? "A";
      selectBitRange(stream, range.startBit, range.endBit)
        .then((events) => {
          const resolved = events[0] as SelectionUpdatedEvent | undefined;
          const syntaxNode = resolved?.syntax_node ?? null;
          if (syntaxNode) {
            updateSelection({ syntaxNode }, "sync");
          }
        })
        .catch((err) => {
          console.error("[SelectionContext] selectBitRange failed:", err);
        });
    },
    [updateSelection],
  );

  const clearTemporal = useCallback(() => {
    setSelection((prev) => {
      if (!prev) return null;
      return {
        ...prev,
        temporal: null,
        source: {
          panel: "syntax",
          timestamp: Date.now(),
        },
      };
    });
  }, []);

  const clearAll = useCallback(() => {
    setSelection(null);
  }, []);

  const subscribe = useCallback(
    (callback: (event: SelectionChangeEvent) => void) => {
      listenersRef.current.add(callback);
      return () => {
        listenersRef.current.delete(callback);
      };
    },
    [],
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

  // Stable forever: `subscribe` and `setFrameSelection` are both stable-forever useCallbacks
  // (see their own definitions above), so this object is created once and never changes
  // reference -- see useActiveFrame's doc above for why that matters.
  const storeValue = useMemo<SelectionStore>(
    () => ({
      subscribe: (onStoreChange) => subscribe(() => onStoreChange()),
      getFrameSnapshot: () => selectionRef.current?.frame ?? null,
      setFrameSelection,
    }),
    [subscribe, setFrameSelection],
  );

  return (
    <SelectionContext.Provider value={value}>
      <SelectionStoreContext.Provider value={storeValue}>
        {children}
      </SelectionStoreContext.Provider>
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
) {
  const { subscribe } = useSelection();
  const callbackRef = useRef(callback);
  callbackRef.current = callback;

  useEffect(() => subscribe((e) => callbackRef.current(e)), [subscribe]);
}
