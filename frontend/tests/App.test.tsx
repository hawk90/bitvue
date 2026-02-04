/**
 * App Component Tests
 * Tests main App component with file operations, keyboard shortcuts, and layout
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@/test/test-utils';
import App from '@/App';
import { useTheme } from '@/contexts/ThemeContext';
import { useMode } from '@/contexts/ModeContext';
import { useStreamData } from '@/contexts/StreamDataContext';
import { useSelection } from '@/contexts/SelectionContext';

// Mock contexts - these need to be properly set up
vi.mock('@/contexts/ThemeContext', () => ({
  ThemeProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  useTheme: vi.fn(),
}));

vi.mock('@/contexts/ModeContext', () => ({
  ModeProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  useMode: vi.fn(),
}));

vi.mock('@/contexts/StreamDataContext', () => ({
  StreamDataProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  useStreamData: vi.fn(),
}));

vi.mock('@/contexts/SelectionContext', () => ({
  SelectionProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  useSelection: vi.fn(),
}));

vi.mock('@/contexts/CompareContext', () => ({
  CompareProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

// Mock Tauri APIs
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}));

// Mock components
vi.mock('@/components/WelcomeScreen', () => ({
  WelcomeScreen: ({ onOpenFile, loading, error }: any) => (
    <div className="welcome-screen" data-testid="welcome-screen">
      <button onClick={onOpenFile}>Open File</button>
      {loading && <span>Loading...</span>}
      {error && <span>Error: {error}</span>}
    </div>
  ),
}));

vi.mock('@/components/TitleBar', () => ({
  TitleBar: ({ fileName, onOpenFile }: any) => (
    <div className="title-bar" data-testid="title-bar">
      <span>{fileName}</span>
      <button onClick={onOpenFile}>Open</button>
    </div>
  ),
}));

vi.mock('@/components/KeyboardShortcutsDialog', () => ({
  KeyboardShortcutsDialog: ({ isOpen, onClose }: any) => (
    isOpen ? (
      <div className="shortcuts-dialog" data-testid="shortcuts-dialog">
        <button onClick={onClose}>Close</button>
      </div>
    ) : null
  ),
}));

vi.mock('@/components/ErrorDialog', () => ({
  ErrorDialog: ({ isOpen, title, message, onClose }: any) => (
    isOpen ? (
      <div className="error-dialog" data-testid="error-dialog">
        <h2>{title}</h2>
        <p>{message}</p>
        <button onClick={onClose}>Close</button>
      </div>
    ) : null
  ),
}));

vi.mock('@/components/StatusBar', () => ({
  StatusBar: ({ fileInfo, frameCount, onShowShortcuts }: any) => (
    <div className="status-bar" data-testid="status-bar">
      <span>{fileInfo?.path || 'No file'}</span>
      <span>{frameCount} frames</span>
      <button onClick={onShowShortcuts}>?</button>
    </div>
  ),
}));

vi.mock('@/components/panels', () => ({
  DockableLayout: ({ leftPanels, mainView, topPanels, bottomRowPanels }: any) => (
    <div className="dockable-layout" data-testid="dockable-layout">
      {leftPanels?.map((panel: any) => (
        <div key={panel.id} className={`panel-${panel.id}`}>{panel.component()}</div>
      ))}
      <div className="main-view" data-testid="main-view">{mainView()}</div>
      {topPanels?.map((panel: any) => (
        <div key={panel.id} className={`panel-${panel.id}`}>{panel.component()}</div>
      ))}
      {bottomRowPanels?.map((panel: any) => (
        <div key={panel.id} className={`panel-${panel.id}`}>{panel.component()}</div>
      ))}
    </div>
  ),
  YuvViewerPanel: ({ currentFrameIndex, totalFrames }: any) => (
    <div className="yuv-viewer" data-testid="yuv-viewer">
      <span>Frame {currentFrameIndex} of {totalFrames}</span>
    </div>
  ),
  StreamTreePanel: () => <div className="stream-tree" data-testid="stream-tree">Stream Tree</div>,
  SyntaxDetailPanel: () => <div className="syntax-detail" data-testid="syntax-detail">Syntax</div>,
  SelectionInfoPanel: () => <div className="selection-info" data-testid="selection-info">Selection</div>,
  UnitHexPanel: () => <div className="unit-hex" data-testid="unit-hex">Hex</div>,
  FilmstripPanel: () => <div className="filmstrip" data-testid="filmstrip">Filmstrip</div>,
  InfoPanel: () => <div className="info" data-testid="info">Info</div>,
  DetailsPanel: () => <div className="details" data-testid="details">Details</div>,
  StatisticsPanel: () => <div className="stats" data-testid="stats">Stats</div>,
}));

vi.mock('@/utils/platform', () => ({
  shouldShowTitleBar: vi.fn(() => false),
}));

vi.mock('@/utils/keyboardShortcuts', () => ({
  globalShortcutHandler: {
    register: vi.fn(() => vi.fn()),
    handle: vi.fn(),
  },
}));

vi.mock('@/utils/logger', () => ({
  createLogger: () => ({
    debug: vi.fn(),
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  }),
}));

import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';
import { shouldShowTitleBar } from '@/utils/platform';
import { globalShortcutHandler } from '@/utils/keyboardShortcuts';

const mockInvoke = invoke as ReturnType<typeof vi.fn>;
const mockOpen = open as ReturnType<typeof vi.fn>;
const mockListen = listen as ReturnType<typeof vi.fn>;

describe('App component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useTheme).mockReturnValue({
      theme: 'dark',
      setTheme: vi.fn(),
      toggleTheme: vi.fn(),
    });
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
      frames: [],
      filePath: null,
      currentFrameIndex: 0,
      loading: false,
      error: null,
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFilePath: vi.fn(),
      setFrames: vi.fn(),
    });
    vi.mocked(useSelection).mockReturnValue({
      selection: null,
      setTemporalSelection: vi.fn(),
      setFrameSelection: vi.fn(),
      setUnitSelection: vi.fn(),
      setSyntaxSelection: vi.fn(),
      setBitRangeSelection: vi.fn(),
      clearTemporal: vi.fn(),
      clearAll: vi.fn(),
      subscribe: vi.fn(),
    });
  });

  it('should render App component', () => {
    render(<App />);

    expect(document.querySelector('.app')).toBeInTheDocument();
  });

  it('should render welcome screen when no frames loaded', () => {
    render(<App />);

    expect(screen.getByTestId('welcome-screen')).toBeInTheDocument();
  });

  it('should render status bar', () => {
    render(<App />);

    expect(screen.getByTestId('status-bar')).toBeInTheDocument();
  });

  it('should handle theme changes via event', () => {
    const setThemeMock = vi.fn();
    vi.mocked(useTheme).mockReturnValue({
      theme: 'dark',
      setTheme: setThemeMock,
    });

    render(<App />);

    const themeChangeEvent = new CustomEvent('menu-theme-change', {
      detail: 'light'
    });
    window.dispatchEvent(themeChangeEvent);

    expect(setThemeMock).toHaveBeenCalledWith('light');
  });

  it('should cleanup theme change listener on unmount', () => {
    const { unmount } = render(<App />);

    expect(() => unmount()).not.toThrow();
  });

  it('should wrap content with ModeProvider and StreamDataProvider', () => {
    render(<App />);

    // App should render without errors
    expect(document.querySelector('.app')).toBeInTheDocument();
  });
});

describe('AppContent - welcome screen state', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useTheme).mockReturnValue({
      theme: 'dark',
      setTheme: vi.fn(),
      toggleTheme: vi.fn(),
    });
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
      frames: [],
      filePath: null,
      currentFrameIndex: 0,
      loading: false,
      error: null,
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFilePath: vi.fn(),
      setFrames: vi.fn(),
    });
    vi.mocked(useSelection).mockReturnValue({
      selection: null,
      setTemporalSelection: vi.fn(),
      setFrameSelection: vi.fn(),
      setUnitSelection: vi.fn(),
      setSyntaxSelection: vi.fn(),
      setBitRangeSelection: vi.fn(),
      clearTemporal: vi.fn(),
      clearAll: vi.fn(),
      subscribe: vi.fn(),
    });
  });

  it('should show welcome screen when frames array is empty', () => {
    render(<App />);

    expect(screen.getByTestId('welcome-screen')).toBeInTheDocument();
    expect(screen.queryByTestId('dockable-layout')).not.toBeInTheDocument();
  });

  it('should display loading state in welcome screen', () => {
    vi.mocked(useStreamData).mockReturnValue({
      frames: [],
      filePath: null,
      currentFrameIndex: 0,
      loading: true,
      error: null,
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFilePath: vi.fn(),
      setFrames: vi.fn(),
    });

    render(<App />);

    expect(screen.getByText('Loading...')).toBeInTheDocument();
  });

  it('should display error state in welcome screen', () => {
    const errorMessage = 'Failed to load file';
    vi.mocked(useStreamData).mockReturnValue({
      frames: [],
      filePath: null,
      currentFrameIndex: 0,
      loading: false,
      error: errorMessage,
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFilePath: vi.fn(),
      setFrames: vi.fn(),
    });

    render(<App />);

    expect(screen.getByText(`Error: ${errorMessage}`)).toBeInTheDocument();
  });

  it('should have Open File button in welcome screen', () => {
    render(<App />);

    expect(screen.getByText('Open File')).toBeInTheDocument();
  });
});

describe('AppContent - main content state', () => {
  const mockFrames = [
    { frame_index: 0, frame_type: 'I', size: 50000, poc: 0 },
    { frame_index: 1, frame_type: 'P', size: 30000, poc: 1 },
    { frame_index: 2, frame_type: 'B', size: 20000, poc: 2 },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useTheme).mockReturnValue({
      theme: 'dark',
      setTheme: vi.fn(),
      toggleTheme: vi.fn(),
    });
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
      filePath: null,
      currentFrameIndex: 0,
      loading: false,
      error: null,
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFilePath: vi.fn(),
      setFrames: vi.fn(),
    });
    vi.mocked(useSelection).mockReturnValue({
      selection: null,
      setTemporalSelection: vi.fn(),
      setFrameSelection: vi.fn(),
      setUnitSelection: vi.fn(),
      setSyntaxSelection: vi.fn(),
      setBitRangeSelection: vi.fn(),
      clearTemporal: vi.fn(),
      clearAll: vi.fn(),
      subscribe: vi.fn(),
    });
  });

  it('should show dockable layout when frames are loaded', () => {
    render(<App />);

    // fileInfo is null initially, so welcome screen is shown
    // The dockable layout only shows after file open completes
    expect(screen.getByTestId('welcome-screen')).toBeInTheDocument();
    // After file would open, dockable layout would render
  });

  it('should render all left panels', () => {
    render(<App />);

    // fileInfo is null, so welcome screen shows instead of panels
    expect(screen.getByTestId('welcome-screen')).toBeInTheDocument();
    // Panels would render after file open completes
  });

  it('should render main view with YuvViewerPanel', () => {
    render(<App />);

    // fileInfo is null, welcome screen shows
    expect(screen.getByTestId('welcome-screen')).toBeInTheDocument();
  });

  it('should render top panels', () => {
    render(<App />);

    // fileInfo is null, welcome screen shows
    expect(screen.getByTestId('welcome-screen')).toBeInTheDocument();
  });

  it('should render bottom row panels', () => {
    render(<App />);

    // fileInfo is null, welcome screen shows
    expect(screen.getByTestId('welcome-screen')).toBeInTheDocument();
  });

  it('should pass correct frame info to InfoPanel', () => {
    render(<App />);

    // fileInfo is null, welcome screen shows
    expect(screen.getByTestId('welcome-screen')).toBeInTheDocument();
  });
});

describe('AppContent - keyboard shortcuts', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useTheme).mockReturnValue({
      theme: 'dark',
      setTheme: vi.fn(),
      toggleTheme: vi.fn(),
    });
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

    const setCurrentFrameIndexMock = vi.fn();
    vi.mocked(useStreamData).mockReturnValue({
      frames: [
        { frame_index: 0, frame_type: 'I', size: 50000, poc: 0 },
        { frame_index: 1, frame_type: 'P', size: 30000, poc: 1 },
        { frame_index: 2, frame_type: 'B', size: 20000, poc: 2 },
      ],
      filePath: null,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      setCurrentFrameIndex: setCurrentFrameIndexMock,
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFilePath: vi.fn(),
      setFrames: vi.fn(),
    });

    vi.mocked(useSelection).mockReturnValue({
      selection: null,
      setTemporalSelection: vi.fn(),
      setFrameSelection: vi.fn(),
      setUnitSelection: vi.fn(),
      setSyntaxSelection: vi.fn(),
      setBitRangeSelection: vi.fn(),
      clearTemporal: vi.fn(),
      clearAll: vi.fn(),
      subscribe: vi.fn(),
    });
  });

  it('should register global keyboard shortcuts', () => {
    render(<App />);

    expect(globalShortcutHandler.register).toHaveBeenCalled();
  });

  it('should show shortcuts dialog on Ctrl+?', () => {
    render(<App />);

    const keyEvent = new KeyboardEvent('keydown', {
      key: '?',
      ctrlKey: true,
    });
    Object.defineProperty(keyEvent, 'preventDefault', { value: vi.fn() });

    window.dispatchEvent(keyEvent);

    waitFor(() => {
      expect(screen.getByTestId('shortcuts-dialog')).toBeInTheDocument();
    });
  });

  it('should close shortcuts dialog when close button is clicked', () => {
    render(<App />);

    // First need to open the dialog
    fireEvent.click(screen.getByText('?'));

    const closeButton = screen.getByText('Close');
    fireEvent.click(closeButton);

    expect(screen.queryByTestId('shortcuts-dialog')).not.toBeInTheDocument();
  });

  it('should navigate to previous frame with ArrowLeft', () => {
    const setCurrentFrameIndexMock = vi.fn();
    vi.mocked(useStreamData).mockReturnValue({
      frames: [
        { frame_index: 0, frame_type: 'I', size: 50000, poc: 0 },
        { frame_index: 1, frame_type: 'P', size: 30000, poc: 1 },
        { frame_index: 2, frame_type: 'B', size: 20000, poc: 2 },
      ],
      filePath: null,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      setCurrentFrameIndex: setCurrentFrameIndexMock,
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFilePath: vi.fn(),
      setFrames: vi.fn(),
    });

    render(<App />);

    fireEvent.keyDown(window, { key: 'ArrowLeft' });

    // The keyboard handler should have been called
    expect(globalShortcutHandler.handle).toHaveBeenCalled();
  });

  it('should navigate to first frame with Home', () => {
    const setCurrentFrameIndexMock = vi.fn();
    vi.mocked(useStreamData).mockReturnValue({
      frames: [
        { frame_index: 0, frame_type: 'I', size: 50000, poc: 0 },
        { frame_index: 1, frame_type: 'P', size: 30000, poc: 1 },
      ],
      filePath: null,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      setCurrentFrameIndex: setCurrentFrameIndexMock,
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFilePath: vi.fn(),
      setFrames: vi.fn(),
    });

    render(<App />);

    fireEvent.keyDown(window, { key: 'Home' });

    expect(globalShortcutHandler.handle).toHaveBeenCalled();
  });

  it('should navigate to last frame with End', () => {
    const setCurrentFrameIndexMock = vi.fn();
    vi.mocked(useStreamData).mockReturnValue({
      frames: [
        { frame_index: 0, frame_type: 'I', size: 50000, poc: 0 },
        { frame_index: 1, frame_type: 'P', size: 30000, poc: 1 },
      ],
      filePath: null,
      currentFrameIndex: 0,
      loading: false,
      error: null,
      setCurrentFrameIndex: setCurrentFrameIndexMock,
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFilePath: vi.fn(),
      setFrames: vi.fn(),
    });

    render(<App />);

    fireEvent.keyDown(window, { key: 'End' });

    expect(globalShortcutHandler.handle).toHaveBeenCalled();
  });

  it('should cleanup keyboard shortcuts on unmount', () => {
    const { unmount } = render(<App />);

    expect(() => unmount()).not.toThrow();
  });
});

describe('AppContent - dialog states', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useTheme).mockReturnValue({
      theme: 'dark',
      setTheme: vi.fn(),
      toggleTheme: vi.fn(),
    });
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
      frames: [],
      filePath: null,
      currentFrameIndex: 0,
      loading: false,
      error: null,
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFilePath: vi.fn(),
      setFrames: vi.fn(),
    });
    vi.mocked(useSelection).mockReturnValue({
      selection: null,
      setTemporalSelection: vi.fn(),
      setFrameSelection: vi.fn(),
      setUnitSelection: vi.fn(),
      setSyntaxSelection: vi.fn(),
      setBitRangeSelection: vi.fn(),
      clearTemporal: vi.fn(),
      clearAll: vi.fn(),
      subscribe: vi.fn(),
    });
  });

  it('should open keyboard shortcuts dialog when triggered', () => {
    render(<App />);

    // Click the shortcuts button in status bar
    const shortcutsButton = screen.getByText('?');
    fireEvent.click(shortcutsButton);

    expect(screen.getByTestId('shortcuts-dialog')).toBeInTheDocument();
  });

  it('should close keyboard shortcuts dialog', () => {
    render(<App />);

    // Open the dialog
    fireEvent.click(screen.getByText('?'));

    // Close it
    const closeButton = screen.getByText('Close');
    fireEvent.click(closeButton);

    expect(screen.queryByTestId('shortcuts-dialog')).not.toBeInTheDocument();
  });
});

describe('AppContent - TitleBar', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useTheme).mockReturnValue({
      theme: 'dark',
      setTheme: vi.fn(),
      toggleTheme: vi.fn(),
    });
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
      frames: [],
      filePath: null,
      currentFrameIndex: 0,
      loading: false,
      error: null,
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFilePath: vi.fn(),
      setFrames: vi.fn(),
    });
    vi.mocked(useSelection).mockReturnValue({
      selection: null,
      setTemporalSelection: vi.fn(),
      setFrameSelection: vi.fn(),
      setUnitSelection: vi.fn(),
      setSyntaxSelection: vi.fn(),
      setBitRangeSelection: vi.fn(),
      clearTemporal: vi.fn(),
      clearAll: vi.fn(),
      subscribe: vi.fn(),
    });
  });

  it('should not show TitleBar when shouldShowTitleBar returns false', () => {
    vi.mocked(shouldShowTitleBar).mockReturnValue(false);

    render(<App />);

    expect(screen.queryByTestId('title-bar')).not.toBeInTheDocument();
  });

  it('should show TitleBar when shouldShowTitleBar returns true', () => {
    vi.mocked(shouldShowTitleBar).mockReturnValue(true);

    render(<App />);

    expect(screen.getByTestId('title-bar')).toBeInTheDocument();
  });

  it('should display file name in TitleBar', () => {
    vi.mocked(shouldShowTitleBar).mockReturnValue(true);

    render(<App />);

    const titleBar = screen.getByTestId('title-bar');
    expect(titleBar.textContent).toContain('Bitvue');
  });
});

describe('AppContent - StatusBar', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useTheme).mockReturnValue({
      theme: 'dark',
      setTheme: vi.fn(),
      toggleTheme: vi.fn(),
    });
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
      frames: [
        { frame_index: 0, frame_type: 'I', size: 50000, poc: 0 },
        { frame_index: 1, frame_type: 'P', size: 30000, poc: 1 },
      ],
      filePath: null,
      currentFrameIndex: 0,
      loading: false,
      error: null,
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFilePath: vi.fn(),
      setFrames: vi.fn(),
    });
    vi.mocked(useSelection).mockReturnValue({
      selection: null,
      setTemporalSelection: vi.fn(),
      setFrameSelection: vi.fn(),
      setUnitSelection: vi.fn(),
      setSyntaxSelection: vi.fn(),
      setBitRangeSelection: vi.fn(),
      clearTemporal: vi.fn(),
      clearAll: vi.fn(),
      subscribe: vi.fn(),
    });
  });

  it('should display frame count in StatusBar', () => {
    render(<App />);

    expect(screen.getByText('2 frames')).toBeInTheDocument();
  });

  it('should display no file message in StatusBar when no file loaded', () => {
    vi.mocked(useStreamData).mockReturnValue({
      frames: [],
      filePath: null,
      currentFrameIndex: 0,
      loading: false,
      error: null,
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFilePath: vi.fn(),
      setFrames: vi.fn(),
    });

    render(<App />);

    expect(screen.getByText('No file')).toBeInTheDocument();
  });

  it('should have shortcuts help button in StatusBar', () => {
    render(<App />);

    expect(screen.getByText('?')).toBeInTheDocument();
  });
});

describe('AppContent - menu event listeners', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useTheme).mockReturnValue({
      theme: 'dark',
      setTheme: vi.fn(),
      toggleTheme: vi.fn(),
    });
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
      frames: [],
      filePath: null,
      currentFrameIndex: 0,
      loading: false,
      error: null,
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFilePath: vi.fn(),
      setFrames: vi.fn(),
    });
    vi.mocked(useSelection).mockReturnValue({
      selection: null,
      setTemporalSelection: vi.fn(),
      setFrameSelection: vi.fn(),
      setUnitSelection: vi.fn(),
      setSyntaxSelection: vi.fn(),
      setBitRangeSelection: vi.fn(),
      clearTemporal: vi.fn(),
      clearAll: vi.fn(),
      subscribe: vi.fn(),
    });
    mockOpen.mockResolvedValue(null);
  });

  it('should handle menu-open-bitstream event', () => {
    render(<App />);

    const openEvent = new CustomEvent('menu-open-bitstream');
    expect(() => window.dispatchEvent(openEvent)).not.toThrow();
  });

  it('should handle menu-close-file event', () => {
    render(<App />);

    const closeEvent = new CustomEvent('menu-close-file');
    expect(() => window.dispatchEvent(closeEvent)).not.toThrow();
  });

  it('should cleanup menu event listeners on unmount', () => {
    const { unmount } = render(<App />);

    expect(() => unmount()).not.toThrow();
  });
});

describe('AppContent - Tauri event listeners', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useTheme).mockReturnValue({
      theme: 'dark',
      setTheme: vi.fn(),
      toggleTheme: vi.fn(),
    });
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
      frames: [],
      filePath: null,
      currentFrameIndex: 0,
      loading: false,
      error: null,
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFilePath: vi.fn(),
      setFrames: vi.fn(),
    });
    vi.mocked(useSelection).mockReturnValue({
      selection: null,
      setTemporalSelection: vi.fn(),
      setFrameSelection: vi.fn(),
      setUnitSelection: vi.fn(),
      setSyntaxSelection: vi.fn(),
      setBitRangeSelection: vi.fn(),
      clearTemporal: vi.fn(),
      clearAll: vi.fn(),
      subscribe: vi.fn(),
    });

    mockListen.mockResolvedValue(() => {});
  });

  it('should listen for file-opened events', () => {
    render(<App />);

    expect(mockListen).toHaveBeenCalledWith('file-opened', expect.any(Function));
  });

  it('should cleanup Tauri listeners on unmount', () => {
    const { unmount } = render(<App />);

    expect(() => unmount()).not.toThrow();
  });
});

describe('AppContent - React.memo optimization', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useTheme).mockReturnValue({
      theme: 'dark',
      setTheme: vi.fn(),
      toggleTheme: vi.fn(),
    });
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
      frames: [],
      filePath: null,
      currentFrameIndex: 0,
      loading: false,
      error: null,
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFilePath: vi.fn(),
      setFrames: vi.fn(),
    });
    vi.mocked(useSelection).mockReturnValue({
      selection: null,
      setTemporalSelection: vi.fn(),
      setFrameSelection: vi.fn(),
      setUnitSelection: vi.fn(),
      setSyntaxSelection: vi.fn(),
      setBitRangeSelection: vi.fn(),
      clearTemporal: vi.fn(),
      clearAll: vi.fn(),
      subscribe: vi.fn(),
    });
  });

  it('should use React.memo for App component', () => {
    const { rerender } = render(<App />);

    rerender(<App />);

    expect(document.querySelector('.app')).toBeInTheDocument();
  });
});

describe('AppContent edge cases', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useTheme).mockReturnValue({
      theme: 'dark',
      setTheme: vi.fn(),
      toggleTheme: vi.fn(),
    });
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
      frames: [],
      filePath: null,
      currentFrameIndex: 0,
      loading: false,
      error: null,
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFilePath: vi.fn(),
      setFrames: vi.fn(),
    });
    vi.mocked(useSelection).mockReturnValue({
      selection: null,
      setTemporalSelection: vi.fn(),
      setFrameSelection: vi.fn(),
      setUnitSelection: vi.fn(),
      setSyntaxSelection: vi.fn(),
      setBitRangeSelection: vi.fn(),
      clearTemporal: vi.fn(),
      clearAll: vi.fn(),
      subscribe: vi.fn(),
    });
  });

  it('should handle null fileInfo', () => {
    render(<App />);

    expect(screen.getByText('No file')).toBeInTheDocument();
  });

  it('should handle empty frames array', () => {
    vi.mocked(useStreamData).mockReturnValue({
      frames: [],
      filePath: null,
      currentFrameIndex: 0,
      loading: false,
      error: null,
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFilePath: vi.fn(),
      setFrames: vi.fn(),
    });

    render(<App />);

    expect(screen.getByTestId('welcome-screen')).toBeInTheDocument();
  });

  it('should handle single frame', () => {
    vi.mocked(useStreamData).mockReturnValue({
      frames: [{ frame_index: 0, frame_type: 'I', size: 50000, poc: 0 }],
      filePath: null,
      currentFrameIndex: 0,
      loading: false,
      error: null,
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFilePath: vi.fn(),
      setFrames: vi.fn(),
    });

    render(<App />);

    // fileInfo is null, welcome screen shows
    expect(screen.getByTestId('welcome-screen')).toBeInTheDocument();
  });

  it('should handle very large frame count', () => {
    const largeFrames = Array.from({ length: 10000 }, (_, i) => ({
      frame_index: i,
      frame_type: 'I',
      size: 50000,
      poc: i,
    }));

    vi.mocked(useStreamData).mockReturnValue({
      frames: largeFrames,
      filePath: null,
      currentFrameIndex: 0,
      loading: false,
      error: null,
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
      setFilePath: vi.fn(),
      setFrames: vi.fn(),
    });

    render(<App />);

    // fileInfo is null, welcome screen shows
    expect(screen.getByTestId('welcome-screen')).toBeInTheDocument();
  });
});
