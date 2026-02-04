/**
 * TabContainer Component Tests
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@/test/test-utils';
import { TabContainer, TabContent, TabsWithContent } from '../TabContainer';

describe('TabContainer', () => {
  const mockTabs = [
    { id: 'tab1' as const, label: 'Tab 1', icon: 'codicon-home' },
    { id: 'tab2' as const, label: 'Tab 2', icon: 'codicon-list-tree' },
    { id: 'tab3' as const, label: 'Tab 3', icon: 'codicon-search' },
  ];

  it('should render all tabs', () => {
    render(
      <TabContainer
        tabs={mockTabs}
        activeTab="tab1"
        onTabChange={vi.fn()}
      />
    );

    expect(screen.getByText('Tab 1')).toBeInTheDocument();
    expect(screen.getByText('Tab 2')).toBeInTheDocument();
    expect(screen.getByText('Tab 3')).toBeInTheDocument();
  });

  it('should mark active tab', () => {
    render(
      <TabContainer
        tabs={mockTabs}
        activeTab="tab2"
        onTabChange={vi.fn()}
      />
    );

    const activeTab = screen.getByRole('tab', { selected: true });
    expect(activeTab).toHaveTextContent('Tab 2');
  });

  it('should call onTabChange when tab is clicked', () => {
    const handleChange = vi.fn();
    render(
      <TabContainer
        tabs={mockTabs}
        activeTab="tab1"
        onTabChange={handleChange}
      />
    );

    const tab2 = screen.getByRole('tab', { name: /Tab 2/ });
    tab2.click();

    expect(handleChange).toHaveBeenCalledWith('tab2');
  });

  it('should render tab icons', () => {
    render(
      <TabContainer
        tabs={mockTabs}
        activeTab="tab1"
        onTabChange={vi.fn()}
      />
    );

    expect(screen.getByText('Tab 1')).toBeInTheDocument();
    // Check that icons are present via the codicon class
    expect(document.querySelector('.codicon-home')).toBeInTheDocument();
  });

  it('should hide icons when showIcons is false', () => {
    render(
      <TabContainer
        tabs={mockTabs}
        activeTab="tab1"
        onTabChange={vi.fn()}
        showIcons={false}
      />
    );

    expect(document.querySelectorAll('.tab-icon').length).toBe(0);
  });

  it('should render tab badges', () => {
    const tabsWithBadges = [
      ...mockTabs,
      { id: 'tab4' as const, label: 'Tab 4', badge: 5 },
    ];

    render(
      <TabContainer
        tabs={tabsWithBadges}
        activeTab="tab1"
        onTabChange={vi.fn()}
      />
    );

    expect(screen.getByText('5')).toBeInTheDocument();
  });

  it('should disable tab when disabled is true', () => {
    const tabsWithDisabled = [
      { id: 'tab1' as const, label: 'Tab 1' },
      { id: 'tab2' as const, label: 'Tab 2', disabled: true },
    ];

    render(
      <TabContainer
        tabs={tabsWithDisabled}
        activeTab="tab1"
        onTabChange={vi.fn()}
      />
    );

    const disabledTab = screen.getByRole('tab', { name: /Tab 2/ });
    expect(disabledTab).toBeDisabled();
    expect(disabledTab).toHaveClass('tab-disabled');
  });

  it('should apply variant classes', () => {
    const { container } = render(
      <TabContainer
        tabs={mockTabs}
        activeTab="tab1"
        onTabChange={vi.fn()}
        variant="compact"
      />
    );

    expect(container.querySelector('.tab-container-compact')).toBeInTheDocument();
  });

  it('should apply position classes', () => {
    const { container: containerLeft } = render(
      <TabContainer
        tabs={mockTabs}
        activeTab="tab1"
        onTabChange={vi.fn()}
        position="left"
      />
    );

    expect(containerLeft.querySelector('.tab-container-left')).toBeInTheDocument();
  });

  it('should apply custom className', () => {
    const { container } = render(
      <TabContainer
        tabs={mockTabs}
        activeTab="tab1"
        onTabChange={vi.fn()}
        className="custom-class"
      />
    );

    const tabContainer = container.querySelector('.tab-container');
    expect(tabContainer).toHaveClass('custom-class');
  });

  it('should not render tabs when empty array', () => {
    const { container } = render(
      <TabContainer
        tabs={[]}
        activeTab=""
        onTabChange={vi.fn()}
      />
    );

    expect(container.querySelectorAll('.tab-button').length).toBe(0);
  });

  it('should set correct aria attributes', () => {
    render(
      <TabContainer
        tabs={mockTabs}
        activeTab="tab2"
        onTabChange={vi.fn()}
      />
    );

    const activeTab = screen.getByRole('tab', { selected: true });
    expect(activeTab).toHaveAttribute('aria-selected', 'true');

    const inactiveTabs = screen.getAllByRole('tab', { selected: false });
    expect(inactiveTabs.length).toBe(2);
  });

  it('should set tabIndex 0 for active tab', () => {
    render(
      <TabContainer
        tabs={mockTabs}
        activeTab="tab2"
        onTabChange={vi.fn()}
      />
    );

    const activeTab = screen.getByRole('tab', { selected: true });
    expect(activeTab).toHaveAttribute('tabIndex', '0');
  });

  it('should set tabIndex -1 for inactive tabs', () => {
    render(
      <TabContainer
        tabs={mockTabs}
        activeTab="tab2"
        onTabChange={vi.fn()}
      />
    );

    const inactiveTabs = screen.getAllByRole('tab', { selected: false });
    inactiveTabs.forEach(tab => {
      expect(tab).toHaveAttribute('tabIndex', '-1');
    });
  });
});

describe('TabContent', () => {
  it('should render content when active', () => {
    render(
      <TabContent tabId="tab1" isActive>
        <div>Tab Content</div>
      </TabContent>
    );

    expect(screen.getByText('Tab Content')).toBeInTheDocument();
  });

  it('should not render content when inactive', () => {
    render(
      <TabContent tabId="tab1" isActive={false}>
        <div>Tab Content</div>
      </TabContent>
    );

    expect(screen.queryByText('Tab Content')).not.toBeInTheDocument();
  });

  it('should apply role and id attributes', () => {
    render(
      <TabContent tabId="tab1" isActive>
        <div>Content</div>
      </TabContent>
    );

    const content = document.querySelector('[role="tabpanel"]');
    expect(content).toHaveAttribute('id', 'panel-tab1');
    expect(content).toHaveAttribute('aria-labelledby', 'tab-tab1');
  });

  it('should apply custom className', () => {
    const { container } = render(
      <TabContent tabId="tab1" isActive className="custom-content">
        <div>Content</div>
      </TabContent>
    );

    expect(container.querySelector('.custom-content')).toBeInTheDocument();
  });
});

describe('TabsWithContent', () => {
  const tabsWithContent = [
    {
      id: 'tab1' as const,
      label: 'Tab 1',
      icon: 'codicon-home',
      content: <div>Content 1</div>,
    },
    {
      id: 'tab2' as const,
      label: 'Tab 2',
      icon: 'codicon-list-tree',
      content: <div>Content 2</div>,
    },
    {
      id: 'tab3' as const,
      label: 'Tab 3',
      icon: 'codicon-search',
      content: <div>Content 3</div>,
    },
  ];

  it('should render tabs and content', () => {
    render(
      <TabsWithContent
        tabs={tabsWithContent}
        activeTab="tab2"
        onTabChange={vi.fn()}
      />
    );

    expect(screen.getByText('Tab 1')).toBeInTheDocument();
    expect(screen.getByText('Tab 2')).toBeInTheDocument();
    expect(screen.getByText('Tab 3')).toBeInTheDocument();

    expect(screen.getByText('Content 2')).toBeInTheDocument();
  });

  it('should only render active tab content', () => {
    render(
      <TabsWithContent
        tabs={tabsWithContent}
        activeTab="tab1"
        onTabChange={vi.fn()}
      />
    );

    expect(screen.getByText('Content 1')).toBeInTheDocument();
    expect(screen.queryByText('Content 2')).not.toBeInTheDocument();
    expect(screen.queryByText('Content 3')).not.toBeInTheDocument();
  });

  it('should call onTabChange when tab is clicked', () => {
    const handleChange = vi.fn();
    render(
      <TabsWithContent
        tabs={tabsWithContent}
        activeTab="tab1"
        onTabChange={handleChange}
      />
    );

    const tab3 = screen.getByRole('tab', { name: /Tab 3/ });
    tab3.click();

    expect(handleChange).toHaveBeenCalledWith('tab3');
  });

  it('should render tabs with badges', () => {
    const tabsWithBadges = tabsWithContent.map(tab => ({
      ...tab,
      badge: tab.id === 'tab1' ? 5 : undefined,
    }));

    render(
      <TabsWithContent
        tabs={tabsWithBadges}
        activeTab="tab1"
        onTabChange={vi.fn()}
      />
    );

    expect(screen.getByText('5')).toBeInTheDocument();
  });

  it('should apply variant to tabs', () => {
    const { container } = render(
      <TabsWithContent
        tabs={tabsWithContent}
        activeTab="tab1"
        onTabChange={vi.fn()}
        variant="pills"
      />
    );

    expect(container.querySelector('.tab-container-pills')).toBeInTheDocument();
  });

  it('should apply position to tabs', () => {
    const { container } = render(
      <TabsWithContent
        tabs={tabsWithContent}
        activeTab="tab1"
        onTabChange={vi.fn()}
        position="left"
      />
    );

    expect(container.querySelector('.tab-container-left')).toBeInTheDocument();
  });
});
