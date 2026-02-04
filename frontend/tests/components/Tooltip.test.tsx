/**
 * Tooltip Component Tests
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@/test/test-utils';
import { Tooltip } from '../Tooltip';

describe('Tooltip', () => {
  beforeEach(() => {
    vi.useRealTimers();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should render children with tooltip', () => {
    render(
      <Tooltip content="Tooltip content">
        <button>Hover me</button>
      </Tooltip>
    );

    expect(screen.getByText('Hover me')).toBeInTheDocument();
  });

  it('should show tooltip after default delay', async () => {
    render(
      <Tooltip content="Tooltip content">
        <button>Hover me</button>
      </Tooltip>
    );

    const button = screen.getByText('Hover me');
    fireEvent.mouseEnter(button);

    // Should not show immediately
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();

    // Wait for tooltip to appear after default delay (300ms)
    await waitFor(() => {
      expect(screen.queryByRole('tooltip')).toBeInTheDocument();
    }, { timeout: 500 });

    expect(screen.getByText('Tooltip content')).toBeInTheDocument();
  });

  it('should show tooltip with custom delay', async () => {
    render(
      <Tooltip content="Tooltip content" delay={500}>
        <button>Hover me</button>
      </Tooltip>
    );

    const button = screen.getByText('Hover me');
    fireEvent.mouseEnter(button);

    // Should not show before delay
    await waitFor(() => {
      expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
    }, { timeout: 200 });

    // Wait for tooltip to appear after custom delay (500ms)
    await waitFor(() => {
      expect(screen.queryByRole('tooltip')).toBeInTheDocument();
    }, { timeout: 700 });
  });

  it('should hide tooltip on mouse leave', async () => {
    render(
      <Tooltip content="Tooltip content">
        <button>Hover me</button>
      </Tooltip>
    );

    const button = screen.getByText('Hover me');
    fireEvent.mouseEnter(button);

    // Wait for tooltip to appear
    await waitFor(() => {
      expect(screen.queryByRole('tooltip')).toBeInTheDocument();
    }, { timeout: 500 });

    fireEvent.mouseLeave(button);
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
  });

  it('should not show tooltip when disabled', async () => {
    render(
      <Tooltip content="Tooltip content" disabled>
        <button>Hover me</button>
      </Tooltip>
    );

    const button = screen.getByText('Hover me');
    fireEvent.mouseEnter(button);

    // Wait enough time for tooltip to appear if it wasn't disabled
    await waitFor(() => {
      expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
    }, { timeout: 500 });
  });

  it('should show tooltip on focus', async () => {
    render(
      <Tooltip content="Tooltip content">
        <button>Hover me</button>
      </Tooltip>
    );

    const button = screen.getByText('Hover me') as HTMLButtonElement;
    fireEvent.focus(button);

    await waitFor(() => {
      expect(screen.queryByRole('tooltip')).toBeInTheDocument();
    }, { timeout: 500 });
  });

  it('should hide tooltip on blur', async () => {
    render(
      <Tooltip content="Tooltip content">
        <button>Hover me</button>
      </Tooltip>
    );

    const button = screen.getByText('Hover me') as HTMLButtonElement;
    fireEvent.focus(button);

    await waitFor(() => {
      expect(screen.queryByRole('tooltip')).toBeInTheDocument();
    }, { timeout: 500 });

    fireEvent.blur(button);
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
  });

  it('should apply correct placement class', async () => {
    render(
      <Tooltip content="Tooltip content" placement="top">
        <button>Hover me</button>
      </Tooltip>
    );

    const button = screen.getByText('Hover me');
    fireEvent.mouseEnter(button);

    await waitFor(() => {
      const tooltip = screen.getByRole('tooltip');
      expect(tooltip).toHaveClass('tooltip-top');
    }, { timeout: 500 });
  });

  it('should render arrow element', async () => {
    render(
      <Tooltip content="Tooltip content" placement="top">
        <button>Hover me</button>
      </Tooltip>
    );

    const button = screen.getByText('Hover me');
    fireEvent.mouseEnter(button);

    await waitFor(() => {
      const arrow = document.querySelector('.tooltip-arrow');
      expect(arrow).toBeInTheDocument();
    }, { timeout: 500 });
  });

  it('should handle different placements', async () => {
    const placements: Array<'top' | 'bottom' | 'left' | 'right'> = ['top', 'bottom', 'left', 'right'];

    for (const placement of placements) {
      const { unmount } = render(
        <Tooltip content="Tooltip content" placement={placement}>
          <button>Hover me</button>
        </Tooltip>
      );

      const button = screen.getByText('Hover me');
      fireEvent.mouseEnter(button);

      await waitFor(() => {
        const tooltip = screen.getByRole('tooltip');
        expect(tooltip).toHaveClass(`tooltip-${placement}`);
      }, { timeout: 500 });

      unmount();
    }
  });

  it('should clear timeout on unmount', async () => {
    const { unmount } = render(
      <Tooltip content="Tooltip content">
        <button>Hover me</button>
      </Tooltip>
    );

    const button = screen.getByText('Hover me');
    fireEvent.mouseEnter(button);

    // Unmount before delay completes
    unmount();

    // Wait for delay time to pass
    await new Promise(resolve => setTimeout(resolve, 350));

    // Should not throw error
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
  });

  it('should reset timeout on rapid hover', async () => {
    render(
      <Tooltip content="Tooltip content">
        <button>Hover me</button>
      </Tooltip>
    );

    const button = screen.getByText('Hover me');

    // First hover
    fireEvent.mouseEnter(button);

    // Wait a bit
    await new Promise(resolve => setTimeout(resolve, 100));

    // Mouse leave and enter again
    fireEvent.mouseLeave(button);
    fireEvent.mouseEnter(button);

    // Wait a bit more
    await new Promise(resolve => setTimeout(resolve, 100));

    // Should not show yet (timer reset)
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();

    // Complete the full delay
    await waitFor(() => {
      expect(screen.queryByRole('tooltip')).toBeInTheDocument();
    }, { timeout: 500 });
  });
});
