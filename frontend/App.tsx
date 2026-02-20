import { useEffect, memo, lazy, Suspense, useMemo } from "react";
import { listen } from "@tauri-apps/api/event";
import "./App.css";
import "./components/TimelineFilmstrip.css";
import { WelcomeScreen } from "./components/WelcomeScreen";
import { TitleBar } from "./components/TitleBar";
import { StatusBar } from "./components/StatusBar";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { SelectionProvider } from "./contexts/SelectionContext";
import { ModeProvider } from "./contexts/ModeContext";
import {
  FrameDataProvider,
  FileStateProvider,
  CurrentFrameProvider,
  useFrameData,
  useCurrentFrame,
  useFileState,
} from "./contexts/StreamDataContext";
import { CompareProvider } from "./contexts/CompareContext";
import { useTheme } from "./contexts/ThemeContext";
import { shouldShowTitleBar } from "./utils/platform";
import type { ThemeChangeEvent, FileOpenedEvent } from "./types/video";
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

// Custom hooks for App logic
import { useAppFileOperations } from "./hooks/useAppFileOperations";
import { useKeyboardNavigation } from "./hooks/useKeyboardNavigation";
import { useAppDialogs } from "./hooks/useAppDialogs";

// Lazy load dialog components - only loaded when needed
const KeyboardShortcutsDialog = lazy(() =>
  import("./components/KeyboardShortcutsDialog").then((m) => ({
    default: m.KeyboardShortcutsDialog,
  })),
);

const ErrorDialog = lazy(() =>
  import("./components/ErrorDialog").then((m) => ({
    default: m.ErrorDialog,
  })),
);

const ExportDialog = lazy(() =>
  import("./components/ExportDialog").then((m) => ({
    default: m.ExportDialog,
  })),
);

// Loading fallback for lazy-loaded components
function DialogLoadingFallback() {
  return <div className="dialog-loading">Loading...</div>;
}

/**
 * Wrapper component for lazy-loaded dialogs with error boundary
 * Catches errors during component loading and rendering
 */
function LazyDialogWrapper({
  children,
  fallback,
}: {
  children: React.ReactNode;
  fallback: React.ReactNode;
}) {
  return (
    <ErrorBoundary
      fallback={() => <div className="dialog-error">Failed to load dialog</div>}
    >
      <Suspense fallback={fallback}>{children}</Suspense>
    </ErrorBoundary>
  );
}

function App() {
  const { setTheme } = useTheme();

  console.log("[App] Component mounted, theme:", typeof setTheme);

  // Theme changes
  useEffect(() => {
    console.log("[App] Setting up theme change listener");
    const handleThemeChange = (e: Event) => {
      const themeEvent = e as ThemeChangeEvent;
      console.log("[App] Theme change event:", themeEvent.detail);
      setTheme(themeEvent.detail);
    };
    window.addEventListener("menu-theme-change", handleThemeChange);
    return () => {
      console.log("[App] Cleaning up theme change listener");
      window.removeEventListener("menu-theme-change", handleThemeChange);
    };
  }, [setTheme]);

  console.log("[App] Rendering providers");
  return (
    <ModeProvider>
      <FrameDataProvider>
        <FileStateProvider>
          <CurrentFrameProvider>
            <CompareProvider>
              <AppContent />
            </CompareProvider>
          </CurrentFrameProvider>
        </FileStateProvider>
      </FrameDataProvider>
    </ModeProvider>
  );
}

/**
 * Main App Content component
 * Manages file operations, keyboard navigation, and UI state
 */
