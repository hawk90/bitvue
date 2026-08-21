/**
 * Stream Data Context
 *
 * @deprecated This context is split into multiple focused contexts for better performance.
 * Use the individual contexts instead:
 * - FrameDataContext for frame data (useFrameData)
 * - FileStateContext for file operations (useFileState)
 * - SelectionContext for the active frame (useCurrentFrame / useActiveFrame)
 *
 * This file now re-exports the split contexts. Still a real, live indirection layer -- App.tsx,
 * BitViewPanel.tsx, and useAppFileOperations.ts import through this file rather than the split
 * contexts directly, so it can't be deleted outright (verified 2026-08-20: removing it broke
 * typecheck).
 *
 * `useCurrentFrame` used to come from a dedicated `CurrentFrameContext` (a second, independent
 * `useState<number>` that `FrameSyncBridge.tsx` had to keep manually equal to
 * `SelectionContext`'s own frame state). Removed 2026-08-21 (axis-2 Context-ownership cleanup) --
 * see types/selection.ts's SelectionState doc, invariant 1. `useCurrentFrame` now re-exports
 * SelectionContext's compatibility shim instead; callers didn't need to change.
 */

// Re-export the split contexts for easy migration
export { FrameDataProvider, useFrameData } from "./FrameDataContext";
export type { FrameDataContextType } from "./FrameDataContext";

export { FileStateProvider, useFileState } from "./FileStateContext";
export type { FileStateContextType } from "./FileStateContext";

export { useCurrentFrame } from "./SelectionContext";

// Re-export types
export type { FrameStats } from "./FrameDataContext";
