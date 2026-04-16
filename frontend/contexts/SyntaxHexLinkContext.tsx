/**
 * SyntaxHexLink Context
 *
 * Shared state for cross-panel linking between Syntax tree nodes and HEX view.
 * When a syntax node with a byte_offset is clicked, this context propagates
 * the offset to the HEX panel for scrolling and highlighting.
 */

import { createContext, useContext, useState, ReactNode, useMemo } from "react";

interface SyntaxHexLinkContextType {
  /** Byte offset to highlight in HEX view, or null if none */
  highlightedByteOffset: number | null;
  setHighlightedByteOffset: (offset: number | null) => void;
}

const SyntaxHexLinkContext = createContext<SyntaxHexLinkContextType>({
  highlightedByteOffset: null,
  setHighlightedByteOffset: () => {},
});

export function SyntaxHexLinkProvider({ children }: { children: ReactNode }) {
  const [highlightedByteOffset, setHighlightedByteOffset] = useState<
    number | null
  >(null);

  const value = useMemo(
    () => ({ highlightedByteOffset, setHighlightedByteOffset }),
    [highlightedByteOffset],
  );

  return (
    <SyntaxHexLinkContext.Provider value={value}>
      {children}
    </SyntaxHexLinkContext.Provider>
  );
}

export function useSyntaxHexLink(): SyntaxHexLinkContextType {
  return useContext(SyntaxHexLinkContext);
}
