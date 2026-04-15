/**
 * Mode Context
 *
 * Manages visualization mode state for the main viewer.
 * Codec-aware: available modes and F-key mappings change when a file is loaded.
 *
 * Public API additions vs v1:
 *   activeCodec      — codec string currently loaded (null = no file)
 *   setActiveCodec   — called by App when a file is opened / closed
 *   availableModes   — CodecModeEntry[] for the current codec (main modes only)
 *   availableOverlays — CodecModeEntry[] for info overlays of current codec
 */

import {
  createContext,
  useContext,
  useState,
  useCallback,
  useMemo,
  useRef,
  ReactNode,
} from "react";
import {
  type VisualizationMode,
  type CodecModeEntry,
  getMainModesForCodec,
  getInfoOverlaysForCodec,
  getDefaultModeForCodec,
  getModeByFKey,
} from "../utils/codecModeRegistry";

// Re-export so existing imports from ModeContext still work
export type { VisualizationMode };

export type YuvComponent = "y" | "u" | "v";
export type ComponentMask = `${YuvComponent}${YuvComponent}${YuvComponent}`;

export interface ModeContextType {
  // ── Core mode state ────────────────────────────────────────────────────────
  currentMode: VisualizationMode;
  setMode: (mode: VisualizationMode) => void;
  cycleMode: () => void;

  // ── Codec awareness ────────────────────────────────────────────────────────
  /** Currently loaded codec identifier (e.g. "HEVC", "AV1"). null = no file. */
  activeCodec: string | null;
  /** Update the active codec — call this when a file is opened or closed. */
  setActiveCodec: (codec: string | null) => void;
  /** Main (non-overlay) modes available for the current codec. */
  availableModes: CodecModeEntry[];
  /** Info-overlay modes available for the current codec. */
  availableOverlays: CodecModeEntry[];

  // ── Info overlay toggles ───────────────────────────────────────────────────
  /** Set of currently active info-overlay mode keys. */
  activeOverlays: ReadonlySet<VisualizationMode>;
  /** Toggle an info overlay on/off. No-op if the overlay is not available for
   *  the current codec. */
  toggleOverlay: (mode: VisualizationMode) => void;
  /** Returns true if the given overlay is currently active. */
  isOverlayActive: (mode: VisualizationMode) => boolean;
  /** Deactivate all info overlays at once. */
  clearOverlays: () => void;

  // ── Component visibility ───────────────────────────────────────────────────
  componentMask: ComponentMask;
  toggleComponent: (component: YuvComponent) => void;
  setComponentMask: (mask: ComponentMask) => void;

  // ── Legacy overlay toggles (kept for backward compat) ─────────────────────
  showGrid: boolean;
  toggleGrid: () => void;
  showLabels: boolean;
  toggleLabels: () => void;
  showBlockTypes: boolean;
  toggleBlockTypes: () => void;
}

const ModeContext = createContext<ModeContextType | undefined>(undefined);

/** localStorage key for persisted overlay preferences: Record<codec, mode[]> */
const OVERLAY_PREFS_KEY = "bitvue-overlay-prefs";

function loadOverlayPrefs(): Record<string, VisualizationMode[]> {
  try {
    const raw = localStorage.getItem(OVERLAY_PREFS_KEY);
    if (raw) return JSON.parse(raw) as Record<string, VisualizationMode[]>;
  } catch {
    // ignore parse errors
  }
  return {};
}

function saveOverlayPrefs(prefs: Record<string, VisualizationMode[]>): void {
  try {
    localStorage.setItem(OVERLAY_PREFS_KEY, JSON.stringify(prefs));
  } catch {
    // ignore storage errors
  }
}

