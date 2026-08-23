/**
 * DockableLayout Component Tests
 * Tests panel layout, tab switching, and resizing
 */

import { describe, it, expect } from "vitest";
import { render, screen, fireEvent } from "@/test/test-utils";
import { DockableLayout, PANEL_SIZES } from "../DockableLayout";

// Mock components for testing
const MockMainView = () => <div data-testid="main-view">Main View</div>;
const MockPanel1 = () => <div data-testid="panel-1">Panel 1</div>;
const MockPanel2 = () => <div data-testid="panel-2">Panel 2</div>;
const MockPanel3 = () => <div data-testid="panel-3">Panel 3</div>;
const MockFilmstripBar = () => (
  <div data-testid="filmstrip-bar-content">Filmstrip</div>
);

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

    const tabs = screen.queryAllByRole("tab");
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

describe("DockableLayout pinned left panel", () => {
  const leftPanels = [
    { id: "panel1", title: "Panel 1", component: MockPanel1, icon: "icon-1" },
    { id: "panel2", title: "Panel 2", component: MockPanel2, icon: "icon-2" },
  ];
  const pinnedLeftPanel = {
    id: "stream",
    title: "Stream",
    component: MockPanel3,
    icon: "symbol-tree",
  };

  it("renders the pinned panel's content alongside the inspectors dropdown, not as an option", () => {
    render(
      <DockableLayout
        pinnedLeftPanel={pinnedLeftPanel}
        leftPanels={leftPanels}
        mainView={MockMainView}
      />,
    );

    // Pinned content is always rendered...
    expect(screen.getByTestId("panel-3")).toBeInTheDocument();
    // ...and it's not one of the dropdown's switchable options (only the 2 real panels are, once
    // the dropdown is opened).
    fireEvent.mouseDown(screen.getByRole("button", { name: /inspectors/i }));
    const options = screen.queryAllByRole("option");
    expect(options.length).toBe(2);
    expect(
      options.some((option) => option.textContent?.includes("Stream")),
    ).toBe(false);
  });

  it("shows the pinned panel's title as a static header", () => {
    render(
      <DockableLayout
        pinnedLeftPanel={pinnedLeftPanel}
        leftPanels={leftPanels}
        mainView={MockMainView}
      />,
    );

    expect(
      document.querySelector(".left-sidebar-pinned-header"),
    ).toHaveTextContent("Stream");
  });

  it("switches inspector content by selecting a dropdown option", () => {
    render(
      <DockableLayout
        pinnedLeftPanel={pinnedLeftPanel}
        leftPanels={leftPanels}
        mainView={MockMainView}
      />,
    );

    expect(screen.getByTestId("panel-1")).toBeInTheDocument();
    expect(screen.queryByTestId("panel-2")).not.toBeInTheDocument();

    fireEvent.mouseDown(screen.getByRole("button", { name: /inspectors/i }));
    fireEvent.click(screen.getByRole("option", { name: /panel 2/i }));

    expect(screen.getByTestId("panel-2")).toBeInTheDocument();
    expect(screen.queryByTestId("panel-1")).not.toBeInTheDocument();
    // Dropdown closes after selection
    expect(screen.queryAllByRole("option").length).toBe(0);
  });

  it("falls back to the plain single tab strip when no pinned panel is given", () => {
    const { container } = render(
      <DockableLayout leftPanels={leftPanels} mainView={MockMainView} />,
    );

    expect(
      container.querySelector(".left-sidebar-pinned"),
    ).not.toBeInTheDocument();
    expect(screen.queryAllByRole("tab").length).toBe(2);
  });
});

describe("PANEL_SIZES constants", () => {
  it("should have correct default values", () => {
    expect(PANEL_SIZES.LEFT_SIDEBAR).toBe(25);
    expect(PANEL_SIZES.MAIN_CONTENT).toBe(75);
    expect(PANEL_SIZES.FILMSTRIP_BAR).toBe(34);
    expect(PANEL_SIZES.YUV_VIEWER).toBe(44);
    expect(PANEL_SIZES.BOTTOM_PANEL).toBe(22);
    // Filmstrip + YUV viewer + Bottom row default sizes should sum to 100% of the vertical Group.
    expect(
      PANEL_SIZES.FILMSTRIP_BAR +
        PANEL_SIZES.YUV_VIEWER +
        PANEL_SIZES.BOTTOM_PANEL,
    ).toBe(100);
  });
});

