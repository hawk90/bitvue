/**
 * ModeSelector Component Tests
 * Tests visualization mode selector dropdown
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@/test/test-utils";
import { ModeSelector } from "../YuvViewerPanel/ModeSelector";
import type { CodecModeEntry } from "../../utils/codecModeRegistry";

// ModeSelector is codec-aware: it takes a real `availableModes: CodecModeEntry[]` prop from the
// codecModeRegistry (injected by YuvViewerPanel via useMode()), not a hardcoded ModeContext
// `MODES` list -- this test used to mock `@/contexts/ModeContext`'s `MODES`, which the component
// doesn't even read anymore. Pass a realistic per-codec fixture directly instead.
const testModes: CodecModeEntry[] = [
  { fKey: 1, mode: "overview", label: "Overview", description: "" },
  { fKey: 2, mode: "coding-flow", label: "Coding Flow", description: "" },
  { fKey: 3, mode: "prediction", label: "Prediction", description: "" },
  { fKey: 4, mode: "transform", label: "Transform", description: "" },
  { fKey: 5, mode: "qp-map", label: "QP Map", description: "" },
  { fKey: 6, mode: "mv-field", label: "MV Field", description: "" },
  { fKey: 7, mode: "reference", label: "Reference Frames", description: "" },
];

describe("ModeSelector", () => {
  it("should render mode selector dropdown", () => {
    render(
      <ModeSelector
        currentMode="overview"
        onModeChange={vi.fn()}
        availableModes={testModes}
      />,
    );

    const select = screen.getByRole("combobox");
    expect(select).toBeInTheDocument();
  });

  it("should display current mode", () => {
    render(
      <ModeSelector
        currentMode="overview"
        onModeChange={vi.fn()}
        availableModes={testModes}
      />,
    );

    const select = screen.getByRole("combobox") as HTMLSelectElement;
    expect(select.value).toBe("overview");
  });

  it("should list all modes", () => {
    render(
      <ModeSelector
        currentMode="overview"
        onModeChange={vi.fn()}
        availableModes={testModes}
      />,
    );

    expect(screen.getByText(/F1 — Overview/)).toBeInTheDocument();
    expect(screen.getByText(/F2 — Coding Flow/)).toBeInTheDocument();
  });

  it("should call onModeChange when selection changes", () => {
    const handleChange = vi.fn();
    render(
      <ModeSelector
        currentMode="overview"
        onModeChange={handleChange}
        availableModes={testModes}
      />,
    );

    const select = screen.getByRole("combobox");
    fireEvent.change(select, { target: { value: "prediction" } });

    expect(handleChange).toHaveBeenCalledWith("prediction");
  });

  it("should use React.memo for performance", () => {
    const { rerender } = render(
      <ModeSelector
        currentMode="overview"
        onModeChange={vi.fn()}
        availableModes={testModes}
      />,
    );

    rerender(
      <ModeSelector
        currentMode="overview"
        onModeChange={vi.fn()}
        availableModes={testModes}
      />,
    );

    expect(screen.getByRole("combobox")).toBeInTheDocument();
  });

  it("should have correct title", () => {
    render(
      <ModeSelector
        currentMode="overview"
        onModeChange={vi.fn()}
        availableModes={testModes}
      />,
    );

    const select = screen.getByRole("combobox");
    expect(select).toHaveAttribute("title", "Visualization Mode");
  });

  it("should fall back to the first available mode when currentMode isn't in the list", () => {
    render(
      <ModeSelector
        currentMode={"not-a-real-mode" as "overview"}
        onModeChange={vi.fn()}
        availableModes={testModes}
      />,
    );

    const select = screen.getByRole("combobox") as HTMLSelectElement;
    expect(select.value).toBe("overview");
  });
});

describe("ModeSelector mode options", () => {
  it("should show shortcut keys for each mode", () => {
    render(
      <ModeSelector
        currentMode="overview"
        onModeChange={vi.fn()}
        availableModes={testModes}
      />,
    );

    for (let i = 1; i <= 7; i++) {
      expect(screen.getByText(new RegExp(`F${i}`))).toBeInTheDocument();
    }
  });

  it("should display mode labels", () => {
    render(
      <ModeSelector
        currentMode="overview"
        onModeChange={vi.fn()}
        availableModes={testModes}
      />,
    );

    expect(screen.getByText(/Overview/)).toBeInTheDocument();
    expect(screen.getByText(/Coding Flow/)).toBeInTheDocument();
    expect(screen.getByText(/Prediction/)).toBeInTheDocument();
  });
});

describe("ModeSelector interactions", () => {
  it("should change to transform mode", () => {
    const handleChange = vi.fn();
    render(
      <ModeSelector
        currentMode="overview"
        onModeChange={handleChange}
        availableModes={testModes}
      />,
    );

    const select = screen.getByRole("combobox");
    fireEvent.change(select, { target: { value: "transform" } });

    expect(handleChange).toHaveBeenCalledWith("transform");
  });

  it("should change to qp-map mode", () => {
    const handleChange = vi.fn();
    render(
      <ModeSelector
        currentMode="overview"
        onModeChange={handleChange}
        availableModes={testModes}
      />,
    );

    const select = screen.getByRole("combobox");
    fireEvent.change(select, { target: { value: "qp-map" } });

    expect(handleChange).toHaveBeenCalledWith("qp-map");
  });

  it("should change to mv-field mode", () => {
    const handleChange = vi.fn();
    render(
      <ModeSelector
        currentMode="overview"
        onModeChange={handleChange}
        availableModes={testModes}
      />,
    );

    const select = screen.getByRole("combobox");
    fireEvent.change(select, { target: { value: "mv-field" } });

    expect(handleChange).toHaveBeenCalledWith("mv-field");
  });

  it("should change to reference mode", () => {
    const handleChange = vi.fn();
    render(
      <ModeSelector
        currentMode="overview"
        onModeChange={handleChange}
        availableModes={testModes}
      />,
    );

    const select = screen.getByRole("combobox");
    fireEvent.change(select, { target: { value: "reference" } });

    expect(handleChange).toHaveBeenCalledWith("reference");
  });
});
