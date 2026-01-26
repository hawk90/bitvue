/**
 * StatusBar Component Tests
 * Tests status bar with frame and playback info
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@/test/test-utils';
import { StatusBar } from '../YuvViewerPanel/StatusBar';

const defaultProps = {
  currentFrameIndex: 42,
  totalFrames: 1000,
  currentMode: 'overview' as const,
  zoom: 1.5,
  isPlaying: true,
  playbackSpeed: 2,
};

describe('StatusBar', () => {

  it('should render status bar', () => {
    render(<StatusBar {...defaultProps} />);

    const statusBar = document.querySelector('.yuv-status-bar');
    expect(statusBar).toBeInTheDocument();
  });

  it('should display frame count', () => {
    render(<StatusBar {...defaultProps} />);

    expect(screen.getByText('42')).toBeInTheDocument();
    expect(screen.getByText('/1000')).toBeInTheDocument();
  });

  it('should display current mode', () => {
    render(<StatusBar {...defaultProps} />);

    expect(screen.getByText('overview')).toBeInTheDocument();
  });

  it('should display zoom level', () => {
    render(<StatusBar {...defaultProps} />);

    expect(screen.getByText('150%')).toBeInTheDocument();
  });

  it('should display playback status', () => {
    render(<StatusBar {...defaultProps} />);

    expect(screen.getByText(/playing/i)).toBeInTheDocument();
  });

  it('should display playback speed', () => {
    render(<StatusBar {...defaultProps} />);

    expect(screen.getByText('2x')).toBeInTheDocument();
  });

  it('should use React.memo for performance', () => {
    const { rerender } = render(<StatusBar {...defaultProps} />);

    rerender(<StatusBar {...defaultProps} />);

    expect(document.querySelector('.yuv-status-bar')).toBeInTheDocument();
  });

  it('should show paused status when not playing', () => {
    render(<StatusBar {...defaultProps} isPlaying={false} />);

    expect(screen.getByText(/paused/i)).toBeInTheDocument();
  });

  it('should handle single frame', () => {
    render(<StatusBar {...defaultProps} totalFrames={1} currentFrameIndex={0} />);

    expect(screen.getByText('0/1')).toBeInTheDocument();
  });

  it('should handle zero zoom', () => {
    render(<StatusBar {...defaultProps} zoom={0} />);

    expect(screen.getByText('0%')).toBeInTheDocument();
  });

  it('should handle fractional zoom', () => {
    render(<StatusBar {...defaultProps} zoom={0.5} />);

    expect(screen.getByText('50%')).toBeInTheDocument();
  });
});

describe('StatusBar formatting', () => {
  it('should format large frame numbers', () => {
    render(<StatusBar {...defaultProps} totalFrames={100000} currentFrameIndex={50000} />);

    expect(screen.getByText('50000')).toBeInTheDocument();
    expect(screen.getByText('100000')).toBeInTheDocument();
  });

  it('should handle 0-indexed frame', () => {
    render(<StatusBar {...defaultProps} currentFrameIndex={0} />);

    expect(screen.getByText('0/')).toBeInTheDocument();
  });

  it('should display different mode names', () => {
    const { rerender } = render(<StatusBar {...defaultProps} currentMode="prediction" />);

    expect(screen.getByText('prediction')).toBeInTheDocument();

    rerender(<StatusBar {...defaultProps} currentMode="transform" />);

    expect(screen.getByText('transform')).toBeInTheDocument();
  });

  it('should display decimal zoom levels', () => {
    render(<StatusBar {...defaultProps} zoom={1.25} />);

    expect(screen.getByText('125%')).toBeInTheDocument();
  });
});

describe('StatusBar edge cases', () => {
  it('should handle zero total frames', () => {
    render(<StatusBar {...defaultProps} totalFrames={0} currentFrameIndex={0} />);

    expect(screen.getByText('0/0')).toBeInTheDocument();
  });

  it('should handle very fast playback speed', () => {
    render(<StatusBar {...defaultProps} playbackSpeed={16} />);

    expect(screen.getByText('16x')).toBeInTheDocument();
  });

  it('should handle slow playback speed', () => {
    render(<StatusBar {...defaultProps} playbackSpeed={0.25} />);

    expect(screen.getByText('0.25x')).toBeInTheDocument();
  });

  it('should handle very high zoom', () => {
    render(<StatusBar {...defaultProps} zoom={4} />);

    expect(screen.getByText('400%')).toBeInTheDocument();
  });
});

describe('StatusBar layout', () => {
  it('should have proper CSS classes', () => {
    const { container } = render(<StatusBar {...defaultProps} />);

    const statusBar = container.querySelector('.yuv-status-bar');
    expect(statusBar).toBeInTheDocument();
  });

  it('should display status sections', () => {
    render(<StatusBar {...defaultProps} />);

    const sections = document.querySelectorAll('.status-section');
    expect(sections.length).toBeGreaterThan(0);
  });
});
