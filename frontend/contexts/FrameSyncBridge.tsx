/**
 * Frame Sync Bridge
 *
 * SelectionContext (Filmstrip/Timeline/BookmarksPanel/DebugPanel) and CurrentFrameContext
 * (App.tsx's Prev/Next/goto navigation, the main preview decode pipeline, InfoPanel,
 * SyntaxDetailPanel, ...) are two independent `useState`s with no code linking them --
 * clicking a filmstrip thumbnail or a timeline position moved Filmstrip's own highlight and
 * Timeline's scrubber but left the decoded preview and every metadata panel frozen on the old
 * frame, and conversely Prev/Next never moved Filmstrip/Timeline's highlight. This bridge is
 * the single place that keeps them equal, in both directions, without either side needing to
 * know the other exists.
 *
 * `lastSyncedRef` is a shared value (not one ref per direction): after either effect pushes a
 * value across, the two contexts become equal on the next render, so the *other* effect's
 * equality check on that same render is skipped -- pushing would otherwise re-trigger the
 * effect that just fired, ping-ponging forever.
 */

import { useEffect, useRef } from "react";
import { useSelection } from "./SelectionContext";
import { useCurrentFrame } from "./CurrentFrameContext";

export function FrameSyncBridge(): null {
  const { selection, setFrameSelection } = useSelection();
  const { currentFrameIndex, setCurrentFrameIndex } = useCurrentFrame();
  const selectionFrameIndex = selection?.frame?.frameIndex;
  const lastSyncedRef = useRef<number | null>(null);

  // Read via a ref (rather than a dep) so a Prev/Next push always uses the current stream
  // without needing to re-run this effect on stream changes that leave frameIndex untouched.
  const streamRef = useRef(selection?.frame?.stream);
  streamRef.current = selection?.frame?.stream;

  useEffect(() => {
    if (selectionFrameIndex == null) return;
    if (selectionFrameIndex === lastSyncedRef.current) return;
    lastSyncedRef.current = selectionFrameIndex;
    setCurrentFrameIndex(selectionFrameIndex);
  }, [selectionFrameIndex, setCurrentFrameIndex]);

  useEffect(() => {
    if (currentFrameIndex === lastSyncedRef.current) return;
    lastSyncedRef.current = currentFrameIndex;
    setFrameSelection(
      { stream: streamRef.current ?? "A", frameIndex: currentFrameIndex },
      "sync",
    );
  }, [currentFrameIndex, setFrameSelection]);

  return null;
}
