import { FrameNavigationToolbar } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

// KNOWN LIMITATION (flagged, not fixed): FrameNavigationToolbar takes zero data props -- every
// value it renders (`frames`, `currentFrameIndex`) comes from `useFrameData()`/`useCurrentFrame()`
// (FrameDataContext/CurrentFrameContext). Those ARE two of the 7 providers the DS build harness
// auto-wraps every card in (`cfg.provider` in .design-sync/config.json), but the harness seeds
// them with each provider's own default state -- `FrameDataProvider`'s default is `frames: []` --
// and there is no config-level mechanism to inject non-default initial data (FrameDataProvider's
// signature is `({ children })`, no initial-state prop). The component's very first line of
// render logic is `if (frames.length === 0) return null;`, so with the harness's default (empty)
// context this renders nothing at all in every story, regardless of the `onNavigate` prop below.
// Per the "don't wrap the 7 harness-provided contexts yourself" convention, this preview does NOT
// route around it with a local FrameDataProvider/CurrentFrameProvider -- surfaced in the sync
// report instead. Fixing this cleanly needs a config.json change (out of scope for a
// previews-only pass): either seed `cfg.provider`'s FrameDataProvider/CurrentFrameProvider with
// non-empty default props, or add per-story context overrides to the DS harness itself.
export const Default = () => (
  <div style={previewBg}>
    <FrameNavigationToolbar onNavigate={() => {}} />
  </div>
);
