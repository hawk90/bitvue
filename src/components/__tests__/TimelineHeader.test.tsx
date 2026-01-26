/**
 * TimelineHeader Component Tests
 */

import { describe, it, expect } from 'vitest';
import { render, screen } from '@/test/test-utils';
import { TimelineHeader } from '@/components/TimelineHeader';

describe('TimelineHeader', () => {
  it('should render timeline title', () => {
    render(<TimelineHeader currentFrame={42} totalFrames={100} />);

    expect(screen.getByText('Timeline')).toBeInTheDocument();
  });

  it('should render frame count', () => {
    render(<TimelineHeader currentFrame={42} totalFrames={100} />);

    expect(screen.getByText('43 / 100')).toBeInTheDocument();
  });

  it('should display current frame + 1 (0-indexed to 1-indexed)', () => {
    render(<TimelineHeader currentFrame={0} totalFrames={100} />);

    expect(screen.getByText('1 / 100')).toBeInTheDocument();
  });

  it('should handle edge cases', () => {
    const { rerender } = render(
      <TimelineHeader currentFrame={0} totalFrames={1} />
    );

    expect(screen.getByText('1 / 1')).toBeInTheDocument();

    rerender(<TimelineHeader currentFrame={99} totalFrames={100} />);
    expect(screen.getByText('100 / 100')).toBeInTheDocument();
  });

  it('should have graph icon', () => {
    render(<TimelineHeader currentFrame={0} totalFrames={100} />);

    const icon = document.querySelector('.codicon-graph');
    expect(icon).toBeInTheDocument();
  });

  it('should have correct ARIA attributes', () => {
    render(<TimelineHeader currentFrame={42} totalFrames={100} />);

    const info = screen.getByRole('status');
    expect(info).toHaveAttribute('aria-live', 'polite');
  });
});
