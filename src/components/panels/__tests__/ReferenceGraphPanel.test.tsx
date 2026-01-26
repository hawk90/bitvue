/**
 * ReferenceGraphPanel Component Tests
 * Tests reference graph placeholder panel
 */

import { describe, it, expect } from 'vitest';
import { render, screen } from '@/test/test-utils';
import { ReferenceGraphPanel } from '../ReferenceGraphPanel';

describe('ReferenceGraphPanel', () => {
  it('should render reference graph panel', () => {
    render(<ReferenceGraphPanel />);

    expect(screen.getByText('Reference Graph')).toBeInTheDocument();
  });

  it('should display coming soon message', () => {
    render(<ReferenceGraphPanel />);

    expect(screen.getByText(/coming soon/i)).toBeInTheDocument();
  });

  it('should use React.memo for performance', () => {
    const { rerender } = render(<ReferenceGraphPanel />);

    rerender(<ReferenceGraphPanel />);

    expect(screen.getByText('Reference Graph')).toBeInTheDocument();
  });

  it('should render graph icon', () => {
    const { container } = render(<ReferenceGraphPanel />);

    const icon = container.querySelector('.codicon-graph');
    expect(icon).toBeInTheDocument();
  });

  it('should display proper description', () => {
    render(<ReferenceGraphPanel />);

    expect(screen.getByText(/Frame dependency and DPB visualization/)).toBeInTheDocument();
  });
});