export function ModeProvider({ children }: { children: ReactNode }) {
  const [activeCodec, setActiveCodecState] = useState<string | null>(null);
  const [currentMode, setCurrentModeState] =
    useState<VisualizationMode>("overview");
  // Keep overlay prefs in a ref to avoid stale closure issues during setActiveCodec
  const overlayPrefsRef =
    useRef<Record<string, VisualizationMode[]>>(loadOverlayPrefs());

  // Derived: available modes for the current codec
  const availableModes = useMemo(
    () => getMainModesForCodec(activeCodec),
    [activeCodec],
  );
  const availableOverlays = useMemo(
    () => getInfoOverlaysForCodec(activeCodec),
    [activeCodec],
  );

  // Info overlay toggle state — which overlays are currently shown on top
  const [activeOverlaysState, setActiveOverlaysState] = useState<
    Set<VisualizationMode>
  >(new Set());

  const toggleOverlay = useCallback(
    (mode: VisualizationMode) => {
      // Guard: only allow overlays that exist for this codec
      const isAvailable = availableOverlays.some((o) => o.mode === mode);
      if (!isAvailable) return;
      setActiveOverlaysState((prev) => {
        const next = new Set(prev);
        if (next.has(mode)) {
          next.delete(mode);
        } else {
          next.add(mode);
        }
        // Persist the new set for this codec
        if (activeCodec) {
          const prefs = overlayPrefsRef.current;
          prefs[activeCodec] = [...next];
          overlayPrefsRef.current = prefs;
          saveOverlayPrefs(prefs);
        }
        return next;
      });
    },
    [availableOverlays, activeCodec],
  );

  const isOverlayActive = useCallback(
    (mode: VisualizationMode) => activeOverlaysState.has(mode),
    [activeOverlaysState],
  );

  const clearOverlays = useCallback(() => {
    setActiveOverlaysState(new Set());
  }, []);

  // Component visibility (Y, U, V)
  const [componentMask, setComponentMaskState] = useState<ComponentMask>("yuv");

  // Overlay toggles
  const [showGrid, setShowGrid] = useState(false);
  const [showLabels, setShowLabels] = useState(true);
  const [showBlockTypes, setShowBlockTypes] = useState(false);

  /**
   * Switch active codec.
   * Resets current mode to the codec's default if the current mode is not
   * available in the new codec's mode list.
   */
  const setActiveCodec = useCallback(
    (codec: string | null) => {
      setActiveCodecState(codec);
      // Restore saved overlays for this codec, filtered to those still available
      if (codec) {
        const saved = overlayPrefsRef.current[codec] ?? [];
        const available = getInfoOverlaysForCodec(codec).map((o) => o.mode);
        const restored = saved.filter((m) => available.includes(m));
        setActiveOverlaysState(new Set(restored));
      } else {
        setActiveOverlaysState(new Set());
      }
      const newModes = getMainModesForCodec(codec);
      const isCurrentValid = newModes.some((m) => m.mode === currentMode);
      if (!isCurrentValid) {
        setCurrentModeState(getDefaultModeForCodec(codec));
      }
    },
    [currentMode],
  );

  /**
   * Set mode — only allows modes that are valid for the current codec.
   * Falls back silently if an invalid mode is requested (e.g., AV1-only mode
   * requested while HEVC is loaded).
   */
  const setMode = useCallback(
    (mode: VisualizationMode) => {
      const allEntries = [...availableModes, ...availableOverlays];
      if (allEntries.some((m) => m.mode === mode)) {
        setCurrentModeState(mode);
      } else {
        // Also allow legacy fallback modes that have no codec restriction
        const legacyModes: VisualizationMode[] = [
          "overview",
          "deblocking",
          "residuals",
          "av1-features",
        ];
        if (legacyModes.includes(mode)) {
          setCurrentModeState(mode);
        }
      }
    },
    [availableModes, availableOverlays],
  );

  /**
   * Cycle through the main modes for the current codec (wraps around).
   */
  const cycleMode = useCallback(() => {
    if (availableModes.length === 0) return;
    const currentIndex = availableModes.findIndex(
      (m) => m.mode === currentMode,
    );
    const nextIndex = (currentIndex + 1) % availableModes.length;
    setCurrentModeState(availableModes[nextIndex].mode);
  }, [currentMode, availableModes]);

  /**
   * Handle an F-key press — maps to the correct mode for the active codec.
   * Returns true if the key was handled, false if no mode is bound to that key.
   */
  const handleFKey = useCallback(
    (fKey: number): boolean => {
      const mode = getModeByFKey(activeCodec, fKey);
      if (mode) {
        setCurrentModeState(mode);
        return true;
      }
      return false;
    },
    [activeCodec],
  );

  const toggleComponent = useCallback((component: YuvComponent) => {
    setComponentMaskState((prev) => {
      const components = prev.split("") as YuvComponent[];
      const index = components.indexOf(component);
      const ORDER: YuvComponent[] = ["y", "u", "v"];
      if (index > -1) {
        const next = [...components];
        next.splice(index, 1);
        return next.join("") as ComponentMask;
      } else {
        const ordered = ORDER.filter(
          (c) => components.includes(c) || c === component,
        );
        return ordered.join("") as ComponentMask;
      }
    });
  }, []);

  const toggleGrid = useCallback(() => setShowGrid((p) => !p), []);
  const toggleLabels = useCallback(() => setShowLabels((p) => !p), []);
  const toggleBlockTypes = useCallback(() => setShowBlockTypes((p) => !p), []);

  const value = useMemo<ModeContextType>(
    () => ({
      currentMode,
      setMode,
      cycleMode,
      activeCodec,
      setActiveCodec,
      availableModes,
      availableOverlays,
      activeOverlays: activeOverlaysState,
      toggleOverlay,
      isOverlayActive,
      clearOverlays,
      componentMask,
      toggleComponent,
      setComponentMask: setComponentMaskState,
      showGrid,
      toggleGrid,
      showLabels,
      toggleLabels,
      showBlockTypes,
      toggleBlockTypes,
      handleFKey,
    }),
    [
      currentMode,
      setMode,
      cycleMode,
      activeCodec,
      setActiveCodec,
      availableModes,
      availableOverlays,
      activeOverlaysState,
      toggleOverlay,
      isOverlayActive,
      clearOverlays,
      componentMask,
      toggleComponent,
      showGrid,
      toggleGrid,
      showLabels,
      toggleLabels,
      showBlockTypes,
      toggleBlockTypes,
      handleFKey,
    ],
  );

  return <ModeContext.Provider value={value}>{children}</ModeContext.Provider>;
}

