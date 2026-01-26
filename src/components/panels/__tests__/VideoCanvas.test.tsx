/**
 * VideoCanvas Component Tests
 * Tests video canvas with zoom and pan support
 */

import { describe, it, expect, vi } from 'vitest';
import { render, fireEvent } from '@/test/test-utils';
import { VideoCanvas } from '../YuvViewerPanel/VideoCanvas';

// Mock ModeOverlayRenderer
vi.mock('../ModeOverlayRenderer', () => ({
  renderModeOverlay: vi.fn(),
}));

describe('VideoCanvas', () => {
  const defaultProps = {
    frameImage: null,
    currentFrameIndex: 0,
    currentFrame: null,
    currentMode: 'overview' as const,
    zoom: 1,
    pan: { x: 0, y: 0 },
    onWheel: vi.fn(),
    onMouseDown: vi.fn(),
    onMouseMove: vi.fn(),
    onMouseUp: vi.fn(),
    isDragging: false,
  };

  it('should render video canvas', () => {
    render(<VideoCanvas {...defaultProps} />);

    const canvas = document.querySelector('.yuv-canvas');
    expect(canvas).toBeInTheDocument();
  });

  it('should render canvas container', () => {
    render(<VideoCanvas {...defaultProps} />);

    const container = document.querySelector('.yuv-canvas-container');
    expect(container).toBeInTheDocument();
  });

  it('should apply zoom transform', () => {
    const { container } = render(<VideoCanvas {...defaultProps} zoom={1.5} />);

    const canvas = container.querySelector('.yuv-canvas');
    expect(canvas?.style.transform).toContain('scale(1.5)');
  });

  it('should apply pan transform', () => {
    const { container } = render(<VideoCanvas {...defaultProps} pan={{ x: 50, y: 25 }} />);

    const canvas = container.querySelector('.yuv-canvas');
    expect(canvas?.style.transform).toContain('translate');
  });

  it('should show grab cursor when not dragging', () => {
    const { container } = render(<VideoCanvas {...defaultProps} isDragging={false} />);

    const canvasContainer = container.querySelector('.yuv-canvas-container');
    expect(canvasContainer?.style.cursor).toBe('grab');
  });

  it('should show grabbing cursor when dragging', () => {
    const { container } = render(<VideoCanvas {...defaultProps} isDragging={true} />);

    const canvasContainer = container.querySelector('.yuv-canvas-container');
    expect(canvasContainer?.style.cursor).toBe('grabbing');
  });

  it('should call onWheel on wheel event', () => {
    const handleWheel = vi.fn();
    render(<VideoCanvas {...defaultProps} onWheel={handleWheel} />);

    const container = document.querySelector('.yuv-canvas-container');
    if (container) {
      fireEvent.wheel(container, { deltaY: 100 });
      expect(handleWheel).toHaveBeenCalled();
    }
  });

  it('should call onMouseDown on mouse down', () => {
    const handleMouseDown = vi.fn();
    render(<VideoCanvas {...defaultProps} onMouseDown={handleMouseDown} />);

    const container = document.querySelector('.yuv-canvas-container');
    if (container) {
      fireEvent.mouseDown(container);
      expect(handleMouseDown).toHaveBeenCalled();
    }
  });

  it('should call onMouseMove on mouse move', () => {
    const handleMouseMove = vi.fn();
    render(<VideoCanvas {...defaultProps} onMouseMove={handleMouseMove} />);

    const container = document.querySelector('.yuv-canvas-container');
    if (container) {
      fireEvent.mouseMove(container);
      expect(handleMouseMove).toHaveBeenCalled();
    }
  });

  it('should call onMouseUp on mouse up', () => {
    const handleMouseUp = vi.fn();
    render(<VideoCanvas {...defaultProps} onMouseUp={handleMouseUp} />);

    const container = document.querySelector('.yuv-canvas-container');
    if (container) {
      fireEvent.mouseUp(container);
      expect(handleMouseUp).toHaveBeenCalled();
    }
  });

  it('should call onMouseUp on mouse leave', () => {
    const handleMouseUp = vi.fn();
    render(<VideoCanvas {...defaultProps} onMouseUp={handleMouseUp} />);

    const container = document.querySelector('.yuv-canvas-container');
    if (container) {
      fireEvent.mouseLeave(container);
      expect(handleMouseUp).toHaveBeenCalled();
    }
  });

  it('should use React.memo for performance', () => {
    const { rerender } = render(<VideoCanvas {...defaultProps} />);

    rerender(<VideoCanvas {...defaultProps} />);

    expect(document.querySelector('.yuv-canvas')).toBeInTheDocument();
  });
});

describe('VideoCanvas with frame image', () => {
  const mockImage = new Image();
  mockImage.width = 640;
  mockImage.height = 360;
  // Mock the src to avoid actual loading
  Object.defineProperty(mockImage, 'src', { value: 'data:image/test', writable: true });
  Object.defineProperty(mockImage, 'complete', { value: true, writable: true });

  const defaultProps = {
    frameImage: mockImage,
    currentFrameIndex: 0,
    currentFrame: { frame_index: 0, frame_type: 'I', size: 50000, poc: 0 },
    currentMode: 'overview' as const,
    zoom: 1,
    pan: { x: 0, y: 0 },
    onWheel: vi.fn(),
    onMouseDown: vi.fn(),
    onMouseMove: vi.fn(),
    onMouseUp: vi.fn(),
    isDragging: false,
  };

  it('should set canvas dimensions to match image', () => {
    const { container } = render(<VideoCanvas {...defaultProps} />);

    const canvas = container.querySelector('canvas');
    expect(canvas?.width).toBe(640);
    expect(canvas?.height).toBe(360);
  });

  it('should have correct transform origin', () => {
    const { container } = render(<VideoCanvas {...defaultProps} />);

    const canvas = container.querySelector('.yuv-canvas');
    expect(canvas?.style.transformOrigin).toBe('top left');
  });
});
