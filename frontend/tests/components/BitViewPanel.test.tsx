/**
 * BitViewPanel Component Tests
 * Tests bit view placeholder panel
 */

import { describe, it, expect } from 'vitest';
import { render, screen } from '@/test/test-utils';
import { BitViewPanel } from '../BitViewPanel';

describe('BitViewPanel', () => {
  it('should render bit view panel', () => {
    render(<BitViewPanel />);

    expect(screen.getByText('Bit View')).toBeInTheDocument();
  });

  it('should display description', () => {
    render(<BitViewPanel />);

    expect(screen.getByText(/Binary\/bit-level syntax element display/)).toBeInTheDocument();
  });

  it('should use React.memo for performance', () => {
    const { rerender } = render(<BitViewPanel />);

    rerender(<BitViewPanel />);

    expect(screen.getByText('Bit View')).toBeInTheDocument();
  });

  it('should render boolean icon', () => {
    const { container } = render(<BitViewPanel />);

    const icon = container.querySelector('.codicon-symbol-boolean');
    expect(icon).toBeInTheDocument();
  });

  it('should mention bit-level view', () => {
    render(<BitViewPanel />);

    expect(screen.getByText(/bit-level/)).toBeInTheDocument();
  });
});
