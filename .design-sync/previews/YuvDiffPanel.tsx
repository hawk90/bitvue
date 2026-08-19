import { YuvDiffPanel } from "bitvue";
import { Component, type CSSProperties, type ReactNode } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24, width: 640 };

// KNOWN BLOCKER (flagged, not fixed -- out of scope for a previews-only pass): YuvDiffPanel calls
// `useYuvDiff()` (frontend/contexts/YuvDiffContext.tsx) unconditionally as its very first line.
// That context is NOT one of the 7 providers the DS build harness auto-wraps every card in
// (.design-sync/config.json's `provider` chain is Theme->Layout->FrameData->FileState->
// CurrentFrame->Selection->Mode only), and `YuvDiffContext.tsx` is also not in `extraEntries`, so
// it isn't even re-exported from the "bitvue" bare specifier this preview is required to import
// from -- there is no way to reach the real `YuvDiffProvider` from this file at all. Without a
// provider in the tree, `useYuvDiff()` throws synchronously
// ("useYuvDiff must be used inside YuvDiffProvider") on every render, regardless of props.
// Fix (needs a config.json change, not a previews change): add
// "./contexts/YuvDiffContext.tsx" to `extraEntries` and `YuvDiffProvider` to the `provider` chain.
//
// This preview is still written to the real prop shape (`currentFrameIndex`/`onJumpToFrame`) so
// it starts working the moment that config change lands. Meanwhile, a small local error boundary
// (plain React, no relative frontend/ import) turns the otherwise-uncaught crash into a legible
// in-card explanation instead of a blank/broken catalog card.
class MissingProviderBoundary extends Component<
  { children: ReactNode },
  { failed: boolean }
> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  render() {
    if (this.state.failed) {
      return (
        <div
          style={{
            border: "1px dashed #666",
            borderRadius: 4,
            padding: 16,
            color: "#aaa",
            fontSize: 12,
            fontFamily: "monospace",
            lineHeight: 1.6,
          }}
        >
          YuvDiffPanel requires YuvDiffProvider, which isn't wired into the design-sync harness's
          provider chain (.design-sync/config.json) or extraEntries yet -- see comment in
          YuvDiffPanel.tsx preview source for details.
        </div>
      );
    }
    return this.props.children;
  }
}

export const NoFileLoaded = () => (
  <div style={previewBg}>
    <MissingProviderBoundary>
      <YuvDiffPanel currentFrameIndex={0} onJumpToFrame={() => {}} />
    </MissingProviderBoundary>
  </div>
);

export const WithReferenceLoaded = () => (
  <div style={previewBg}>
    <MissingProviderBoundary>
      <YuvDiffPanel currentFrameIndex={86} onJumpToFrame={() => {}} />
    </MissingProviderBoundary>
  </div>
);
