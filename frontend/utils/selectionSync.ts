/**
 * Selection Sync Utilities
 *
 * Tri-Sync propagation rules for selection synchronization.
 * Extracted from SelectionContext for better testability and reusability.
 */

import type { SelectionState } from "../types/selection";

/**
 * Apply Tri-Sync propagation rules to selection state
 *
 * Tri-Sync Rules:
 * 1. Temporal selection → Frame selection
 * 2. Unit selection → BitRange selection
 *
 * (A former Rule 3, SyntaxNode → BitRange via a `fieldType`-keyed size *estimate*, was removed
 * 2026-08-21: `SyntaxNodeId` is now a flat backend-matching string with no embedded offset/
 * fieldType to estimate from -- real callers now provide the real `bitRange` directly alongside
 * the node id, see `setSyntaxSelection`'s signature in types/selection.ts.)
 *
 * @param sel - Current selection state
 * @returns Synced selection state with propagated values
 */
export function applyTriSyncRules(sel: SelectionState): SelectionState {
  // Rule 1: Temporal selection → Frame selection
  // If temporal is set and frame is not set, or if frame index doesn't match temporal
  if (
    sel.temporal &&
    (!sel.frame || sel.frame.frameIndex !== sel.temporal.frameIndex)
  ) {
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

  return sel;
}

/**
 * Create a selection source with timestamp
 */
export function createSelectionSource(
  panel: SelectionState["source"]["panel"],
): SelectionState["source"] {
  return {
    panel,
    timestamp: Date.now(),
  };
}

/**
 * Merge selection updates with existing selection state
 */
export function mergeSelectionUpdates(
  base: SelectionState | null,
  updates: Partial<SelectionState>,
  sourcePanel: SelectionState["source"]["panel"],
): SelectionState {
  return {
    ...(base || {
      streamId: "A",
      temporal: null,
      frame: null,
      unit: null,
      syntaxNode: null,
      bitRange: null,
    }),
    ...updates,
    source: createSelectionSource(sourcePanel),
  };
}
