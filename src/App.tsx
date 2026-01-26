import { useState, useCallback, useEffect, memo } from "react";
import { invoke } from '@tauri-apps/api/core';
import { listen } from "@tauri-apps/api/event";
import { open } from '@tauri-apps/plugin-dialog';
import "./App.css";
import "./components/TimelineFilmstrip.css";
import { WelcomeScreen } from "./components/WelcomeScreen";
import { TitleBar } from "./components/TitleBar";
import { KeyboardShortcutsDialog } from "./components/KeyboardShortcutsDialog";
import { ErrorDialog } from "./components/ErrorDialog";
import { StatusBar } from "./components/StatusBar";
import { ExportDialog } from "./components/ExportDialog";
import { globalShortcutHandler, type ShortcutConfig } from "./utils/keyboardShortcuts";
import { SelectionProvider } from "./contexts/SelectionContext";
import { ModeProvider } from "./contexts/ModeContext";
import { StreamDataProvider, useStreamData } from "./contexts/StreamDataContext";
import { CompareProvider } from "./contexts/CompareContext";
import { useTheme } from "./contexts/ThemeContext";
import { shouldShowTitleBar } from "./utils/platform";
import { createLogger } from "./utils/logger";
import type { FileInfo, ThemeChangeEvent, FileOpenedEvent } from "./types/video";
import {
  DockableLayout,
  FilmstripPanel,
  YuvViewerPanel,
  StreamTreePanel,
  SyntaxDetailPanel,
  SelectionInfoPanel,
  UnitHexPanel,
  StatisticsPanel,
  InfoPanel,
  DetailsPanel,
} from "./components/panels";

const logger = createLogger('App');

function App() {
  const { setTheme } = useTheme();
  const [fileInfo, setFileInfo] = useState<FileInfo | null>(null);

  // Theme changes
  useEffect(() => {
    const handleThemeChange = (e: Event) => {
      const themeEvent = e as ThemeChangeEvent;
      setTheme(themeEvent.detail);
    };
    window.addEventListener('menu-theme-change', handleThemeChange);
    return () => {
      window.removeEventListener('menu-theme-change', handleThemeChange);
    };
  }, [setTheme]);

  return (
    <ModeProvider>
      <StreamDataProvider>
        <CompareProvider>
          <AppContent fileInfo={fileInfo} setFileInfo={setFileInfo} />
        </CompareProvider>
      </StreamDataProvider>
    </ModeProvider>
  );
}

