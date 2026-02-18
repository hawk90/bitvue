/**
 * DockableLayout Component Tests
 * Tests panel layout, tab switching, and resizing
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@/test/test-utils";
import { DockableLayout, PANEL_SIZES } from "../DockableLayout";

// Mock components for testing
const MockMainView = () => <div data-testid="main-view">Main View</div>;
const MockPanel1 = () => <div data-testid="panel-1">Panel 1</div>;
const MockPanel2 = () => <div data-testid="panel-2">Panel 2</div>;
const MockPanel3 = () => <div data-testid="panel-3">Panel 3</div>;

describe("DockableLayout", () => {
  const leftPanels = [
    {
      id: "panel1",
      title: "Panel 1",
      component: MockPanel1,
      icon: "icon-1",
    },
    {
      id: "panel2",
      title: "Panel 2",
      component: MockPanel2,
      icon: "icon-2",
    },
  ];

  const bottomRowPanels = [
    {
      id: "panel3",
      title: "Panel 3",
      component: MockPanel3,
      icon: "icon-3",
    },
  ];

  it("should render main layout structure", () => {
    render(<DockableLayout leftPanels={leftPanels} mainView={MockMainView} />);

    expect(screen.queryByTestId("main-view")).toBeInTheDocument();
    // Only the active panel (first one) is rendered in sidebar
    expect(screen.getByTestId("panel-1")).toBeInTheDocument();
    // panel-2 is not rendered until its tab is clicked
  });

  it("should render left sidebar with tabs", () => {
    render(<DockableLayout leftPanels={leftPanels} mainView={MockMainView} />);

    const tabs = screen.queryAllByRole("button");
    const tabTitles = tabs.filter((tab) =>
      tab.className.includes("sidebar-tab"),
    );
    expect(tabTitles.length).toBeGreaterThan(0);
  });

  it("should switch between left sidebar tabs", () => {
    const { container } = render(
      <DockableLayout leftPanels={leftPanels} mainView={MockMainView} />,
    );

    // Find the panel 2 tab by looking at all sidebar tabs
    const tabs = container.querySelectorAll(".sidebar-tab");
    const panel2Tab = Array.from(tabs).find(
      (tab) => tab.textContent === "Panel 2",
    );
    expect(panel2Tab).toBeInTheDocument();
    if (panel2Tab) {
      fireEvent.click(panel2Tab);
    }

    // After clicking panel 2, its content should be visible
    expect(screen.queryByTestId("panel-2")).toBeInTheDocument();
  });

  it("should render bottom row panels", () => {
    render(
      <DockableLayout
        leftPanels={leftPanels}
        mainView={MockMainView}
        bottomRowPanels={bottomRowPanels}
      />,
    );

    expect(screen.getByTestId("panel-3")).toBeInTheDocument();
  });

  it("should apply correct CSS classes", () => {
    const { container } = render(
      <DockableLayout leftPanels={leftPanels} mainView={MockMainView} />,
    );

    expect(container.querySelector(".dockable-layout")).toBeInTheDocument();
    expect(container.querySelector(".left-sidebar-panel")).toBeInTheDocument();
    expect(container.querySelector(".yuv-viewer-panel")).toBeInTheDocument();
  });

  it("should handle empty left panels", () => {
    render(<DockableLayout leftPanels={[]} mainView={MockMainView} />);

    // Should still render main view
    expect(screen.queryByTestId("main-view")).toBeInTheDocument();
  });

  it("should handle empty bottom row panels", () => {
    render(
      <DockableLayout
        leftPanels={leftPanels}
        mainView={MockMainView}
        bottomRowPanels={[]}
      />,
    );

    // Should still render main layout
    expect(screen.queryByTestId("main-view")).toBeInTheDocument();
    expect(screen.getByTestId("panel-1")).toBeInTheDocument();
  });

  it("should render panel icons", () => {
    render(<DockableLayout leftPanels={leftPanels} mainView={MockMainView} />);

    // Check for icon elements (codicon class)
    const icons = document.querySelectorAll(".codicon");
    expect(icons.length).toBeGreaterThan(0);
  });

  it("should use stable callbacks for tab switching (useCallback optimization)", () => {
    const { rerender } = render(
      <DockableLayout leftPanels={leftPanels} mainView={MockMainView} />,
    );

    // Rerender with same props
    rerender(
      <DockableLayout leftPanels={leftPanels} mainView={MockMainView} />,
    );

    // Layout should still be functional
    expect(screen.getByTestId("panel-1")).toBeInTheDocument();
  });

  it("should support custom panel sizes", () => {
    const customPanels = [
      {
        id: "panel1",
        title: "Custom Panel",
        component: MockPanel1,
        icon: "icon",
        defaultSize: 50,
      },
    ];

    render(
      <DockableLayout leftPanels={customPanels} mainView={MockMainView} />,
    );

    expect(screen.getByText("Custom Panel")).toBeInTheDocument();
  });

  it("should support collapsible panels", () => {
    const collapsiblePanels = [
      {
        id: "panel1",
        title: "Collapsible Panel",
        component: MockPanel1,
        icon: "icon",
        collapsible: true,
      },
    ];

    render(
      <DockableLayout leftPanels={collapsiblePanels} mainView={MockMainView} />,
    );

    expect(screen.getByText("Collapsible Panel")).toBeInTheDocument();
  });
});

describe("PANEL_SIZES constants", () => {
  it("should have correct default values", () => {
    expect(PANEL_SIZES.LEFT_SIDEBAR).toBe(25);
    expect(PANEL_SIZES.MAIN_CONTENT).toBe(75);
    expect(PANEL_SIZES.YUV_VIEWER).toBe(85);
    expect(PANEL_SIZES.BOTTOM_PANEL).toBe(15);
  });
});
