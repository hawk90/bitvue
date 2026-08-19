/**
 * Dockable Panel Layout
 *
 * panel layout with resizable panels
 *
 * Layout structure:
 * ┌─────────────────────────────────────────────────────────────────────────┐
 * │ Menu Bar                                                                  │
 * ├─────────────────────────────────────────────────────────────────────────┤
 * │ Filmstrip/Timeline Area                                                   │
 * ├──────────┬──────────────────────────────────────────────────────────────┤
 * │  Left    │  Main View Area (YUV Player)                                  │
 * │  Panel   │                                                              │
 * │  (Tabs)  │                                                              │
 * │          │                                                              │
 * ├──────────┴──────────────────────────────────────────────────────────────┤
 * │ Panel 1  │  Panel 2  │  Panel 3                                          │
 * └──────────┴───────────────┴──────────────────────────────────────────────┘
 */

import React, { memo, useState, useCallback, useRef, useEffect } from "react";
import { Group, Panel, Separator } from "react-resizable-panels";
import "./DockableLayout.css";

/**
 * Panel size constants (in percentages)
 */
export const PANEL_SIZES = {
  /** Left sidebar default width percentage */
  LEFT_SIDEBAR: 25,
  /** Main content area width percentage (calculated) */
  MAIN_CONTENT: 75,
  /** YUV viewer height percentage */
  YUV_VIEWER: 78,
  /** Bottom panel default height percentage -- 15 left Info/Details/Stats/Diagnostics' real
      content (e.g. Info's File/Frames/Duration rows) clipped by the window edge on any window
      close to the 1280x800 default, with no visible scroll affordance to signal there was more
      below. 22 fits that real content without scrolling in the common case. */
  BOTTOM_PANEL: 22,
} as const;

/**
 * Panel minimum size constraints (in percentages)
 */
export const PANEL_MIN_SIZES = {
  LEFT_SIDEBAR: 15,
  MAIN_CONTENT: 30,
  YUV_VIEWER: 20,
  /** Same reasoning as `PANEL_SIZES.BOTTOM_PANEL` -- 10 let a user-driven resize shrink this to a
      near-unusable sliver. */
  BOTTOM_PANEL: 15,
} as const;

/**
 * Panel configuration with proper type safety
 * The component prop is typed to accept no props or an empty object
 * to maintain type safety while allowing flexible panel components
 */
export interface PanelConfig<TProps = Record<string, never>> {
  id: string;
  title: string;
  component: React.ComponentType<TProps>;
  icon?: string;
  defaultSize?: number;
  minSize?: number;
  collapsible?: boolean;
}

/**
 * Default panel config for components that don't require props
 */
export type DefaultPanelConfig = PanelConfig<Record<string, never>>;

interface DockableLayoutProps {
  /** Always-visible panel pinned above the left sidebar's tabs (e.g. Stream Tree) -- see
   * LeftSidebar's doc for why this is separate from `leftPanels` rather than just another tab. */
  pinnedLeftPanel?: PanelConfig;
  /** Left sidebar tabbed panels */
  leftPanels: PanelConfig[];
  /** Main view component (YUV Viewer) */
  mainView: React.ComponentType;
  /** Top panels (Filmstrip/Timeline) */
  topPanels?: PanelConfig[];
  /** Bottom row panels (3 panels below main view) */
  bottomRowPanels?: PanelConfig[];
  /** Default sizes (percentage) */
  defaultLeftSize?: number;
}

// ---------------------------------------------------------------------------
// Shared tabbed container — used by both LeftSidebar and BottomPanelBar
// ---------------------------------------------------------------------------

interface TabbedPanelContainerProps {
  panels: PanelConfig[];
  /** CSS class for the tabs wrapper element */
  tabsClassName: string;
  /** CSS class for each individual tab button */
  tabClassName: string;
  /** CSS class for the active-tab modifier (appended when active) */
  activeTabClassName: string;
  /** CSS class for the panel content area */
  contentClassName: string;
  /** Accessible label for the tablist (defaults to "Panels") */
  ariaLabel?: string;
  /** "list" (default): one always-visible button per panel, current behavior. "dropdown": a
   * single trigger button showing the active panel that opens a menu to switch -- for panel
   * groups where showing every option at once (a vertical list of N buttons) costs more space
   * than it's worth, e.g. the left sidebar's 4 Inspector views. */
  variant?: "list" | "dropdown";
}

