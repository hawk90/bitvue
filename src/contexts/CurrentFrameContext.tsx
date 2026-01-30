/**
 * Current Frame Context
 *
 * Manages the current frame index for navigation
 * Separated from file data to prevent unnecessary re-renders
 */

import { createContext, useContext, useState, useCallback, ReactNode, useMemo } from 'react';

interface CurrentFrameContextType {
  currentFrameIndex: number;
  setCurrentFrameIndex: (index: number) => void;
}

const CurrentFrameContext = createContext<CurrentFrameContextType | undefined>(undefined);

export function CurrentFrameProvider({ children }: { children: ReactNode }) {
  const [currentFrameIndex, setCurrentFrameIndex] = useState(0);

  const contextValue = useMemo<CurrentFrameContextType>(() => ({
    currentFrameIndex,
    setCurrentFrameIndex,
  }), [currentFrameIndex, setCurrentFrameIndex]);

  return (
    <CurrentFrameContext.Provider value={contextValue}>
      {children}
    </CurrentFrameContext.Provider>
  );
}

export function useCurrentFrame(): CurrentFrameContextType {
  const context = useContext(CurrentFrameContext);
  if (!context) {
    throw new Error('useCurrentFrame must be used within a CurrentFrameProvider');
  }
  return context;
}
