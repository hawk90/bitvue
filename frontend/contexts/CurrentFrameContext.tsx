/**
 * Current Frame Context
 *
 * Manages the current frame index for navigation.
 *
 * Previously split into separate value/setter contexts to avoid re-rendering setter-only
 * consumers on every frame change -- collapsed back to one plain context (2026-08-20, axis-2
 * cleanup) after finding every real consumer went through the combined `useCurrentFrame()`
 * wrapper (which already subscribes to both halves), and `useCurrentFrameValue`/
 * `useCurrentFrameSetter` had zero direct callers anywhere in the frontend or its tests -- the
 * split was never actually exercised, so it was pure complexity with no realized benefit.
 */

import {
  createContext,
  useContext,
  useState,
  ReactNode,
  Dispatch,
  SetStateAction,
} from "react";

interface CurrentFrameContextType {
  currentFrameIndex: number;
  setCurrentFrameIndex: Dispatch<SetStateAction<number>>;
}

// Default (no-op setter, index 0) rather than null+throw: some tests render consumers without a
// CurrentFrameProvider ancestor and relied on this falling back quietly (found the hard way --
// switching to null+throw broke 43 App.test.tsx cases). Matches this context's original
// pre-2026-08-20 default-value behavior; not a real app path (main.tsx always mounts the
// provider above App).
const CurrentFrameContext = createContext<CurrentFrameContextType>({
  currentFrameIndex: 0,
  setCurrentFrameIndex: () => {},
});

export function CurrentFrameProvider({ children }: { children: ReactNode }) {
  const [currentFrameIndex, setCurrentFrameIndex] = useState(0);
  return (
    <CurrentFrameContext.Provider
      value={{ currentFrameIndex, setCurrentFrameIndex }}
    >
      {children}
    </CurrentFrameContext.Provider>
  );
}

export function useCurrentFrame(): CurrentFrameContextType {
  return useContext(CurrentFrameContext);
}
