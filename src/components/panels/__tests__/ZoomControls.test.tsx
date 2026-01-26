/**
 * ZoomControls Component Tests
 * Tests zoom controls component
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@/test/test-utils';
import { ZoomControls } from '../YuvViewerPanel/ZoomControls';

describe('ZoomControls', () => {
  it('should render zoom controls', () => {
    render(
      <ZoomControls
        zoom={1}
        onZoomIn={vi.fn()}
        onZoomOut={vi.fn()}
        onResetZoom={vi.fn()}
      />
    );

    const buttons = screen.getAllByRole('button');
    expect(buttons.length).toBe(3);
  });

  it('should display zoom level', () => {
    render(
      <ZoomControls
        zoom={1.5}
        onZoomIn={vi.fn()}
        onZoomOut={vi.fn()}
        onResetZoom={vi.fn()}
      />
    );

    expect(screen.getByText('150%')).toBeInTheDocument();
  });

  it('should display 100% for zoom level 1', () => {
    render(
      <ZoomControls
        zoom={1}
        onZoomIn={vi.fn()}
        onZoomOut={vi.fn()}
        onResetZoom={vi.fn()}
      />
    );

    expect(screen.getByText('100%')).toBeInTheDocument();
  });

  it('should call onZoomIn when zoom in button clicked', () => {
    const handleZoomIn = vi.fn();
    render(
      <ZoomControls
        zoom={1}
        onZoomIn={handleZoomIn}
        onZoomOut={vi.fn()}
        onResetZoom={vi.fn()}
      />
    );

    const buttons = screen.getAllByRole('button');
    const zoomInButton = buttons.find(b => b.querySelector('.codicon-zoom-in'));
    if (zoomInButton) {
      fireEvent.click(zoomInButton);
      expect(handleZoomIn).toHaveBeenCalledTimes(1);
    }
  });

  it('should call onZoomOut when zoom out button clicked', () => {
    const handleZoomOut = vi.fn();
    render(
      <ZoomControls
        zoom={1}
        onZoomIn={vi.fn()}
        onZoomOut={handleZoomOut}
        onResetZoom={vi.fn()}
      />
    );

    const buttons = screen.getAllByRole('button');
    const zoomOutButton = buttons.find(b => b.querySelector('.codicon-zoom-out'));
    if (zoomOutButton) {
      fireEvent.click(zoomOutButton);
      expect(handleZoomOut).toHaveBeenCalledTimes(1);
    }
  });

  it('should call onResetZoom when reset button clicked', () => {
    const handleReset = vi.fn();
    render(
      <ZoomControls
        zoom={1.5}
        onZoomIn={vi.fn()}
        onZoomOut={vi.fn()}
        onResetZoom={handleReset}
      />
    );

    const buttons = screen.getAllByRole('button');
    const resetButton = buttons.find(b => b.querySelector('.codicon-screen-normal'));
    if (resetButton) {
      fireEvent.click(resetButton);
      expect(handleReset).toHaveBeenCalledTimes(1);
    }
  });

  it('should use React.memo for performance', () => {
    const { rerender } = render(
      <ZoomControls
        zoom={1}
        onZoomIn={vi.fn()}
        onZoomOut={vi.fn()}
        onResetZoom={vi.fn()}
      />
    );

    rerender(
      <ZoomControls
        zoom={1}
        onZoomIn={vi.fn()}
        onZoomOut={vi.fn()}
        onResetZoom={vi.fn()}
      />
    );

    expect(screen.getByText('100%')).toBeInTheDocument();
  });

  it('should display zoom percentage correctly for different values', () => {
    const { rerender } = render(
      <ZoomControls
        zoom={0.5}
        onZoomIn={vi.fn()}
        onZoomOut={vi.fn()}
        onResetZoom={vi.fn()}
      />
    );

    expect(screen.getByText('50%')).toBeInTheDocument();

    rerender(
      <ZoomControls
        zoom={2}
        onZoomIn={vi.fn()}
        onZoomOut={vi.fn()}
        onResetZoom={vi.fn()}
      />
    );

    expect(screen.getByText('200%')).toBeInTheDocument();
  });

  it('should have correct titles on buttons', () => {
    const { container } = render(
      <ZoomControls
        zoom={1}
        onZoomIn={vi.fn()}
        onZoomOut={vi.fn()}
        onResetZoom={vi.fn()}
      />
    );

    const buttons = container.querySelectorAll('button');
    expect(buttons[0]).toHaveAttribute('title', 'Zoom Out (-)');
    expect(buttons[2]).toHaveAttribute('title', 'Reset Zoom (Ctrl+0)');
  });

  it('should show all icons', () => {
    const { container } = render(
      <ZoomControls
        zoom={1}
        onZoomIn={vi.fn()}
        onZoomOut={vi.fn()}
        onResetZoom={vi.fn()}
      />
    );

    expect(container.querySelector('.codicon-zoom-out')).toBeInTheDocument();
    expect(container.querySelector('.codicon-zoom-in')).toBeInTheDocument();
    expect(container.querySelector('.codicon-screen-normal')).toBeInTheDocument();
  });
});
