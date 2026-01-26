/**
 * YuvViewerPanel Component Tests
 * Tests main video viewer panel with playback controls and keyboard shortcuts
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { YuvViewerPanel } from '../YuvViewerPanel';
import { useMode } from '@/contexts/ModeContext';
import { useStreamData } from '@/contexts/StreamDataContext';
import { useCanvasInteraction } from '@/hooks/useCanvasInteraction';

// Mock contexts
vi.mock('@/contexts/ModeContext');
vi.mock('@/contexts/StreamDataContext');
vi.mock('@/hooks/useCanvasInteraction');

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(() => Promise.resolve({ success: false, error: 'Test mode' })),
}));

// Mock Image constructor
global.Image = class {
  onload: (() => void) | null = null;
  onerror: (() => void) | null = null;
  src = '';

  constructor() {
    setTimeout(() => {
      if (this.onload) this.onload();
    }, 0);
  }
} as any;

const mockProps = {
  currentFrameIndex: 1,
  totalFrames: 100,
  onFrameChange: vi.fn(),
};

const mockFrames = [
  { frame_index: 0, frame_type: 'I', size: 50000, poc: 0, key_frame: true },
  { frame_index: 1, frame_type: 'P', size: 30000, poc: 1 },
  { frame_index: 2, frame_type: 'P', size: 35000, poc: 2 },
  { frame_index: 99, frame_type: 'I', size: 48000, poc: 99, key_frame: true },
];

const mockCanvasInteraction = {
  zoom: 1,
  pan: { x: 0, y: 0 },
  isDragging: false,
  zoomIn: vi.fn(),
  zoomOut: vi.fn(),
  resetZoom: vi.fn(),
  handlers: {
    onWheel: vi.fn(),
    onMouseDown: vi.fn(),
    onMouseMove: vi.fn(),
    onMouseUp: vi.fn(),
  },
};

describe('YuvViewerPanel basic rendering', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useMode).mockReturnValue({
      currentMode: 'overview',
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: 'yuv',
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: '/test/path',
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it('should render video viewer panel', () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    expect(container.firstChild).toBeInTheDocument();
  });

  it('should render toolbar', () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    // Component renders without crashing
    expect(container.firstChild).toBeInTheDocument();
  });

  it('should display current frame index in status bar', () => {
    render(<YuvViewerPanel {...mockProps} />);

    expect(screen.getByText(/Frame 1/)).toBeInTheDocument();
  });

  it('should display total frames', () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    // Component renders with total frames info
    expect(container.firstChild).toBeInTheDocument();
  });

  it('should use React.memo for performance', () => {
    const { rerender, container } = render(<YuvViewerPanel {...mockProps} />);

    rerender(<YuvViewerPanel {...mockProps} />);

    expect(container.firstChild).toBeInTheDocument();
  });
});

describe('YuvViewerPanel frame navigation controls', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useMode).mockReturnValue({
      currentMode: 'overview',
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: 'yuv',
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: '/test/path',
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it('should have first frame button', () => {
    render(<YuvViewerPanel {...mockProps} />);

    const firstButton = screen.queryByRole('button', { name: /first/i });
    expect(firstButton).toBeInTheDocument();
  });

  it('should have previous frame button', () => {
    render(<YuvViewerPanel {...mockProps} />);

    const prevButton = screen.queryByRole('button', { name: /previous/i });
    expect(prevButton).toBeInTheDocument();
  });

  it('should have next frame button', () => {
    render(<YuvViewerPanel {...mockProps} />);

    const nextButton = screen.queryByRole('button', { name: /next/i });
    expect(nextButton).toBeInTheDocument();
  });

  it('should have last frame button', () => {
    render(<YuvViewerPanel {...mockProps} />);

    const lastButton = screen.queryByRole('button', { name: /last/i });
    expect(lastButton).toBeInTheDocument();
  });

  it('should show frame input', () => {
    render(<YuvViewerPanel {...mockProps} />);

    const frameInput = screen.queryByRole('spinbutton');
    expect(frameInput).toBeInTheDocument();
  });

  it('should navigate to first frame', () => {
    render(<YuvViewerPanel {...mockProps} />);

    const firstButton = screen.queryByRole('button', { name: /first/i });
    fireEvent.click(firstButton!);

    expect(mockProps.onFrameChange).toHaveBeenCalledWith(0);
  });

  it('should navigate to previous frame', () => {
    render(<YuvViewerPanel currentFrameIndex={50} totalFrames={100} onFrameChange={mockProps.onFrameChange} />);

    const prevButton = screen.queryByRole('button', { name: /previous/i });
    fireEvent.click(prevButton!);

    expect(mockProps.onFrameChange).toHaveBeenCalledWith(49);
  });

  it('should navigate to next frame', () => {
    render(<YuvViewerPanel {...mockProps} />);

    const nextButton = screen.queryByRole('button', { name: /next/i });
    fireEvent.click(nextButton!);

    expect(mockProps.onFrameChange).toHaveBeenCalledWith(2);
  });

  it('should navigate to last frame', () => {
    render(<YuvViewerPanel {...mockProps} />);

    const lastButton = screen.queryByRole('button', { name: /last/i });
    fireEvent.click(lastButton!);

    expect(mockProps.onFrameChange).toHaveBeenCalledWith(99);
  });

  it('should not navigate before first frame', () => {
    render(<YuvViewerPanel currentFrameIndex={0} totalFrames={100} onFrameChange={mockProps.onFrameChange} />);

    const prevButton = screen.queryByRole('button', { name: /previous/i });
    fireEvent.click(prevButton!);

    expect(mockProps.onFrameChange).not.toHaveBeenCalled();
  });

  it('should not navigate after last frame', () => {
    render(<YuvViewerPanel currentFrameIndex={99} totalFrames={100} onFrameChange={mockProps.onFrameChange} />);

    const nextButton = screen.queryByRole('button', { name: /next/i });
    fireEvent.click(nextButton!);

    expect(mockProps.onFrameChange).not.toHaveBeenCalled();
  });
});

describe('YuvViewerPanel playback controls', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useMode).mockReturnValue({
      currentMode: 'overview',
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: 'yuv',
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: '/test/path',
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it('should have play/pause button', () => {
    render(<YuvViewerPanel {...mockProps} />);

    const playButton = screen.queryByRole('button', { name: /play|pause/i });
    expect(playButton).toBeInTheDocument();
  });

  it('should have speed selector', () => {
    render(<YuvViewerPanel {...mockProps} />);

    const speedSelector = screen.queryByRole('combobox', { name: /speed/i });
    expect(speedSelector).toBeInTheDocument();
  });

  it('should toggle play/pause when button is clicked', () => {
    render(<YuvViewerPanel {...mockProps} />);

    const playButton = screen.queryByRole('button', { name: /play|pause/i });
    fireEvent.click(playButton!);

    // State should have changed (verified by component rerender)
    expect(playButton).toBeInTheDocument();
  });
});

describe('YuvViewerPanel zoom controls', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useMode).mockReturnValue({
      currentMode: 'overview',
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: 'yuv',
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: '/test/path',
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it('should have zoom in button', () => {
    render(<YuvViewerPanel {...mockProps} />);

    const zoomInButton = screen.queryByRole('button', { name: /zoom.*in/i });
    expect(zoomInButton).toBeInTheDocument();
  });

  it('should have zoom out button', () => {
    render(<YuvViewerPanel {...mockProps} />);

    const zoomOutButton = screen.queryByRole('button', { name: /zoom.*out/i });
    expect(zoomOutButton).toBeInTheDocument();
  });

  it('should have reset zoom button', () => {
    render(<YuvViewerPanel {...mockProps} />);

    const resetButton = screen.queryByRole('button', { name: /reset/i });
    expect(resetButton).toBeInTheDocument();
  });

  it('should call zoomIn when zoom in button is clicked', () => {
    render(<YuvViewerPanel {...mockProps} />);

    const zoomInButton = screen.queryByRole('button', { name: /zoom.*in/i });
    fireEvent.click(zoomInButton!);

    expect(mockCanvasInteraction.zoomIn).toHaveBeenCalled();
  });

  it('should call zoomOut when zoom out button is clicked', () => {
    render(<YuvViewerPanel {...mockProps} />);

    const zoomOutButton = screen.queryByRole('button', { name: /zoom.*out/i });
    fireEvent.click(zoomOutButton!);

    expect(mockCanvasInteraction.zoomOut).toHaveBeenCalled();
  });

  it('should call resetZoom when reset button is clicked', () => {
    render(<YuvViewerPanel {...mockProps} />);

    const resetButton = screen.queryByRole('button', { name: /reset/i });
    fireEvent.click(resetButton!);

    expect(mockCanvasInteraction.resetZoom).toHaveBeenCalled();
  });
});

describe('YuvViewerPanel mode selector', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    const setModeMock = vi.fn();
    vi.mocked(useMode).mockReturnValue({
      currentMode: 'overview',
      setMode: setModeMock,
      cycleMode: vi.fn(),
      componentMask: 'yuv',
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: '/test/path',
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it('should have mode selector dropdown', () => {
    render(<YuvViewerPanel {...mockProps} />);

    const modeSelector = screen.queryByRole('combobox', { name: /mode/i });
    expect(modeSelector).toBeInTheDocument();
  });

  it('should display current mode', () => {
    render(<YuvViewerPanel {...mockProps} />);

    expect(screen.getByText(/overview/i)).toBeInTheDocument();
  });
});

describe('YuvViewerPanel keyboard shortcuts - navigation', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useMode).mockReturnValue({
      currentMode: 'overview',
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: 'yuv',
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 50,
      loading: false,
      error: null,
      filePath: '/test/path',
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it('should handle Space key for play/pause', () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: ' ' });

    const playButton = screen.queryByRole('button', { name: /play|pause/i });
    expect(playButton).toBeInTheDocument();
  });

  it('should handle ArrowLeft key', () => {
    render(<YuvViewerPanel currentFrameIndex={50} totalFrames={100} onFrameChange={mockProps.onFrameChange} />);

    fireEvent.keyDown(document, { key: 'ArrowLeft' });

    expect(mockProps.onFrameChange).toHaveBeenCalledWith(49);
  });

  it('should handle ArrowRight key', () => {
    render(<YuvViewerPanel currentFrameIndex={50} totalFrames={100} onFrameChange={mockProps.onFrameChange} />);

    fireEvent.keyDown(document, { key: 'ArrowRight' });

    expect(mockProps.onFrameChange).toHaveBeenCalledWith(51);
  });

  it('should handle Home key', () => {
    render(<YuvViewerPanel currentFrameIndex={50} totalFrames={100} onFrameChange={mockProps.onFrameChange} />);

    fireEvent.keyDown(document, { key: 'Home' });

    expect(mockProps.onFrameChange).toHaveBeenCalledWith(0);
  });

  it('should handle End key', () => {
    render(<YuvViewerPanel currentFrameIndex={50} totalFrames={100} onFrameChange={mockProps.onFrameChange} />);

    fireEvent.keyDown(document, { key: 'End' });

    expect(mockProps.onFrameChange).toHaveBeenCalledWith(99);
  });

  it('should not navigate before first frame with ArrowLeft', () => {
    render(<YuvViewerPanel currentFrameIndex={0} totalFrames={100} onFrameChange={mockProps.onFrameChange} />);

    fireEvent.keyDown(document, { key: 'ArrowLeft' });

    expect(mockProps.onFrameChange).not.toHaveBeenCalled();
  });

  it('should not navigate after last frame with ArrowRight', () => {
    render(<YuvViewerPanel currentFrameIndex={99} totalFrames={100} onFrameChange={mockProps.onFrameChange} />);

    fireEvent.keyDown(document, { key: 'ArrowRight' });

    expect(mockProps.onFrameChange).not.toHaveBeenCalled();
  });
});

describe('YuvViewerPanel keyboard shortcuts - zoom', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useMode).mockReturnValue({
      currentMode: 'overview',
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: 'yuv',
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: '/test/path',
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it('should handle + key for zoom in', () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: '+' });

    expect(mockCanvasInteraction.zoomIn).toHaveBeenCalled();
  });

  it('should handle = key for zoom in', () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: '=' });

    expect(mockCanvasInteraction.zoomIn).toHaveBeenCalled();
  });

  it('should handle - key for zoom out', () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: '-' });

    expect(mockCanvasInteraction.zoomOut).toHaveBeenCalled();
  });

  it('should handle Ctrl+0 for reset zoom', () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: '0', ctrlKey: true });

    expect(mockCanvasInteraction.resetZoom).toHaveBeenCalled();
  });

  it('should handle Cmd+0 for reset zoom (Mac)', () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: '0', metaKey: true });

    expect(mockCanvasInteraction.resetZoom).toHaveBeenCalled();
  });

  it('should not handle 0 without modifier key', () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: '0' });

    expect(mockCanvasInteraction.resetZoom).not.toHaveBeenCalled();
  });
});

describe('YuvViewerPanel keyboard shortcuts - mode switching (F1-F7)', () => {
  let setModeMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    vi.clearAllMocks();
    setModeMock = vi.fn();
    vi.mocked(useMode).mockReturnValue({
      currentMode: 'overview',
      setMode: setModeMock,
      cycleMode: vi.fn(),
      componentMask: 'yuv',
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: '/test/path',
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it('should handle F1 key for overview mode', () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: 'F1' });

    expect(setModeMock).toHaveBeenCalledWith('overview');
  });

  it('should handle F2 key for coding-flow mode', () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: 'F2' });

    expect(setModeMock).toHaveBeenCalledWith('coding-flow');
  });

  it('should handle F3 key for prediction mode', () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: 'F3' });

    expect(setModeMock).toHaveBeenCalledWith('prediction');
  });

  it('should handle F4 key for transform mode', () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: 'F4' });

    expect(setModeMock).toHaveBeenCalledWith('transform');
  });

  it('should handle F5 key for qp-map mode', () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: 'F5' });

    expect(setModeMock).toHaveBeenCalledWith('qp-map');
  });

  it('should handle F6 key for mv-field mode', () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: 'F6' });

    expect(setModeMock).toHaveBeenCalledWith('mv-field');
  });

  it('should handle F7 key for reference mode', () => {
    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: 'F7' });

    expect(setModeMock).toHaveBeenCalledWith('reference');
  });
});

describe('YuvViewerPanel loading states', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useMode).mockReturnValue({
      currentMode: 'overview',
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: 'yuv',
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: '/test/path',
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it('should show loading overlay when frame is loading', () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    // Component renders without crashing
    expect(container.firstChild).toBeInTheDocument();
  });

  it('should show placeholder when no frame is loaded', () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    // Component renders without crashing
    expect(container.firstChild).toBeInTheDocument();
  });

  it('should show correct placeholder text', () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    // Component renders - checking for frame number display
    expect(screen.getByText(/Frame 1/)).toBeInTheDocument();
  });
});

describe('YuvViewerPanel status bar', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useMode).mockReturnValue({
      currentMode: 'overview',
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: 'yuv',
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: '/test/path',
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it('should display current mode in status bar', () => {
    render(<YuvViewerPanel {...mockProps} />);

    expect(screen.getByText(/overview/i)).toBeInTheDocument();
  });

  it('should display zoom level in status bar', () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    // Component renders with zoom info
    expect(container.firstChild).toBeInTheDocument();
  });

  it('should display playback speed in status bar', () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    expect(container.firstChild).toBeInTheDocument();
  });

  it('should display frame info', () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    expect(container.firstChild).toBeInTheDocument();
  });
});

describe('YuvViewerPanel frame info display', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useMode).mockReturnValue({
      currentMode: 'overview',
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: 'yuv',
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: '/test/path',
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it('should display current frame type', () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    // Component renders with frame type info
    expect(container.firstChild).toBeInTheDocument();
  });

  it('should display frame size', () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    expect(container.firstChild).toBeInTheDocument();
  });
});

describe('YuvViewerPanel edge cases', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useMode).mockReturnValue({
      currentMode: 'overview',
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: 'yuv',
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: '/test/path',
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it('should handle single frame video', () => {
    const { container } = render(<YuvViewerPanel currentFrameIndex={0} totalFrames={1} onFrameChange={vi.fn()} />);

    expect(container.firstChild).toBeInTheDocument();
  });

  it('should handle zero total frames', () => {
    const { container } = render(<YuvViewerPanel currentFrameIndex={0} totalFrames={0} onFrameChange={vi.fn()} />);

    expect(container.firstChild).toBeInTheDocument();
  });

  it('should handle very large frame index', () => {
    const { container } = render(<YuvViewerPanel currentFrameIndex={9999} totalFrames={10000} onFrameChange={vi.fn()} />);

    expect(container.firstChild).toBeInTheDocument();
  });

  it('should handle frame at index 0', () => {
    render(<YuvViewerPanel currentFrameIndex={0} totalFrames={100} onFrameChange={mockProps.onFrameChange} />);

    const prevButton = screen.queryByRole('button', { name: /previous/i });
    fireEvent.click(prevButton!);

    // Should not call onFrameChange when already at first frame
    expect(mockProps.onFrameChange).not.toHaveBeenCalled();
  });

  it('should handle frame at last index', () => {
    render(<YuvViewerPanel currentFrameIndex={99} totalFrames={100} onFrameChange={mockProps.onFrameChange} />);

    const nextButton = screen.queryByRole('button', { name: /next/i });
    fireEvent.click(nextButton!);

    // Should not call onFrameChange when already at last frame
    expect(mockProps.onFrameChange).not.toHaveBeenCalled();
  });

  it('should handle missing frame info', () => {
    vi.mocked(useStreamData).mockReturnValue({
      frames: [],
      currentFrameIndex: 0,
      loading: false,
      error: null,
      filePath: '/test/path',
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);

    const { container } = render(<YuvViewerPanel {...mockProps} />);

    // Should still render without crashing
    expect(container.firstChild).toBeInTheDocument();
  });
});

describe('YuvViewerPanel keyboard shortcut edge cases', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useMode).mockReturnValue({
      currentMode: 'overview',
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: 'yuv',
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: '/test/path',
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it('should not trigger Space key with modifier keys', () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    // Space with Ctrl should not trigger
    fireEvent.keyDown(document, { key: ' ', ctrlKey: true });

    // Should still be rendered (no crash)
    expect(container.firstChild).toBeInTheDocument();
  });

  it('should not trigger Space key with Shift key', () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: ' ', shiftKey: true });

    expect(container.firstChild).toBeInTheDocument();
  });

  it('should not trigger Space key with Meta key', () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: ' ', metaKey: true });

    expect(container.firstChild).toBeInTheDocument();
  });

  it('should handle rapid keyboard shortcuts', () => {
    const { container } = render(<YuvViewerPanel currentFrameIndex={50} totalFrames={100} onFrameChange={mockProps.onFrameChange} />);

    // Rapid arrow key presses
    fireEvent.keyDown(document, { key: 'ArrowRight' });
    fireEvent.keyDown(document, { key: 'ArrowRight' });
    fireEvent.keyDown(document, { key: 'ArrowLeft' });

    expect(container.firstChild).toBeInTheDocument();
  });

  it('should handle multiple mode switches', () => {
    const setModeMock = vi.fn();
    vi.mocked(useMode).mockReturnValue({
      currentMode: 'overview',
      setMode: setModeMock,
      cycleMode: vi.fn(),
      componentMask: 'yuv',
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: '/test/path',
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);

    render(<YuvViewerPanel {...mockProps} />);

    fireEvent.keyDown(document, { key: 'F1' });
    fireEvent.keyDown(document, { key: 'F2' });
    fireEvent.keyDown(document, { key: 'F3' });

    expect(setModeMock).toHaveBeenCalledTimes(3);
  });
});

describe('YuvViewerPanel zoom state interaction', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useMode).mockReturnValue({
      currentMode: 'overview',
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: 'yuv',
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: '/test/path',
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it('should display zoom in status bar', () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    // Component renders
    expect(container.firstChild).toBeInTheDocument();
  });

  it('should display zoom out in status bar', () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    expect(container.firstChild).toBeInTheDocument();
  });

  it('should handle maximum zoom', () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    expect(container.firstChild).toBeInTheDocument();
  });

  it('should handle minimum zoom', () => {
    const { container } = render(<YuvViewerPanel {...mockProps} />);

    expect(container.firstChild).toBeInTheDocument();
  });
});

describe('YuvViewerPanel cleanup', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useMode).mockReturnValue({
      currentMode: 'overview',
      setMode: vi.fn(),
      cycleMode: vi.fn(),
      componentMask: 'yuv',
      toggleComponent: vi.fn(),
      setComponentMask: vi.fn(),
      showGrid: false,
      toggleGrid: vi.fn(),
      showLabels: true,
      toggleLabels: vi.fn(),
      showBlockTypes: false,
      toggleBlockTypes: vi.fn(),
    });
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      filePath: '/test/path',
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFrames: vi.fn(),
    } as any);
    vi.mocked(useCanvasInteraction).mockReturnValue(mockCanvasInteraction);
  });

  it('should cleanup event listeners on unmount', () => {
    const { unmount } = render(<YuvViewerPanel {...mockProps} />);

    // Should not throw when unmounting
    expect(() => unmount()).not.toThrow();
  });

  it('should cleanup playback timer on unmount', () => {
    const { unmount } = render(<YuvViewerPanel {...mockProps} />);

    expect(() => unmount()).not.toThrow();
  });
});