const TabbedPanelContainer = memo(function TabbedPanelContainer({
  panels,
  tabsClassName,
  tabClassName,
  activeTabClassName,
  contentClassName,
  ariaLabel,
  variant = "list",
}: TabbedPanelContainerProps) {
  const [activeTab, setActiveTab] = useState(panels[0]?.id ?? "");
  const [dropdownOpen, setDropdownOpen] = useState(false);
  const tabListRef = useRef<HTMLDivElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);

  const activePanelConfig = panels.find((p) => p.id === activeTab);
  const ActivePanel = activePanelConfig?.component;

  const handleTabClick = useCallback((panelId: string) => {
    setActiveTab(panelId);
  }, []);

  useEffect(() => {
    if (variant !== "dropdown" || !dropdownOpen) return;

    const handleClickOutside = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      if (dropdownRef.current && !dropdownRef.current.contains(target)) {
        setDropdownOpen(false);
      }
    };

    document.addEventListener("click", handleClickOutside);
    return () => document.removeEventListener("click", handleClickOutside);
  }, [variant, dropdownOpen]);

  const handleTabKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLButtonElement>) => {
      const currentIndex = panels.findIndex((p) => p.id === activeTab);
      if (currentIndex === -1) return;

      let nextIndex: number | null = null;
      if (e.key === "ArrowRight" || e.key === "ArrowDown") {
        nextIndex = (currentIndex + 1) % panels.length;
      } else if (e.key === "ArrowLeft" || e.key === "ArrowUp") {
        nextIndex = (currentIndex - 1 + panels.length) % panels.length;
      } else if (e.key === "Home") {
        nextIndex = 0;
      } else if (e.key === "End") {
        nextIndex = panels.length - 1;
      }

      if (nextIndex !== null) {
        e.preventDefault();
        const nextPanel = panels[nextIndex];
        setActiveTab(nextPanel.id);
        // Move focus to the newly activated tab button
        const tabButtons =
          tabListRef.current?.querySelectorAll<HTMLButtonElement>(
            '[role="tab"]',
          );
        tabButtons?.[nextIndex]?.focus();
      }
    },
    [activeTab, panels],
  );

  if (variant === "dropdown") {
    return (
      <>
        <div ref={dropdownRef} className={tabsClassName}>
          <button
            className={tabClassName}
            onMouseDown={() => setDropdownOpen((prev) => !prev)}
            aria-haspopup="listbox"
            aria-expanded={dropdownOpen}
            aria-label={ariaLabel ?? "Panels"}
          >
            {activePanelConfig?.icon && (
              <span
                className={`codicon codicon-${activePanelConfig.icon}`}
                aria-hidden="true"
              ></span>
            )}
            <span>{activePanelConfig?.title}</span>
            <span
              className="codicon codicon-chevron-down"
              aria-hidden="true"
            ></span>
          </button>
          {dropdownOpen && (
            <ul
              className={`${tabsClassName}-menu`}
              role="listbox"
              aria-label={ariaLabel ?? "Panels"}
            >
              {panels.map((panel) => (
                <li key={panel.id}>
                  <button
                    role="option"
                    aria-selected={activeTab === panel.id}
                    className={activeTab === panel.id ? activeTabClassName : ""}
                    onClick={() => {
                      handleTabClick(panel.id);
                      setDropdownOpen(false);
                    }}
                  >
                    {panel.icon && (
                      <span
                        className={`codicon codicon-${panel.icon}`}
                        aria-hidden="true"
                      ></span>
                    )}
                    <span>{panel.title}</span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
        <div
          role="tabpanel"
          id={`tabpanel-${activeTab}`}
          className={contentClassName}
        >
          {ActivePanel && <ActivePanel />}
        </div>
      </>
    );
  }

  return (
    <>
      <div
        ref={tabListRef}
        role="tablist"
        aria-label={ariaLabel ?? "Panels"}
        className={tabsClassName}
      >
        {panels.map((panel) => (
          <button
            key={panel.id}
            role="tab"
            aria-selected={activeTab === panel.id}
            aria-controls={`tabpanel-${panel.id}`}
            id={`tab-${panel.id}`}
            tabIndex={activeTab === panel.id ? 0 : -1}
            className={`${tabClassName} ${activeTab === panel.id ? activeTabClassName : ""}`}
            onClick={() => handleTabClick(panel.id)}
            onKeyDown={handleTabKeyDown}
          >
            {panel.icon && (
              <span
                className={`codicon codicon-${panel.icon}`}
                aria-hidden="true"
              ></span>
            )}
            <span>{panel.title}</span>
          </button>
        ))}
      </div>
      <div
        role="tabpanel"
        id={`tabpanel-${activeTab}`}
        aria-labelledby={`tab-${activeTab}`}
        className={contentClassName}
      >
        {ActivePanel && <ActivePanel />}
      </div>
    </>
  );
});

// ---------------------------------------------------------------------------
// DockableLayout
// ---------------------------------------------------------------------------

export const DockableLayout = memo(function DockableLayout({
  pinnedLeftPanel,
  leftPanels,
  mainView: MainView,
  topPanels,
  bottomRowPanels,
  defaultLeftSize = PANEL_SIZES.LEFT_SIDEBAR,
}: DockableLayoutProps) {
  // Resolve top panel component before JSX — JSX requires PascalCase variable for dynamic components
  const FilmstripBar =
    topPanels?.length === 1 && topPanels[0].id === "filmstrip"
      ? topPanels[0].component
      : null;

  return (
    <div className="dockable-layout" data-testid="dockable-layout">
      {/* Filmstrip/Timeline fixed bar — outside Group so height is CSS-controlled, not draggable */}
      {topPanels && topPanels.length > 0 && (
        <div className="filmstrip-bar">
          {FilmstripBar ? (
            <FilmstripBar />
          ) : (
            <BottomPanelBar panels={topPanels} />
          )}
        </div>
      )}

      <Group orientation="vertical" className="layout-vertical">
        {/* Main content area with left sidebar */}
        <Panel
          defaultSize={PANEL_SIZES.YUV_VIEWER}
          minSize={PANEL_MIN_SIZES.YUV_VIEWER}
          id="main-area"
          className="main-area-panel"
        >
          <Group orientation="horizontal" className="layout-horizontal">
            {/* Left Sidebar Panel */}
            {leftPanels && leftPanels.length > 0 && (
              <>
                <Panel
                  defaultSize={defaultLeftSize}
                  minSize={PANEL_MIN_SIZES.LEFT_SIDEBAR}
                  collapsible={true}
                  id="left-sidebar"
                  className="left-sidebar-panel"
                >
                  <LeftSidebar
                    pinnedPanel={pinnedLeftPanel}
                    panels={leftPanels}
                  />
                </Panel>
                <Separator className="resize-handle-horizontal" />
              </>
            )}

            {/* YUV Viewer / Main View */}
            <Panel
              defaultSize={PANEL_SIZES.MAIN_CONTENT}
              minSize={PANEL_MIN_SIZES.MAIN_CONTENT}
              id="yuv-viewer"
              className="yuv-viewer-panel"
            >
              <MainView />
            </Panel>
          </Group>
        </Panel>

        {/* Bottom row panels (3 panels below main view) */}
        {bottomRowPanels && bottomRowPanels.length > 0 && (
          <>
            <Separator className="resize-handle-vertical" />
            <Panel
              defaultSize={PANEL_SIZES.BOTTOM_PANEL}
              minSize={PANEL_MIN_SIZES.BOTTOM_PANEL}
              collapsible={true}
              id="bottom-row"
              className="bottom-row-panel"
            >
              <BottomRowPanelBar panels={bottomRowPanels} />
            </Panel>
          </>
        )}
      </Group>
    </div>
  );
});

// ---------------------------------------------------------------------------
// Left Sidebar: pinned Stream Tree (always visible, per UX_PARITY_MATRIX.md's W0 wireframe --
// Tree and Inspectors are separate simultaneous regions there, not one tab strip) + tabbed
// Inspectors panels below it (Syntax | Selection | Unit HEX | YUV Diff).
// ---------------------------------------------------------------------------
const LeftSidebar = memo(function LeftSidebar({
  pinnedPanel,
  panels,
}: {
  pinnedPanel?: PanelConfig;
  panels: PanelConfig[];
}) {
  if (!pinnedPanel) {
    return (
      <div className="left-sidebar">
        <TabbedPanelContainer
          panels={panels}
          tabsClassName="sidebar-tabs"
          tabClassName="sidebar-tab"
          activeTabClassName="active"
          contentClassName="sidebar-content"
        />
      </div>
    );
  }

  const PinnedComponent = pinnedPanel.component;

  return (
    <div className="left-sidebar">
      <Group orientation="vertical" className="left-sidebar-split">
        <Panel
          defaultSize={40}
          minSize={15}
          collapsible={true}
          id="left-sidebar-pinned"
          className="left-sidebar-pinned"
        >
          <div className="left-sidebar-pinned-header">
            {pinnedPanel.icon && (
              <span
                className={`codicon codicon-${pinnedPanel.icon}`}
                aria-hidden="true"
              ></span>
            )}
            <span>{pinnedPanel.title}</span>
          </div>
          <div className="left-sidebar-pinned-content">
            <PinnedComponent />
          </div>
        </Panel>
        <Separator className="resize-handle-vertical" />
        <Panel
          defaultSize={60}
          minSize={20}
          collapsible={true}
          id="left-sidebar-inspectors"
          className="left-sidebar-inspectors"
        >
          <TabbedPanelContainer
            panels={panels}
            variant="dropdown"
            tabsClassName="sidebar-inspectors-dropdown"
            tabClassName="sidebar-inspectors-dropdown-trigger"
            activeTabClassName="active"
            contentClassName="sidebar-content"
            ariaLabel="Inspectors"
          />
        </Panel>
      </Group>
    </div>
  );
});

// ---------------------------------------------------------------------------
// Bottom Panel Bar (Filmstrip/Timeline)
// Filmstrip with view mode selector
// ---------------------------------------------------------------------------
const BottomPanelBar = memo(function BottomPanelBar({
  panels,
}: {
  panels: PanelConfig[];
}) {
  return (
    <div className="bottom-panel-bar">
      <TabbedPanelContainer
        panels={panels}
        tabsClassName="bottom-panel-tabs"
        tabClassName="bottom-panel-tab"
        activeTabClassName="active"
        contentClassName="bottom-panel-content"
      />
    </div>
  );
});

// ---------------------------------------------------------------------------
// Bottom Row Panel Bar (3 panels displayed horizontally)
// Shows all panels side by side with resize capability
// ---------------------------------------------------------------------------
const BottomRowPanelBar = memo(function BottomRowPanelBar({
  panels,
}: {
  panels: PanelConfig[];
}) {
  return (
    <div className="bottom-row-panel-bar">
      <Group orientation="horizontal" className="bottom-row-layout">
        {panels.map((panel, index) => (
          <React.Fragment key={panel.id}>
            {index > 0 && <Separator className="resize-handle-horizontal" />}
            <Panel
              defaultSize={panel.defaultSize || 33}
              minSize={panel.minSize || 10}
              collapsible={panel.collapsible}
              id={`bottom-row-${panel.id}`}
              className="bottom-row-item"
            >
              <div className="bottom-row-item-content">
                <div className="bottom-row-item-header">
                  {panel.icon && (
                    <span className={`codicon codicon-${panel.icon}`}></span>
                  )}
                  <span>{panel.title}</span>
                </div>
                <div className="bottom-row-item-body">
                  <panel.component />
                </div>
              </div>
            </Panel>
          </React.Fragment>
        ))}
      </Group>
    </div>
  );
});
