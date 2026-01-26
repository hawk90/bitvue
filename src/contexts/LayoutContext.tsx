/**
 * Layout Context
 *
 * Manages panel layout state and persistence
 * Provides reset layout functionality
 */

import { createContext, useContext, useState, useCallback, ReactNode, useEffect } from 'react';
import { createLogger } from '../utils/logger';

const logger = createLogger('LayoutContext');

export interface LayoutState {
  leftPanelSize: number;
  topPanelSize: number;
  bottomPanelSizes: number[];
}

interface LayoutContextType {
  layoutState: LayoutState;
  updateLeftPanel: (size: number) => void;
  updateTopPanel: (size: number) => void;
  updateBottomPanel: (index: number, size: number) => void;
  resetLayout: () => void;
  saveLayout: () => void;
  loadLayout: () => void;
}

const LayoutContext = createContext<LayoutContextType | undefined>(undefined);

// Default layout values
export const DEFAULT_LAYOUT: LayoutState = {
  leftPanelSize: 25,
  topPanelSize: 15,
  bottomPanelSizes: [33, 34, 33],
};

export function LayoutProvider({ children }: { children: ReactNode }) {
  const [layoutState, setLayoutState] = useState<LayoutState>(DEFAULT_LAYOUT);

  // Load saved layout on mount (skip in test environment)
  useEffect(() => {
    if (process.env.NODE_ENV !== 'test') {
      loadLayout();
    }
  }, []);

  const updateLeftPanel = useCallback((size: number) => {
    setLayoutState(prev => ({ ...prev, leftPanelSize: size }));
  }, []);

  const updateTopPanel = useCallback((size: number) => {
    setLayoutState(prev => ({ ...prev, topPanelSize: size }));
  }, []);

  const updateBottomPanel = useCallback((index: number, size: number) => {
    setLayoutState(prev => {
      const newSizes = [...prev.bottomPanelSizes];
      newSizes[index] = size;
      return { ...prev, bottomPanelSizes: newSizes };
    });
  }, []);

  const resetLayout = useCallback(() => {
    setLayoutState(DEFAULT_LAYOUT);
    // Clear saved layout
    try {
      localStorage.removeItem('bitvue-layout');
    } catch (e) {
      // Ignore storage errors
    }
  }, []);

  const saveLayout = useCallback(() => {
    try {
      localStorage.setItem('bitvue-layout', JSON.stringify(layoutState));
    } catch (e) {
      // Ignore storage errors
    }
  }, [layoutState]);

  const loadLayout = useCallback(() => {
    try {
      const saved = localStorage.getItem('bitvue-layout');
      if (saved) {
        const parsed = JSON.parse(saved);
        setLayoutState(parsed);
      }
    } catch (e) {
      // Use defaults on error
      logger.warn('Failed to load layout:', e);
    }
  }, []);

  // Auto-save on changes (skip in test environment)
  useEffect(() => {
    if (process.env.NODE_ENV !== 'test') {
      const timeout = setTimeout(() => {
        saveLayout();
      }, 500); // Debounce saves
      return () => clearTimeout(timeout);
    }
  }, [layoutState, saveLayout]);

  const value: LayoutContextType = {
    layoutState,
    updateLeftPanel,
    updateTopPanel,
    updateBottomPanel,
    resetLayout,
    saveLayout,
    loadLayout,
  };

  return (
    <LayoutContext.Provider value={value}>
      {children}
    </LayoutContext.Provider>
  );
}

export function useLayout(): LayoutContextType {
  const context = useContext(LayoutContext);
  if (!context) {
    throw new Error('useLayout must be used within LayoutProvider');
  }
  return context;
}