function AppContent({ fileInfo, setFileInfo }: { fileInfo: FileInfo | null; setFileInfo: (info: FileInfo | null) => void }) {
  const { frames, loading, error, currentFrameIndex, setCurrentFrameIndex, refreshFrames, clearData, setFilePath } = useStreamData();
  const [openError, setOpenError] = useState<string | null>(null);
  const [showShortcuts, setShowShortcuts] = useState(false);
  const [showExportDialog, setShowExportDialog] = useState(false);

  // Error dialog state
  const [errorDialog, setErrorDialog] = useState<{
    isOpen: boolean;
    title: string;
    message: string;
    details?: string;
    errorCode?: string;
  }>({
    isOpen: false,
    title: '',
    message: '',
  });

  // Show error dialog
  const showErrorDialog = useCallback((title: string, message: string, details?: string, errorCode?: string) => {
    setErrorDialog({
      isOpen: true,
      title,
      message,
      details,
      errorCode,
    });
  }, []);

  // Handle closing the current file
  const handleCloseFile = useCallback(async () => {
    try {
      await invoke('close_file');
      setFileInfo(null);
      setFilePath(null);
      clearData();
    } catch (err) {
      logger.error('Failed to close file:', err);
      showErrorDialog('Failed to Close File', err as string);
    }
  }, [setFileInfo, setFilePath, clearData, showErrorDialog]);

  const handleOpenFile = useCallback(async () => {
    try {
      setOpenError(null);

      const selected = await open({
        multiple: false,
        filters: [
          {
            name: 'Video Files',
            extensions: ['ivf', 'av1', 'hevc', 'h265', 'vvc', 'h266', 'mp4', 'mkv', 'webm', 'ts']
          },
          {
            name: 'All Files',
            extensions: ['*']
          }
        ]
      });

      if (selected && typeof selected === 'string') {
        logger.debug('Opening file:', selected);

        // Call the Tauri command to open the file
        const result = await invoke<FileInfo>('open_file', { path: selected });

        setFileInfo(result);
        setFilePath(result.success ? selected : null);

        if (result.success) {
          logger.info('File opened successfully');
          // Refresh frames after opening file
          await refreshFrames();
        } else {
          showErrorDialog('Failed to Open File', result.error || 'Unknown error', selected);
        }
      }
    } catch (err) {
      logger.error('Failed to open file:', err);
      showErrorDialog('Failed to Open File', err as string);
    }
  }, [refreshFrames, setFileInfo, setFilePath, showErrorDialog]);

  // File menu events
  useEffect(() => {
    const handleOpenBitstreamListener = handleOpenFile;
    const handleCloseFileListener = handleCloseFile;
    const handleExportListener = () => setShowExportDialog(true);

    window.addEventListener('menu-open-bitstream', handleOpenBitstreamListener);
    window.addEventListener('menu-close-file', handleCloseFileListener);
    window.addEventListener('menu-export', handleExportListener);

    return () => {
      window.removeEventListener('menu-open-bitstream', handleOpenBitstreamListener);
      window.removeEventListener('menu-close-file', handleCloseFileListener);
      window.removeEventListener('menu-export', handleExportListener);
    };
  }, [handleOpenFile, handleCloseFile]);

  // Tauri event listeners
  useEffect(() => {
    const unlisten = listen<FileOpenedEvent>("file-opened", async (event) => {
      setFileInfo(event.payload);
      setFilePath(event.payload.success ? event.payload.path : null);
      if (event.payload.success) {
        logger.info(`Opened: ${event.payload.path}`);
        // Refresh frames after opening file
        await refreshFrames();
      } else {
        showErrorDialog('Failed to Open File', event.payload.error || 'Unknown error', event.payload.path);
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [refreshFrames, setFileInfo, setFilePath, showErrorDialog]);

  // Keyboard shortcuts
  useEffect(() => {
    const unregister: (() => void)[] = [];

    // Register shortcuts
    const shortcuts: ShortcutConfig[] = [
      {
        key: '?',
        ctrl: true,
        meta: true,
        description: 'Show shortcuts',
        action: () => setShowShortcuts(true),
      },
      {
        key: 'ArrowLeft',
        description: 'Previous frame',
        action: () => {
          if (currentFrameIndex > 0) setCurrentFrameIndex(currentFrameIndex - 1);
        },
      },
      {
        key: 'ArrowRight',
        description: 'Next frame',
        action: () => {
          if (frames.length > 0 && currentFrameIndex < frames.length - 1) {
            setCurrentFrameIndex(currentFrameIndex + 1);
          }
        },
      },
      {
        key: 'Home',
        description: 'First frame',
        action: () => setCurrentFrameIndex(0),
      },
      {
        key: 'End',
        description: 'Last frame',
        action: () => {
          if (frames.length > 0) setCurrentFrameIndex(frames.length - 1);
        },
      },
    ];

    shortcuts.forEach(shortcut => {
      unregister.push(globalShortcutHandler.register(shortcut));
    });

    // Handle keyboard events
    const handleKeyDown = (e: KeyboardEvent) => {
      globalShortcutHandler.handle(e);
    };

    window.addEventListener('keydown', handleKeyDown);

    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      unregister.forEach(fn => fn());
    };
  }, [currentFrameIndex, frames.length, setCurrentFrameIndex]);

  const welcomeScreen = (
    <WelcomeScreen
      onOpenFile={handleOpenFile}
      loading={loading}
      error={openError || error}
    />
  );

  // Error state when file is opened but no frames are loaded
  const noFramesError = (
    <div style={{
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      justifyContent: 'center',
      height: '100vh',
      gap: '16px',
      color: '#888'
    }}>
      <h2>No Frames Available</h2>
      <p>The file was opened but no frames could be loaded.</p>
      <p>Check the console for debug logs.</p>
      <button onClick={handleOpenFile}>Open Different File</button>
    </div>
  );

  // Main content when file is loaded
  const mainContent = frames.length > 0 ? (
    <DockableLayout
      leftPanels={[
        {
          id: 'stream',
          title: 'Stream',
          component: () => <StreamTreePanel />,
          icon: 'symbol-tree',
        },
        {
          id: 'syntax',
          title: 'Syntax',
          component: () => <SyntaxDetailPanel />,
          icon: 'code',
        },
        {
          id: 'selection',
          title: 'Selection',
          component: () => <SelectionInfoPanel />,
          icon: 'info',
        },
        {
          id: 'hex',
          title: 'Unit HEX',
          component: () => <UnitHexPanel />,
          icon: 'file-code',
        },
      ]}
      mainView={() => (
        <YuvViewerPanel
          currentFrameIndex={currentFrameIndex}
          totalFrames={frames.length}
          onFrameChange={setCurrentFrameIndex}
        />
      )}
      topPanels={[
        {
          id: 'filmstrip',
          title: 'Filmstrip',
          component: () => (
            <FilmstripPanel
              frames={frames}
            />
          ),
          icon: 'media',
        },
      ]}
      bottomRowPanels={[
        {
          id: 'info',
          title: 'Info',
          component: () => (
            <InfoPanel
              filePath={fileInfo?.path}
              frameCount={frames.length}
              currentFrameIndex={currentFrameIndex}
              currentFrame={frames[currentFrameIndex] || null}
            />
          ),
          icon: 'info',
        },
        {
          id: 'details',
          title: 'Details',
          component: () => (
            <DetailsPanel frame={frames[currentFrameIndex] || null} />
          ),
          icon: 'list-tree',
        },
        {
          id: 'stats',
          title: 'Stats',
          component: () => <StatisticsPanel />,
          icon: 'graph',
        },
      ]}
    />
  ) : null;

  return (
    <ModeProvider>
      <SelectionProvider>
        <div className="app">
        {/* Custom TitleBar for Windows/Linux only */}
        {shouldShowTitleBar() && (
          <TitleBar
            fileName={fileInfo?.path || 'Bitvue'}
            onOpenFile={handleOpenFile}
          />
        )}

        <div className="app-container">
          {(() => {
            // File opened successfully with frames
            if (fileInfo?.success && frames.length > 0) {
              return mainContent;
            }
            // File opened but no frames (error state)
            if (fileInfo?.success && frames.length === 0) {
              return noFramesError;
            }
            // No file opened (welcome screen)
            return welcomeScreen;
          })()}
        </div>

        {/* Status Bar */}
        <StatusBar
          fileInfo={fileInfo}
          frameCount={frames.length}
          branch="main"
          onShowShortcuts={() => setShowShortcuts(true)}
        />
      </div>

      {/* Keyboard Shortcuts Dialog */}
      <KeyboardShortcutsDialog
        isOpen={showShortcuts}
        onClose={() => setShowShortcuts(false)}
      />

      {/* Error Dialog */}
      <ErrorDialog
        isOpen={errorDialog.isOpen}
        title={errorDialog.title}
        message={errorDialog.message}
        details={errorDialog.details}
        errorCode={errorDialog.errorCode}
        onClose={() => setErrorDialog({ ...errorDialog, isOpen: false })}
      />

      {/* Export Dialog */}
      <ExportDialog
        isOpen={showExportDialog}
        onClose={() => setShowExportDialog(false)}
        frames={frames}
        codec={fileInfo?.codec}
        width={fileInfo?.width}
        height={fileInfo?.height}
      />
    </SelectionProvider>
    </ModeProvider>
  );
}

export default memo(App);
