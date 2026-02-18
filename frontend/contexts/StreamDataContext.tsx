/**
 * Stream Data Context
 *
 * @deprecated This context is split into multiple focused contexts for better performance.
 * Use the individual contexts instead:
 * - FrameDataContext for frame data (useFrameData)
 * - FileStateContext for file operations (useFileState)
 * - CurrentFrameContext for navigation (useCurrentFrame)
 *
 * This file now re-exports the split contexts and provides a backward-compatible wrapper.
 */

// Re-export the split contexts for easy migration
export { FrameDataProvider, useFrameData } from "./FrameDataContext";
export type { FrameDataContextType } from "./FrameDataContext";

export { FileStateProvider, useFileState } from "./FileStateContext";
export type { FileStateContextType } from "./FileStateContext";

export { CurrentFrameProvider, useCurrentFrame } from "./CurrentFrameContext";
export type { CurrentFrameContextType } from "./CurrentFrameContext";

// Re-export types
export type { FrameStats } from "./FrameDataContext";