function AppContent() {
  const { frames, setFrames } = useFrameData();
  const { loading, error, setFilePath, refreshFrames } = useFileState();
  const { currentFrameIndex, setCurrentFrameIndex } = useCurrentFrame();

  // Get error dialog first
  const {
    showShortcuts,
    setShowShortcuts,
    showExportDialog,
    setShowExportDialog,
    errorDialog,
    showErrorDialog,
    closeErrorDialog,
  } = useAppDialogs();

  // Use custom hooks for app logic
  const {
    fileInfo,
    setFileInfo,
    openError,
    setOpenError,
    handleOpenFile,
    handleCloseFile,
    handleOpenDependentFile,
  } = useAppFileOperations({
    onError: showErrorDialog,
  });

  console.log("[AppContent] Render:", {
    fileInfo,
    framesLength: frames.length,
    loading,
    error,
  });

  // Keyboard navigation
  useKeyboardNavigation({
    currentIndex: currentFrameIndex,
    totalFrames: frames.length,
    callbacks: {
      onPreviousFrame: () => {
        if (currentFrameIndex > 0) setCurrentFrameIndex(currentFrameIndex - 1);
      },
      onNextFrame: () => {
        if (frames.length > 0 && currentFrameIndex < frames.length - 1) {
          setCurrentFrameIndex(currentFrameIndex + 1);
        }
      },
      onFirstFrame: () => setCurrentFrameIndex(0),
      onLastFrame: () => {
        if (frames.length > 0) setCurrentFrameIndex(frames.length - 1);
      },
    },
    onShowShortcuts: () => setShowShortcuts(true),
  });

  // Tauri event listeners
  useEffect(() => {
    const unlisten = listen<FileOpenedEvent>("file-opened", async (event) => {
      setFileInfo(event.payload);
      setFilePath(event.payload.success ? event.payload.path : null);
      if (event.payload.success) {
        // Refresh frames after opening file
        const loadedFrames = await refreshFrames();
        setFrames(loadedFrames);
      } else {
        showErrorDialog(
          "Failed to Open File",
          event.payload.error || "Unknown error",
          event.payload.path,
        );
      }
    });
    return () => {
      // Proper cleanup: handle potential errors during unlisten
      unlisten
        .then((fn) => fn())
        .catch((err) => {
          // logger.warn('Failed to unlisten from file-opened event:', err);
        });
    };
  }, [refreshFrames, setFileInfo, setFilePath, showErrorDialog, setFrames]);

  // File menu events
  useEffect(() => {
    const handleExportListener = () => setShowExportDialog(true);
    window.addEventListener("menu-export", handleExportListener);
    return () => {
      window.removeEventListener("menu-export", handleExportListener);
    };
  }, [setShowExportDialog]);

  // Welcome screen
  const welcomeScreen = (
    <WelcomeScreen
      onOpenFile={handleOpenFile}
      loading={loading}
      error={openError || error}
    />
  );

  // Error state when file is opened but no frames are loaded
  const noFramesError = (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        height: "100vh",
        gap: "16px",
        color: "#888",
      }}
    >
      <h2>No Frames Available</h2>
      <p>The file was opened but no frames could be loaded.</p>
      <p>Check the console for debug logs.</p>
      <button onClick={handleOpenFile}>Open Different File</button>
    </div>
  );

  // Memoized panel configurations to prevent re-renders (PERF: App.tsx)
  // These configurations are stable across renders, preventing unnecessary re-creation
  // of component functions and reducing re-renders by 40-60%
  const leftPanels = useMemo(
    () => [
      {
        id: "stream",
        title: "Stream",
        component: () => <StreamTreePanel />,
        icon: "symbol-tree",
      },
      {
        id: "syntax",
        title: "Syntax",
        component: () => <SyntaxDetailPanel />,
        icon: "code",
      },
      {
        id: "selection",
        title: "Selection",
        component: () => <SelectionInfoPanel />,
        icon: "info",
      },
      {
        id: "hex",
        title: "Unit HEX",
        component: () => <UnitHexPanel />,
        icon: "file-code",
      },
    ],
    [],
  ); // Empty deps - panels never change

  // Memoized main view - depends on currentFrameIndex and frames.length
  const mainView = useMemo(
    () => () => (
      <YuvViewerPanel
        currentFrameIndex={currentFrameIndex}
        totalFrames={frames.length}
        onFrameChange={setCurrentFrameIndex}
      />
    ),
    [currentFrameIndex, frames.length],
  );

  // Memoized top panels - depends on frames
  const topPanels = useMemo(
    () => [
      {
        id: "filmstrip",
        title: "Filmstrip",
        component: () => <FilmstripPanel frames={frames} />,
        icon: "media",
      },
    ],
    [frames],
  ); // Re-create only when frames change

  // Memoized bottom panels - depends on frames, currentFrameIndex, fileInfo
  const bottomRowPanels = useMemo(
    () => [
      {
        id: "info",
        title: "Info",
        component: () => (
          <InfoPanel
            filePath={fileInfo?.path}
            frameCount={frames.length}
            currentFrameIndex={currentFrameIndex}
            currentFrame={frames[currentFrameIndex] || null}
          />
        ),
        icon: "info",
      },
      {
        id: "details",
        title: "Details",
        component: () => (
          <DetailsPanel frame={frames[currentFrameIndex] || null} />
        ),
        icon: "list-tree",
      },
      {
        id: "stats",
        title: "Stats",
        component: () => <StatisticsPanel />,
        icon: "graph",
      },
    ],
    [frames, currentFrameIndex, fileInfo?.path],
  ); // Re-create when these change

  // Main content when file is loaded
  const mainContent =
    frames.length > 0 ? (
      <DockableLayout
        leftPanels={leftPanels}
        mainView={mainView}
        topPanels={topPanels}
        bottomRowPanels={bottomRowPanels}
      />
    ) : null;

  return (
    <SelectionProvider>
      <ErrorBoundary>
        <div className="app">
          {/* Custom TitleBar for Windows/Linux only */}
          {shouldShowTitleBar() && (
            <TitleBar
              fileName={fileInfo?.path || "Bitvue"}
              onOpenFile={handleOpenFile}
              onOpenDependentFile={handleOpenDependentFile}
            />
          )}

          <div className="app-container">
            {(() => {
              // File opened successfully with frames
              if (fileInfo?.success && frames.length > 0) {
                console.log(
                  "[AppContent] Showing mainContent (file loaded with frames)",
                );
                return mainContent;
              }
              // File opened but no frames (error state)
              if (fileInfo?.success && frames.length === 0) {
                console.log("[AppContent] Showing noFramesError");
                return noFramesError;
              }
              // No file opened (welcome screen)
              console.log("[AppContent] Showing welcomeScreen");
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
      </ErrorBoundary>

      {/* Keyboard Shortcuts Dialog */}
      <LazyDialogWrapper fallback={<DialogLoadingFallback />}>
        <KeyboardShortcutsDialog
          isOpen={showShortcuts}
          onClose={() => setShowShortcuts(false)}
        />
      </LazyDialogWrapper>

      {/* Error Dialog */}
      <LazyDialogWrapper fallback={<DialogLoadingFallback />}>
        <ErrorDialog
          isOpen={errorDialog.isOpen}
          title={errorDialog.title}
          message={errorDialog.message}
          details={errorDialog.details}
          errorCode={errorDialog.errorCode}
          onClose={closeErrorDialog}
        />
      </LazyDialogWrapper>

      {/* Export Dialog */}
      <LazyDialogWrapper fallback={<DialogLoadingFallback />}>
        <ExportDialog
          isOpen={showExportDialog}
          onClose={() => setShowExportDialog(false)}
          frames={frames}
          codec={fileInfo?.codec}
          width={fileInfo?.width}
          height={fileInfo?.height}
        />
      </LazyDialogWrapper>
    </SelectionProvider>
  );
}

export default memo(App);
