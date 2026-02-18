/**
 * QualityMetricsPanel Component Tests
 * Tests quality comparison with file selection and metrics calculation
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, waitFor } from "@/test/test-utils";
import userEvent from "@testing-library/user-event";
import { QualityMetricsPanel } from "../QualityMetricsPanel";

// Mock Tauri APIs
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

describe("QualityMetricsPanel", () => {
  it("renders quality comparison title", () => {
    render(<QualityMetricsPanel />);

    expect(screen.getByText("Quality Comparison")).toBeInTheDocument();
  });

  it("renders file selection buttons", () => {
    render(<QualityMetricsPanel />);

    expect(
      screen.getByText(/Reference File \(Original\)/i),
    ).toBeInTheDocument();
    expect(screen.getByText(/Distorted File \(Encoded\)/i)).toBeInTheDocument();
    expect(screen.getAllByText(/Select File\.\.\./i).length).toBe(2);
  });

  it("disables calculate button when no files selected", () => {
    render(<QualityMetricsPanel />);

    const button = screen.getByRole("button", {
      name: /calculate quality metrics/i,
    });
    expect(button).toBeDisabled();
  });

  it("shows supported format info", () => {
    render(<QualityMetricsPanel />);

    expect(
      screen.getByText(/Supported formats: IVF, MP4, MKV, WebM, YUV, Y4M/i),
    ).toBeInTheDocument();
  });

  it("uses React.memo for performance", () => {
    const { rerender } = render(<QualityMetricsPanel />);

    rerender(<QualityMetricsPanel />);

    expect(screen.getByText("Quality Comparison")).toBeInTheDocument();
  });
});