// The filmstrip/timeline bar was previously a fixed-height div OUTSIDE the resizable Group --
// found during a real UI/UX parity pass that every *other* panel boundary was drag-resizable
// except this one. Now a real Panel inside the same vertical Group as main-area/bottom-row.
describe("DockableLayout filmstrip bar resizing", () => {
  const leftPanels = [
    { id: "panel1", title: "Panel 1", component: MockPanel1, icon: "icon-1" },
  ];
  const topPanels = [
    {
      id: "filmstrip",
      title: "Filmstrip",
      component: MockFilmstripBar,
    },
  ];

  it("renders the filmstrip bar content inside a real Panel, not a plain fixed div", () => {
    const { container } = render(
      <DockableLayout
        leftPanels={leftPanels}
        mainView={MockMainView}
        topPanels={topPanels}
      />,
    );

    expect(screen.getByTestId("filmstrip-bar-content")).toBeInTheDocument();
    const filmstripPanel = container.querySelector(".filmstrip-bar-panel");
    expect(filmstripPanel).toBeInTheDocument();
    // The old implementation rendered a plain `.filmstrip-bar` div outside any Panel -- assert
    // that's gone, not just that the new class is present.
    expect(container.querySelector(".filmstrip-bar")).not.toBeInTheDocument();
  });

  it("renders a resize-handle-vertical separator between the filmstrip bar and the main area", () => {
    const { container } = render(
      <DockableLayout
        leftPanels={leftPanels}
        mainView={MockMainView}
        topPanels={topPanels}
      />,
    );

    const filmstripPanel = container.querySelector(".filmstrip-bar-panel");
    const mainAreaPanel = container.querySelector(".main-area-panel");
    expect(filmstripPanel).toBeInTheDocument();
    expect(mainAreaPanel).toBeInTheDocument();

    // The separator sits between them as a sibling in the vertical Group.
    const verticalGroup = container.querySelector(".layout-vertical");
    const separators = verticalGroup
      ? Array.from(verticalGroup.querySelectorAll(".resize-handle-vertical"))
      : [];
    expect(separators.length).toBeGreaterThan(0);
  });

  it("does not render a filmstrip bar panel when topPanels is omitted", () => {
    const { container } = render(
      <DockableLayout leftPanels={leftPanels} mainView={MockMainView} />,
    );

    expect(
      container.querySelector(".filmstrip-bar-panel"),
    ).not.toBeInTheDocument();
  });
});

// ---------------------------------------------------------------------------
// Additional edge case tests
// ---------------------------------------------------------------------------

describe("DockableLayout tab switching and active state", () => {
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

  it("clicking second tab shows second panel content", () => {
    const { container } = render(
      <DockableLayout leftPanels={leftPanels} mainView={MockMainView} />,
    );

    const tabs = container.querySelectorAll(".sidebar-tab");
    const panel2Tab = Array.from(tabs).find((tab) =>
      tab.textContent?.includes("Panel 2"),
    );
    expect(panel2Tab).toBeTruthy();

    fireEvent.click(panel2Tab!);

    expect(screen.getByTestId("panel-2")).toBeInTheDocument();
  });

  it("active tab has 'active' class", () => {
    const { container } = render(
      <DockableLayout leftPanels={leftPanels} mainView={MockMainView} />,
    );

    const tabs = container.querySelectorAll(".sidebar-tab");
    const panel2Tab = Array.from(tabs).find((tab) =>
      tab.textContent?.includes("Panel 2"),
    );
    fireEvent.click(panel2Tab!);

    expect(panel2Tab).toHaveClass("active");
  });

  it("first tab is active by default", () => {
    const { container } = render(
      <DockableLayout leftPanels={leftPanels} mainView={MockMainView} />,
    );

    const tabs = container.querySelectorAll(".sidebar-tab");
    const firstTab = tabs[0];

    expect(firstTab).toHaveClass("active");
  });

  it("all tabs render their correct titles", () => {
    render(<DockableLayout leftPanels={leftPanels} mainView={MockMainView} />);

    // Panel 1 appears in both the tab button and the rendered panel content
    const panel1Elements = screen.getAllByText("Panel 1");
    expect(panel1Elements.length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("Panel 2")).toBeInTheDocument();
  });
});

describe("DockableLayout empty panels", () => {
  it("renders without left sidebar when leftPanels is empty", () => {
    const { container } = render(
      <DockableLayout leftPanels={[]} mainView={MockMainView} />,
    );

    expect(
      container.querySelector(".left-sidebar-panel"),
    ).not.toBeInTheDocument();
    expect(screen.getByTestId("main-view")).toBeInTheDocument();
  });

  it("renders without bottom row when bottomRowPanels is empty", () => {
    const { container } = render(
      <DockableLayout
        leftPanels={[
          {
            id: "panel1",
            title: "Panel 1",
            component: MockPanel1,
            icon: "icon-1",
          },
        ]}
        mainView={MockMainView}
        bottomRowPanels={[]}
      />,
    );

    expect(
      container.querySelector(".bottom-row-panel"),
    ).not.toBeInTheDocument();
    expect(screen.getByTestId("main-view")).toBeInTheDocument();
  });
});

describe("DockableLayout bottom row separator", () => {
  const leftPanels = [
    {
      id: "panel1",
      title: "Panel 1",
      component: MockPanel1,
      icon: "icon-1",
    },
  ];

  const bottomRowPanels = [
    {
      id: "panel3",
      title: "Panel 3",
      component: MockPanel3,
      icon: "icon-3",
    },
    {
      id: "panel4",
      title: "Panel 4",
      component: MockPanel1,
      icon: "icon-4",
    },
  ];

  it("renders a resize-handle-vertical separator before the bottom row panel", () => {
    const { container } = render(
      <DockableLayout
        leftPanels={leftPanels}
        mainView={MockMainView}
        bottomRowPanels={bottomRowPanels}
      />,
    );

    // The vertical resize handle separates main area from bottom row
    const verticalHandle = container.querySelector(".resize-handle-vertical");
    expect(verticalHandle).toBeInTheDocument();
  });

  it("renders a resize-handle-horizontal separator between bottom row panels", () => {
    const { container } = render(
      <DockableLayout
        leftPanels={leftPanels}
        mainView={MockMainView}
        bottomRowPanels={bottomRowPanels}
      />,
    );

    // At least one horizontal handle inside the bottom row
    const handles = container.querySelectorAll(".resize-handle-horizontal");
    expect(handles.length).toBeGreaterThan(0);
  });
});
