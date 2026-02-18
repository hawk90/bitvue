/**
 * Mode Context
 *
 * Manages visualization mode state for the main viewer
 * F1-F7 mode selection, overlay rendering, and component toggles
 */

import {
  createContext,
  useContext,
  useState,
  useCallback,
  ReactNode,
} from "react";

export type VisualizationMode =
  | "overview" // F1: Basic frame display
  | "coding-flow" // F2: Encoder/decoder pipeline
  | "prediction" // F3: Intra/inter prediction modes
  | "transform" // F4: Transform coefficients
  | "qp-map" // F5: Quantization parameter heatmap
  | "mv-field" // F6: Motion vector field
  | "reference"; // F7: Reference frames

export type YuvComponent = "y" | "u" | "v";
export type ComponentMask = `${YuvComponent}${YuvComponent}${YuvComponent}`;

export interface ModeContextType {
  currentMode: VisualizationMode;
  setMode: (mode: VisualizationMode) => void;
  cycleMode: () => void;
  // Component visibility
  componentMask: ComponentMask;
  toggleComponent: (component: YuvComponent) => void;
  setComponentMask: (mask: ComponentMask) => void;
  // Overlay toggles
  showGrid: boolean;
  toggleGrid: () => void;
  showLabels: boolean;
  toggleLabels: () => void;
  showBlockTypes: boolean;
  toggleBlockTypes: () => void;
}

const ModeContext = createContext<ModeContextType | undefined>(undefined);

export function ModeProvider({ children }: { children: ReactNode }) {
  const [currentMode, setCurrentMode] = useState<VisualizationMode>("overview");

  // Component visibility (Y, U, V) - all visible by default
  const [componentMask, setComponentMaskState] = useState<ComponentMask>("yuv");

  // Overlay toggles
  const [showGrid, setShowGrid] = useState(false);
  const [showLabels, setShowLabels] = useState(true);
  const [showBlockTypes, setShowBlockTypes] = useState(false);

  const cycleMode = useCallback(() => {
    const modes: VisualizationMode[] = [
      "overview",
      "coding-flow",
      "prediction",
      "transform",
      "qp-map",
      "mv-field",
      "reference",
    ];
    const currentIndex = modes.indexOf(currentMode);
    const nextIndex = (currentIndex + 1) % modes.length;
    setCurrentMode(modes[nextIndex]);
  }, [currentMode]);

  const toggleComponent = useCallback((component: YuvComponent) => {
    setComponentMaskState((prev) => {
      const components = prev.split("") as YuvComponent[];
      const index = components.indexOf(component);
      const ORDER: YuvComponent[] = ["y", "u", "v"];

      if (index > -1) {
        // Remove component if present
        const newComponents = [...components];
        newComponents.splice(index, 1);
        return newComponents.join("") as ComponentMask;
      } else {
        // Add component in original order
        const ordered = ORDER.filter(
          (c) => components.includes(c) || c === component,
        );
        return ordered.join("") as ComponentMask;
      }
    });
  }, []);

  const toggleGrid = useCallback(() => {
    setShowGrid((prev) => !prev);
  }, []);

  const toggleLabels = useCallback(() => {
    setShowLabels((prev) => !prev);
  }, []);

  const toggleBlockTypes = useCallback(() => {
    setShowBlockTypes((prev) => !prev);
  }, []);

  return (
    <ModeContext.Provider
      value={{
        currentMode,
        setMode: setCurrentMode,
        cycleMode,
        componentMask,
        toggleComponent,
        setComponentMask: setComponentMaskState,
        showGrid,
        toggleGrid,
        showLabels,
        toggleLabels,
        showBlockTypes,
        toggleBlockTypes,
      }}
    >
      {children}
    </ModeContext.Provider>
  );
}

export function useMode(): ModeContextType {
  const context = useContext(ModeContext);
  if (!context) {
    throw new Error("useMode must be used within a ModeProvider");
  }
  return context;
}

// Mode metadata
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
];

// Component metadata
export const COMPONENTS: { key: YuvComponent; label: string; color: string }[] =
  [
    { key: "y", label: "Y (Luma)", color: "#888888" },
    { key: "u", label: "U (Cb)", color: "#00bfff" },
    { key: "v", label: "V (Cr)", color: "#ff6b6b" },
  ];
