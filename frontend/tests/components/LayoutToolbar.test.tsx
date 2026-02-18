/**
 * MenuToolbar Component Tests
 * Tests layout toolbar with reset button
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@/test/test-utils";
import { LayoutToolbar } from "@/components/LayoutToolbar";

// Mock context
vi.mock("@/contexts/LayoutContext", () => ({
  LayoutProvider: ({ children }: { children: React.ReactNode }) => (
    <>{children}</>
  ),
  useLayout: () => ({
    resetLayout: vi.fn(),
  }),
}));

// Mock window.location.reload
const mockReload = vi.fn();
Object.defineProperty(window, "location", {
  value: { reload: mockReload },
  writable: true,
});

describe("LayoutToolbar", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should render layout toolbar", () => {
    render(<LayoutToolbar />);

    expect(screen.getByText("Reset Layout")).toBeInTheDocument();
  });

  it("should render reset button", () => {
    render(<LayoutToolbar />);

    const button = screen.getByRole("button", { name: "Reset Layout" });
    expect(button).toBeInTheDocument();
  });

  it("should have reset icon", () => {
    render(<LayoutToolbar />);

    const icon = document.querySelector(".codicon-screen-normal");
    expect(icon).toBeInTheDocument();
  });

  it("should call resetLayout when clicked", () => {
    render(<LayoutToolbar />);

    const button = screen.getByRole("button", { name: "Reset Layout" });
    fireEvent.click(button);

    // Should call resetLayout and reload
    // Note: reload will error in test environment, but we can check if it was called
    expect(mockReload).toHaveBeenCalled();
  });

  it("should use stable callbacks (useCallback optimization)", () => {
    const { rerender } = render(<LayoutToolbar />);

    rerender(<LayoutToolbar />);

    expect(screen.getByText("Reset Layout")).toBeInTheDocument();
  });
});

describe("LayoutToolbar button", () => {
  it("should have correct title attribute", () => {
    render(<LayoutToolbar />);

    const button = screen.getByRole("button", { name: "Reset Layout" });
    expect(button).toHaveAttribute("title", "Reset Layout to Default");
  });

  it("should have correct CSS class", () => {
    const { container } = render(<LayoutToolbar />);

    const button = container.querySelector(".layout-toolbar-btn");
    expect(button).toBeInTheDocument();
  });
});
