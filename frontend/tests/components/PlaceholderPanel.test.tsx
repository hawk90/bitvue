/**
 * PlaceholderPanel Component Tests
 * Tests placeholder panel for unimplemented features
 */

import { describe, it, expect } from 'vitest';
import { render, screen } from '@/test/test-utils';
import { PlaceholderPanel } from '@/components/panels/PlaceholderPanel';

describe('PlaceholderPanel', () => {
  it('should render placeholder with title', () => {
    render(
      <PlaceholderPanel
        title="Test Panel"
        description="Test description"
        icon="test-icon"
      />
    );

    expect(screen.getByText('Test Panel')).toBeInTheDocument();
  });

  it('should render description', () => {
    render(
      <PlaceholderPanel
        title="Test Panel"
        description="This is a test panel"
        icon="test"
      />
    );

    expect(screen.getByText('This is a test panel')).toBeInTheDocument();
  });

  it('should render icon', () => {
    render(
      <PlaceholderPanel
        title="Test Panel"
        icon="symbol-boolean"
      />
    );

    const icon = document.querySelector('.codicon-symbol-boolean');
    expect(icon).toBeInTheDocument();
  });

  it('should have correct CSS class', () => {
    const { container } = render(
      <PlaceholderPanel title="Test" />
    );

    expect(container.querySelector('.placeholder-panel')).toBeInTheDocument();
  });

  it('should render without icon', () => {
    render(<PlaceholderPanel title="Test" />);

    expect(screen.getByText('Test')).toBeInTheDocument();
  });

  it('should render without description', () => {
    render(
      <PlaceholderPanel
        title="Test Panel"
        icon="test"
      />
    );

    expect(screen.getByText('Test Panel')).toBeInTheDocument();
  });
});

describe('PlaceholderPanel variants', () => {
  it('should be used for BitViewPanel', () => {
    render(<PlaceholderPanel title="Bit View" description="Binary/bit-level syntax element display" icon="symbol-boolean" />);

    expect(screen.getByText('Bit View')).toBeInTheDocument();
    expect(screen.getByText('Binary/bit-level syntax element display')).toBeInTheDocument();
  });

  it('should be used for other unimplemented panels', () => {
    const { container } = render(
      <PlaceholderPanel
        title="Coming Soon"
        description="This feature is coming soon"
        icon="clock"
      />
    );

    expect(container.querySelector('.codicon-clock')).toBeInTheDocument();
  });
});
