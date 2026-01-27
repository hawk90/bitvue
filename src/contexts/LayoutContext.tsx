/**
 * Layout Context
 *
 * Manages panel layout state and persistence
 * Provides reset layout functionality
 */

import { createContext, useContext, useState, useCallback, ReactNode, useEffect } from 'react';
import { createLogger } from '../utils/logger';
import { TIMING } from '../constants/ui';

const logger = createLogger('LayoutContext');

/**
 * Storage error types for better error handling
 */
enum StorageErrorType {
  QUOTA_EXCEEDED = 'QUOTA_EXCEEDED',
  ACCESS_DENIED = 'ACCESS_DENIED',
  PARSE_ERROR = 'PARSE_ERROR',
  UNKNOWN = 'UNKNOWN',
}

/**
 * Identify the type of storage error
 */
function getStorageErrorType(error: unknown): StorageErrorType {
  if (error instanceof DOMException) {
    if (error.name === 'QuotaExceededError') {
      return StorageErrorType.QUOTA_EXCEEDED;
    }
    if (error.name === 'SecurityError') {
      return StorageErrorType.ACCESS_DENIED;
    }
  }
  if (error instanceof SyntaxError) {
    return StorageErrorType.PARSE_ERROR;
  }
  return StorageErrorType.UNKNOWN;
}

/**
 * Handle storage errors with appropriate logging and user feedback
 */
function handleStorageError(error: unknown, operation: string): void {
  const errorType = getStorageErrorType(error);

  switch (errorType) {
    case StorageErrorType.QUOTA_EXCEEDED:
      logger.warn(`Storage quota exceeded while ${operation}. Layout will not persist.`);
      break;
    case StorageErrorType.ACCESS_DENIED:
      logger.warn(`Storage access denied while ${operation}. Using default layout.`);
      break;
    case StorageErrorType.PARSE_ERROR:
      logger.warn(`Failed to parse stored layout while ${operation}. Using defaults.`);
      break;
    default:
      logger.debug(`Storage error during ${operation}:`, error);
  }
}

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
      // Create new array only when necessary
      if (prev.bottomPanelSizes[index] === size) {
        return prev; // No change needed
      }
      const newSizes = prev.bottomPanelSizes.slice(); // More efficient than spread for arrays
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
      handleStorageError(e, 'clearing layout');
    }
  }, []);

  const saveLayout = useCallback(() => {
    try {
      localStorage.setItem('bitvue-layout', JSON.stringify(layoutState));
    } catch (e) {
      handleStorageError(e, 'saving layout');
    }
  }, [layoutState]);

  const loadLayout = useCallback(() => {
    try {
      const saved = localStorage.getItem('bitvue-layout');
      if (saved) {
        const parsed = JSON.parse(saved);
        // Validate the parsed layout structure
        if (
          parsed &&
          typeof parsed === 'object' &&
          'leftPanelSize' in parsed &&
          'topPanelSize' in parsed &&
          'bottomPanelSizes' in parsed &&
          Array.isArray(parsed.bottomPanelSizes)
        ) {
          setLayoutState(parsed);
        } else {
          logger.warn('Invalid layout structure in storage, using defaults');
        }
      }
    } catch (e) {
      handleStorageError(e, 'loading layout');
    }
  }, []);

  // Auto-save on changes (skip in test environment)
  useEffect(() => {
    if (process.env.NODE_ENV !== 'test') {
      const timeout = setTimeout(() => {
        saveLayout();
      }, TIMING.STORAGE_DEBOUNCE_DELAY); // Debounce saves
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
