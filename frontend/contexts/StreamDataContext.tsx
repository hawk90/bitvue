/**
 * Stream Data Context
 *
 * @deprecated This context is split into multiple focused contexts for better performance.
 * Use the individual contexts instead:
 * - FrameDataContext for frame data (useFrameData)
 * - FileStateContext for file operations (useFileState)
 * - CurrentFrameContext for navigation (useCurrentFrame)
 *
 * This file now re-exports the split contexts. Still a real, live indirection layer -- App.tsx,
 * BitViewPanel.tsx, and useAppFileOperations.ts import through this file rather than the split
 * contexts directly, so it can't be deleted outright (verified 2026-08-20: removing it broke
 * typecheck). Collapsing those callers onto direct imports is axis-2 (Context ownership) scope,
 * not this cleanup pass.
 */

// Re-export the split contexts for easy migration
export { FrameDataProvider, useFrameData } from "./FrameDataContext";
export type { FrameDataContextType } from "./FrameDataContext";

export { FileStateProvider, useFileState } from "./FileStateContext";
export type { FileStateContextType } from "./FileStateContext";

export { CurrentFrameProvider, useCurrentFrame } from "./CurrentFrameContext";

// Re-export types
export type { FrameStats } from "./FrameDataContext";
