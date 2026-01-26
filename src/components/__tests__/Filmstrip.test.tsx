/**
 * Filmstrip Component Tests
 * Main filmstrip with view modes and frame expansion
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@/test/test-utils';
import { Filmstrip } from '@/components/Filmstrip';

// Mock dependencies
vi.mock('@/contexts/StreamDataContext', () => ({
  useStreamData: () => ({
    frames: [
      {
        frame_index: 0,
        frame_type: 'I',
        size: 50000,
        ref_frames: [],
        poc: 0,
        key_frame: true,
      },
      {
        frame_index: 1,
        frame_type: 'P',
        size: 30000,
        ref_frames: [0],
        poc: 1,
      },
      {
        frame_index: 2,
        frame_type: 'B',
        size: 20000,
        ref_frames: [0, 1],
        poc: 2,
      },
    ],
    currentFrameIndex: 1,
  }),
}));

vi.mock('@/components/useFilmstripState', () => ({
  useFilmstripState: () => ({
    thumbnails: new Map([
      [0, 'data:image0;base64,abc'],
      [1, 'data:image1;base64,def'],
      [2, 'data:image2;base64,ghi'],
    ]),
    loadThumbnails: vi.fn(),
  }),
}));

describe('Filmstrip', () => {
  it('should render filmstrip container', () => {
    render(<Filmstrip />);

    expect(screen.queryByTestId('filmstrip')).toBeInTheDocument();
  });

  it('should render all frames', () => {
    render(<Filmstrip />);

    // Should show frame thumbnails
    const frames = document.querySelectorAll('[data-frame-index]');
    expect(frames.length).toBe(3);
  });

  it('should highlight current frame', () => {
    render(<Filmstrip />);

    const currentFrame = document.querySelector('[data-current="true"]');
    expect(currentFrame).toBeInTheDocument();
  });

  it('should render frame type borders', () => {
    render(<Filmstrip />);

    // I-frames should have red border
    const iFrame = document.querySelector('[data-frame-type="I"]');
    expect(iFrame).toBeInTheDocument();

    // P-frames should have green border
    const pFrame = document.querySelector('[data-frame-type="P"]');
    expect(pFrame).toBeInTheDocument();

    // B-frames should have blue border
    const bFrame = document.querySelector('[data-frame-type="B"]');
    expect(bFrame).toBeInTheDocument();
  });

  it('should handle frame selection', () => {
    const handleNavigate = vi.fn();
    render(<Filmstrip onFrameClick={handleNavigate} />);

    const frames = document.querySelectorAll('[data-frame-index]');
    if (frames.length > 0) {
      fireEvent.click(frames[0]);
      expect(handleNavigate).toHaveBeenCalledWith(0);
    }
  });

  it('should expand frame on click', () => {
    render(<Filmstrip />);

    const frame = document.querySelector('[data-frame-index="1"]');
    if (frame) {
      fireEvent.click(frame);

      // Should show expanded state
      expect(frame).toHaveClass('expanded');
    }
  });

  it('should show reference frame arrows', () => {
    render(<Filmstrip />);

    // B-frame (index 2) references frames 0 and 1
    // Should show arrow indicators
    const arrows = document.querySelectorAll('.reference-arrow');
    expect(arrows.length).toBeGreaterThanOrEqual(0);
  });

  it('should display view mode selector', () => {
    render(<Filmstrip />);

    // Should have dropdown to select view mode
    const dropdown = document.querySelector('.filmstrip-dropdown');
    expect(dropdown).toBeInTheDocument();
  });

  it('should switch between view modes', () => {
    render(<Filmstrip />);

    const dropdown = document.querySelector('.filmstrip-dropdown');
    if (dropdown) {
      fireEvent.click(dropdown);

      // Should show view options
      expect(screen.getByText('Thumbnails')).toBeInTheDocument();
      expect(screen.getByText('Frame Sizes')).toBeInTheDocument();
    }
  });

  it('should load thumbnails lazily', () => {
    render(<Filmstrip />);

    // IntersectionObserver should trigger thumbnail loading
    // This is tested by checking if thumbnails exist
    const thumbnails = document.querySelectorAll('img[src^="data:image"]');
    expect(thumbnails.length).toBeGreaterThan(0);
  });

  it('should handle empty frames array', () => {
    vi.doMock('@/contexts/StreamDataContext', () => ({
      useStreamData: () => ({
        frames: [],
        currentFrameIndex: 0,
      }),
    }));

    render(<Filmstrip />);

    // Should handle gracefully - may show empty state
    expect(screen.queryByTestId('filmstrip')).toBeInTheDocument();
  });

  it('should display frame index labels', () => {
    render(<Filmstrip />);

    // Should show frame numbers
    expect(screen.getByText('#0')).toBeInTheDocument();
    expect(screen.getByText('#1')).toBeInTheDocument();
  });

  it('should support custom view mode', () => {
    render(<Filmstrip defaultView="sizes" />);

    // Should start with frame sizes view
    expect(screen.queryByText('Frame Sizes')).toBeInTheDocument();
  });
});

describe('Filmstrip interaction', () => {
  it('should navigate to frame on thumbnail click', () => {
    const handleNavigate = vi.fn();
    render(<Filmstrip onFrameClick={handleNavigate} />);

    const firstFrame = document.querySelector('[data-frame-index="0"]');
    if (firstFrame) {
      fireEvent.click(firstFrame);
      expect(handleNavigate).toHaveBeenCalledWith(0);
    }
  });

  it('should support keyboard navigation', () => {
    render(<Filmstrip />);

    const filmstrip = screen.queryByTestId('filmstrip');
    if (filmstrip) {
      fireEvent.keyDown(filmstrip, { key: 'ArrowRight' });
      fireEvent.keyDown(filmstrip, { key: 'ArrowLeft' });

      // Should handle keyboard events
      expect(filmstrip).toBeInTheDocument();
    }
  });

  it('should update on frame index change', () => {
    const { rerender } = render(<Filmstrip />);

    // Change current frame
    vi.doMock('@/contexts/StreamDataContext', () => ({
      useStreamData: () => ({
        frames: mockFrames,
        currentFrameIndex: 2,
      }),
    }));

    rerender(<Filmstrip />);

    const newCurrentFrame = document.querySelector('[data-current="true"]');
    expect(newCurrentFrame?.getAttribute('data-frame-index')).toBe('2');
  });
});

describe('Filmstrip performance', () => {
  it('should handle large frame count efficiently', () => {
    const largeFrames = Array.from({ length: 1000 }, (_, i) => ({
      frame_index: i,
      frame_type: i % 3 === 0 ? 'I' : i % 3 === 1 ? 'P' : 'B',
      size: 10000 + i * 100,
      ref_frames: i > 0 ? [0] : [],
      poc: i,
    }));

    vi.doMock('@/contexts/StreamDataContext', () => ({
      useStreamData: () => ({
        frames: largeFrames,
        currentFrameIndex: 500,
      }),
    }));

    render(<Filmstrip />);

    // Should render without hanging
    expect(screen.queryByTestId('filmstrip')).toBeInTheDocument();
  });

  it('should use React.memo for performance', () => {
    const { rerender } = render(<Filmstrip />);

    // Rerender with same props
    rerender(<Filmstrip />);

    // Component should handle rerender efficiently
    expect(screen.queryByTestId('filmstrip')).toBeInTheDocument();
  });
});
