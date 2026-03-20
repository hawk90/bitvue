/**
 * Current Frame Context
 *
 * Manages the current frame index for navigation.
 * Separated into value and setter contexts to prevent unnecessary re-renders:
 * - Components that only read the frame index subscribe to ValueCtx.
 * - Components that only dispatch changes subscribe to SetterCtx.
 */

import {
  createContext,
  useContext,
  useState,
  ReactNode,
  Dispatch,
  SetStateAction,
} from "react";

const ValueCtx = createContext<number>(0);
const SetterCtx = createContext<Dispatch<SetStateAction<number>>>(() => {});

export function CurrentFrameProvider({ children }: { children: ReactNode }) {
  const [currentFrame, setCurrentFrame] = useState(0);
  return (
    <SetterCtx.Provider value={setCurrentFrame}>
      <ValueCtx.Provider value={currentFrame}>{children}</ValueCtx.Provider>
    </SetterCtx.Provider>
  );
}

export const useCurrentFrameValue = (): number => useContext(ValueCtx);

export const useCurrentFrameSetter = (): Dispatch<SetStateAction<number>> =>
  useContext(SetterCtx);

// Keep useCurrentFrame for backwards compatibility
export function useCurrentFrame(): {
  currentFrameIndex: number;
  setCurrentFrameIndex: Dispatch<SetStateAction<number>>;
} {
  return {
    currentFrameIndex: useCurrentFrameValue(),
    setCurrentFrameIndex: useCurrentFrameSetter(),
  };
}
