/**
 * Selection Sync Utilities
 *
 * Tri-Sync propagation rules for selection synchronization.
 * Extracted from SelectionContext for better testability and reusability.
 */

import type { SelectionState } from "../types/selection";

/**
 * Estimate bit size for a given field type
 * Used for calculating bit ranges from syntax node selections
 */
export function estimateBitSize(fieldType: string): number {
  // Rough estimate for common field types
  const bits: Record<string, number> = {
    u1: 1,
    u2: 2,
    u3: 3,
    u4: 4,
    u5: 5,
    u6: 6,
    u7: 7,
    u8: 8,
    "ue(v)": 0, // Variable, need actual parsing
    leb128: 0,
    bytes: 32, // Default
  };

  return bits[fieldType] ?? 32;
}

/**
 * Apply Tri-Sync propagation rules to selection state
 *
 * Tri-Sync Rules:
 * 1. Temporal selection → Frame selection
 * 2. Unit selection → BitRange selection
 * 3. SyntaxNode selection → BitRange selection
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

  // Rule 3: SyntaxNode selection → BitRange selection
  if (sel.syntaxNode?.offset !== undefined && !sel.bitRange) {
    const size = sel.syntaxNode.fieldType
      ? estimateBitSize(sel.syntaxNode.fieldType)
      : 32;
    return {
      ...sel,
      bitRange: {
        startBit: sel.syntaxNode.offset,
        endBit: sel.syntaxNode.offset + size,
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
