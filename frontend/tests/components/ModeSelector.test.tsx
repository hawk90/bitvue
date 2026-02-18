/**
 * ModeSelector Component Tests
 * Tests visualization mode selector dropdown
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@/test/test-utils";
import { ModeSelector } from "../YuvViewerPanel/ModeSelector";

// Mock MODES constant - path from __tests__ directory
vi.mock("../../contexts/ModeContext", () => ({
  MODES: [
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
  ],
  VisualizationMode: {} as any,
}));

describe("ModeSelector", () => {
  it("should render mode selector dropdown", () => {
    render(<ModeSelector currentMode="overview" onModeChange={vi.fn()} />);

    const select = screen.getByRole("combobox");
    expect(select).toBeInTheDocument();
  });

  it("should display current mode", () => {
    render(<ModeSelector currentMode="overview" onModeChange={vi.fn()} />);

    const select = screen.getByRole("combobox") as HTMLSelectElement;
    expect(select.value).toBe("overview");
  });

  it("should list all modes", () => {
    render(<ModeSelector currentMode="overview" onModeChange={vi.fn()} />);

    expect(screen.getByText(/F1 - Overview/)).toBeInTheDocument();
    expect(screen.getByText(/F2 - Coding Flow/)).toBeInTheDocument();
  });

  it("should call onModeChange when selection changes", () => {
    const handleChange = vi.fn();
    render(<ModeSelector currentMode="overview" onModeChange={handleChange} />);

    const select = screen.getByRole("combobox");
    fireEvent.change(select, { target: { value: "prediction" } });

    expect(handleChange).toHaveBeenCalledWith("prediction");
  });

  it("should use React.memo for performance", () => {
    const { rerender } = render(
      <ModeSelector currentMode="overview" onModeChange={vi.fn()} />,
    );

    rerender(<ModeSelector currentMode="overview" onModeChange={vi.fn()} />);

    expect(screen.getByRole("combobox")).toBeInTheDocument();
  });

  it("should have correct title", () => {
    render(<ModeSelector currentMode="overview" onModeChange={vi.fn()} />);

    const select = screen.getByRole("combobox");
    expect(select).toHaveAttribute("title", "Visualization Mode");
  });
});

describe("ModeSelector mode options", () => {
  it("should show shortcut keys for each mode", () => {
    render(<ModeSelector currentMode="overview" onModeChange={vi.fn()} />);

    for (let i = 1; i <= 7; i++) {
      expect(screen.getByText(new RegExp(`F${i}`))).toBeInTheDocument();
    }
  });

  it("should display mode labels", () => {
    render(<ModeSelector currentMode="overview" onModeChange={vi.fn()} />);

    // Use regex patterns since text includes shortcut prefix
    expect(screen.getByText(/Overview/)).toBeInTheDocument();
    expect(screen.getByText(/Coding Flow/)).toBeInTheDocument();
    expect(screen.getByText(/Prediction/)).toBeInTheDocument();
  });
});

describe("ModeSelector interactions", () => {
  it("should change to transform mode", () => {
    const handleChange = vi.fn();
    render(<ModeSelector currentMode="overview" onModeChange={handleChange} />);

    const select = screen.getByRole("combobox");
    fireEvent.change(select, { target: { value: "transform" } });

    expect(handleChange).toHaveBeenCalledWith("transform");
  });

  it("should change to qp-map mode", () => {
    const handleChange = vi.fn();
    render(<ModeSelector currentMode="overview" onModeChange={handleChange} />);

    const select = screen.getByRole("combobox");
    fireEvent.change(select, { target: { value: "qp-map" } });

    expect(handleChange).toHaveBeenCalledWith("qp-map");
  });

  it("should change to mv-field mode", () => {
    const handleChange = vi.fn();
    render(<ModeSelector currentMode="overview" onModeChange={handleChange} />);

    const select = screen.getByRole("combobox");
    fireEvent.change(select, { target: { value: "mv-field" } });

    expect(handleChange).toHaveBeenCalledWith("mv-field");
  });

  it("should change to reference mode", () => {
    const handleChange = vi.fn();
    render(<ModeSelector currentMode="overview" onModeChange={handleChange} />);

    const select = screen.getByRole("combobox");
    fireEvent.change(select, { target: { value: "reference" } });

    expect(handleChange).toHaveBeenCalledWith("reference");
  });
});
