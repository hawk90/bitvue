/**
 * TabContainer Component
 *
 * Generic tab container for consistent tab behavior across the application
 * Replaces duplicate tab implementations in SyntaxDetailPanel, UnitHexPanel, etc.
 */

import { memo, useCallback, MouseEvent } from "react";
import "./TabContainer.css";

export interface TabOption<T extends string> {
  /** Unique identifier for the tab */
  id: T;
  /** Display label for the tab */
  label: string;
  /** Optional icon class name (e.g., 'codicon-list-tree') */
  icon?: string;
  /** Optional badge count or content */
  badge?: number | string;
  /** Whether the tab is disabled */
  disabled?: boolean;
}

export interface TabContainerProps<T extends string> {
  /** Available tab options */
  tabs: TabOption<T>[];
  /** Currently active tab */
  activeTab: T;
  /** Callback when tab is changed */
  onTabChange: (tabId: T) => void;
  /** Optional CSS class name */
  className?: string;
  /** Tab variant style */
  variant?: "default" | "compact" | "pills";
  /** Tab position */
  position?: "top" | "left" | "right" | "bottom";
  /** Whether to show icons */
  showIcons?: boolean;
}

export const TabContainer = memo(function TabContainer<T extends string>({
  tabs,
  activeTab,
  onTabChange,
  className = "",
  variant = "default",
  position = "top",
  showIcons = true,
}: TabContainerProps<T>) {
  const handleTabClick = useCallback(
    (event: MouseEvent<HTMLButtonElement>, tabId: T) => {
      event.preventDefault();
      onTabChange(tabId);
    },
    [onTabChange],
  );

  return (
    <div
      className={`tab-container tab-container-${position} tab-container-${variant} ${className}`.trim()}
    >
      <div className="tab-list" role="tablist">
        {tabs.map((tab) => {
          const isActive = tab.id === activeTab;
          const isDisabled = tab.disabled ?? false;

          return (
            <button
              key={tab.id}
              type="button"
              role="tab"
              aria-selected={isActive}
              aria-disabled={isDisabled}
              disabled={isDisabled}
              className={`tab-button ${isActive ? "tab-active" : ""} ${
                isDisabled ? "tab-disabled" : ""
              }`}
              onClick={(e) => handleTabClick(e, tab.id)}
              tabIndex={isActive ? 0 : -1}
            >
              {tab.icon && showIcons && (
                <span
                  className={`codicon ${tab.icon} tab-icon`}
                  aria-hidden="true"
                />
              )}
              <span className="tab-label">{tab.label}</span>
              {tab.badge !== undefined && (
                <span className="tab-badge" aria-label={`${tab.badge} items`}>
                  {tab.badge}
                </span>
              )}
            </button>
          );
        })}
      </div>
    </div>
  );
}) as <T extends string>(props: TabContainerProps<T>) => JSX.Element;

TabContainer.displayName = "TabContainer";

/**
 * TabContent component for rendering tab panel content
 */
export interface TabContentProps {
  /** ID of the tab this content belongs to */
  tabId: string;
  /** Whether this tab is currently active */
  isActive: boolean;
  /** Optional CSS class name */
  className?: string;
  /** Content to render */
  children: React.ReactNode;
}

export const TabContent = memo(function TabContent({
  tabId,
  isActive,
  className = "",
  children,
}: TabContentProps) {
  if (!isActive) return null;

  return (
    <div
      role="tabpanel"
      id={`panel-${tabId}`}
      aria-labelledby={`tab-${tabId}`}
      className={`tab-content ${className}`.trim()}
    >
      {children}
    </div>
  );
});

TabContent.displayName = "TabContent";

/**
 * Complete tabs container with content
 */
export interface TabsWithContentProps<T extends string> {
  /** Tab configuration with associated content */
  tabs: Array<
    TabOption<T> & {
      content: React.ReactNode;
    }
  >;
  /** Currently active tab */
  activeTab: T;
  /** Callback when tab is changed */
  onTabChange: (tabId: T) => void;
  /** Optional CSS class name for container */
  className?: string;
  /** Tab variant style */
  variant?: "default" | "compact" | "pills";
  /** Tab position */
  position?: "top" | "left" | "right" | "bottom";
}

export function TabsWithContent<T extends string>({
  tabs,
  activeTab,
  onTabChange,
  className,
  variant,
  position,
}: TabsWithContentProps<T>) {
  const tabOptions: TabOption<T>[] = tabs.map((tab) => ({
    id: tab.id,
    label: tab.label,
    icon: tab.icon,
    badge: tab.badge,
    disabled: tab.disabled,
  }));

  return (
    <div className={`tabs-with-content ${className ?? ""}`.trim()}>
      <TabContainer
        tabs={tabOptions}
        activeTab={activeTab}
        onTabChange={onTabChange}
        variant={variant}
        position={position}
      />
      {tabs.map((tab) => (
        <TabContent key={tab.id} tabId={tab.id} isActive={tab.id === activeTab}>
          {tab.content}
        </TabContent>
      ))}
    </div>
  );
}