export function useMode(): ModeContextType & {
  handleFKey: (fKey: number) => boolean;
} {
  const context = useContext(ModeContext);
  if (!context) {
    throw new Error("useMode must be used within a ModeProvider");
  }
  return context as ModeContextType & { handleFKey: (fKey: number) => boolean };
}

// ─── Legacy exports (keep backward compat) ───────────────────────────────────

/**
 * @deprecated Use availableModes from useMode() instead.
 * Kept so components that import MODES directly continue to compile.
 */
export const MODES: {
  key: VisualizationMode;
  label: string;
  shortcut: string;
  description: string;
}[] = [
  {
    key: "overview",
    label: "Overview",
    shortcut: "F1",
    description: "High-level stream overview",
  },
  {
    key: "coding-flow",
    label: "Coding Flow",
    shortcut: "F2",
    description: "Encoder/decoder pipeline view",
  },
  {
    key: "prediction",
    label: "Prediction",
    shortcut: "F3",
    description: "Intra/inter prediction modes",
  },
  {
    key: "transform",
    label: "Transform",
    shortcut: "F4",
    description: "Transform coefficients",
  },
  {
    key: "qp-map",
    label: "QP Map",
    shortcut: "F5",
    description: "Quantization parameter heatmap",
  },
  {
    key: "mv-field",
    label: "MV Field",
    shortcut: "F6",
    description: "Motion vector field visualization",
  },
  {
    key: "reference",
    label: "Reference Frames",
    shortcut: "F7",
    description: "Frame dependency graph",
  },
  {
    key: "deblocking",
    label: "Deblocking",
    shortcut: "F8",
    description: "Deblocking filter boundary strength",
  },
  {
    key: "residuals",
    label: "Residuals",
    shortcut: "F9",
    description: "Residual energy heatmap (QP-based)",
  },
  {
    key: "av1-features",
    label: "AV1 Features",
    shortcut: "F10",
    description: "CDEF, Loop Restoration, Film Grain",
  },
];

export const COMPONENTS: { key: YuvComponent; label: string; color: string }[] =
  [
    { key: "y", label: "Y (Luma)", color: "#888888" },
    { key: "u", label: "U (Cb)", color: "#00bfff" },
    { key: "v", label: "V (Cr)", color: "#ff6b6b" },
  ];
