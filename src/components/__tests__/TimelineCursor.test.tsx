/**
 * TimelineCursor Component Tests
 */

import { describe, it, expect } from 'vitest';
import { render } from '@/test/test-utils';
import { TimelineCursor } from '@/components/TimelineCursor';

describe('TimelineCursor', () => {
  it('should render with correct position', () => {
    const { container } = render(
      <TimelineCursor positionPercent={50} frameIndex={42} />
    );

    const cursor = container.querySelector('.timeline-cursor') as HTMLElement;
    expect(cursor).toBeInTheDocument();
    expect(cursor.style.left).toBe('50%');
  });

  it('should have correct title attribute', () => {
    const { container } = render(
      <TimelineCursor positionPercent={25} frameIndex={100} />
    );

    const cursor = container.querySelector('.timeline-cursor');
    expect(cursor).toHaveAttribute('title', 'Frame 100');
  });

  it('should have aria-hidden attribute', () => {
    const { container } = render(
      <TimelineCursor positionPercent={50} frameIndex={0} />
    );

    const cursor = container.querySelector('.timeline-cursor');
    expect(cursor).toHaveAttribute('aria-hidden', 'true');
  });

  it('should handle edge positions', () => {
    const { container: startContainer } = render(
      <TimelineCursor positionPercent={0} frameIndex={0} />
    );
    const { container: endContainer } = render(
      <TimelineCursor positionPercent={100} frameIndex={999} />
    );

    const startCursor = startContainer.querySelector('.timeline-cursor') as HTMLElement;
    const endCursor = endContainer.querySelector('.timeline-cursor') as HTMLElement;

    expect(startCursor.style.left).toBe('0%');
    expect(endCursor.style.left).toBe('100%');
  });
});
