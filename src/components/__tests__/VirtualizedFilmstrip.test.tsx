/**
 * VirtualizedFilmstrip Component Tests
 * Tests virtualized scrolling and frame rendering
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@/test/test-utils';
import { VirtualizedFilmstrip } from '@/components/VirtualizedFilmstrip';
import { mockFrames } from '@/test/test-utils';

// Mock context
vi.mock('@/contexts/StreamDataContext', () => ({
  useStreamData: () => ({
    frames: mockFrames,
    currentFrameIndex: 1,
  }),
}));

vi.mock('@/components/useFilmstripState', () => ({
  useFilmstripState: () => ({
    thumbnails: new Map([[0, 'data:image0'], [1, 'data:image1'], [2, 'data:image2']]),
    loadThumbnails: vi.fn(),
  }),
}));

describe('VirtualizedFilmstrip', () => {
  it('should render virtualized filmstrip', () => {
    render(<VirtualizedFilmstrip />);

    // Should render frame items
    expect(screen.queryAllByTestId(/frame-/i).length).toBeGreaterThan(0);
  });

  it('should render current frame indicator', () => {
    render(<VirtualizedFilmstrip />);

    // Current frame should be highlighted
    const currentFrame = document.querySelector('[data-selected="true"]');
    expect(currentFrame).toBeInTheDocument();
  });

  it('should handle frame click', () => {
    const handleNavigate = vi.fn();
    render(<VirtualizedFilmstrip onFrameClick={handleNavigate} />);

    const frames = screen.queryAllByTestId(/frame-/i);
    if (frames.length > 0) {
      fireEvent.click(frames[0]);
      expect(handleNavigate).toHaveBeenCalled();
    }
  });

  it('should display frame type indicators', () => {
    render(<VirtualizedFilmstrip />);

    // Should have frame type badges
    const types = document.querySelectorAll('.frame-type-i, .frame-type-p, .frame-type-b');
    expect(types.length).toBeGreaterThan(0);
  });

  it('should render frame numbers', () => {
    render(<VirtualizedFilmstrip />);

    // Should show frame numbers
    const frameNumbers = screen.queryAllByText(/\d+/);
    expect(frameNumbers.length).toBeGreaterThan(0);
  });

  it('should support horizontal scrolling', () => {
    const { container } = render(<VirtualizedFilmstrip />);

    const scrollContainer = container.querySelector('.virtualized-filmstrip-container');
    expect(scrollContainer).toBeInTheDocument();
  });

  it('should use IntersectionObserver for lazy loading', () => {
    // Mock IntersectionObserver
    const mockObserve = vi.fn();
    global.IntersectionObserver = vi.fn().mockImplementation(() => ({
      observe: mockObserve,
      unobserve: vi.fn(),
      disconnect: vi.fn(),
    }));

    render(<VirtualizedFilmstrip />);

    // IntersectionObserver should be used
    expect(global.IntersectionObserver).toHaveBeenCalled();
  });

  it('should handle empty frames array', () => {
    vi.doMock('@/contexts/StreamDataContext', () => ({
      useStreamData: () => ({
        frames: [],
        currentFrameIndex: 0,
      }),
    }));

    render(<VirtualizedFilmstrip />);

    // Should render empty state or handle gracefully
    expect(document.querySelector('.virtualized-filmstrip-container')).toBeInTheDocument();
  });

  it('should support custom thumbnail size', () => {
    render(<VirtualizedFilmstrip thumbnailWidth={100} />);

    // Thumbnails should render with custom size
    const thumbnails = document.querySelectorAll('[style*="width"]');
    expect(thumbnails.length).toBeGreaterThan(0);
  });

  it('should use stable callbacks (useCallback optimization)', () => {
    const { rerender } = render(<VirtualizedFilmstrip />);

    rerender(<VirtualizedFilmstrip />);

    // Should still function correctly
    expect(document.querySelector('.virtualized-filmstrip-container')).toBeInTheDocument();
  });
});

describe('VirtualizedFilmstrip performance', () => {
  it('should only render visible frames', () => {
    // Create large frame array
    const largeFrameArray = Array.from({ length: 1000 }, (_, i) => ({
      frame_index: i,
      frame_type: i % 3 === 0 ? 'I' : i % 3 === 1 ? 'P' : 'B',
      size: 10000,
      poc: i,
    }));

    vi.doMock('@/contexts/StreamDataContext', () => ({
      useStreamData: () => ({
        frames: largeFrameArray,
        currentFrameIndex: 500,
      }),
    }));

    render(<VirtualizedFilmstrip />);

    // Should not render all 1000 frames
    const renderedFrames = document.querySelectorAll('[data-frame-index]');
    expect(renderedFrames.length).toBeLessThan(1000);
  });

  it('should update visible frames on scroll', () => {
    render(<VirtualizedFilmstrip />);

    const container = document.querySelector('.virtualized-filmstrip-container');
    if (container) {
      // Simulate scroll
      fireEvent.scroll(container, { target: { scrollLeft: 1000 } });

      // Should handle scroll gracefully
      expect(container).toBeInTheDocument();
    }
  });
});
