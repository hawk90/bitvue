import { useEffect, memo, lazy, Suspense, useCallback, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import "./components/TimelineFilmstrip.css";
import { WelcomeScreen } from "./components/WelcomeScreen";
import { TitleBar } from "./components/TitleBar";
import { StatusBar } from "./components/StatusBar";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { SelectionProvider } from "./contexts/SelectionContext";
import { ModeProvider, useMode } from "./contexts/ModeContext";
import {
  FrameDataProvider,
  FileStateProvider,
  CurrentFrameProvider,
  useFrameData,
  useCurrentFrame,
  useFileState,
} from "./contexts/StreamDataContext";
import { CompareProvider } from "./contexts/CompareContext";
import { YuvDiffProvider, useYuvDiff } from "./contexts/YuvDiffContext";
import { useTheme } from "./contexts/ThemeContext";
import { useLayout } from "./contexts/LayoutContext";
import { shouldShowTitleBar } from "./utils/platform";
import type { ThemeChangeEvent, FileOpenedEvent } from "./types/video";
import { isKeyframe } from "./types/video";
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
import { GoToFrameDialog } from "./components/GoToFrameDialog";

// Custom hooks for App logic
import { useAppFileOperations } from "./hooks/useAppFileOperations";
import { useKeyboardNavigation } from "./hooks/useKeyboardNavigation";
import { useAppDialogs } from "./hooks/useAppDialogs";
import { useRecentFiles } from "./hooks/useRecentFiles";

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

  // Theme changes
  useEffect(() => {
    const handleThemeChange = (e: Event) => {
      const themeEvent = e as ThemeChangeEvent;
      setTheme(themeEvent.detail);
    };
    window.addEventListener("menu-theme-change", handleThemeChange);
    return () => {
      window.removeEventListener("menu-theme-change", handleThemeChange);
    };
  }, [setTheme]);
  return (
    <ModeProvider>
      <FrameDataProvider>
        <FileStateProvider>
          <CurrentFrameProvider>
            <CompareProvider>
              <YuvDiffProvider>
                <AppContent />
              </YuvDiffProvider>
            </CompareProvider>
          </CurrentFrameProvider>
        </FileStateProvider>
      </FrameDataProvider>
    </ModeProvider>
  );
}

// Stable panel component wrappers — defined outside AppContent to avoid remounting
const StreamTreePanelWrapper = memo(function StreamTreePanelWrapper() {
  return <StreamTreePanel />;
});
const SyntaxDetailPanelWrapper = memo(function SyntaxDetailPanelWrapper() {
  return <SyntaxDetailPanel />;
});
const SelectionInfoPanelWrapper = memo(function SelectionInfoPanelWrapper() {
  return <SelectionInfoPanel />;
});
const UnitHexPanelWrapper = memo(function UnitHexPanelWrapper() {
  return <UnitHexPanel />;
});
const StatisticsPanelWrapper = memo(function StatisticsPanelWrapper() {
  return <StatisticsPanel />;
});

/**
 * Stable main view component — reads current frame data from context.
 * Defined outside AppContent so it has a stable identity and never causes remounting.
 */
const MainViewFromContext = memo(function MainViewFromContext() {
  const { frames } = useFrameData();
  const { currentFrameIndex, setCurrentFrameIndex } = useCurrentFrame();
  return (
    <YuvViewerPanel
      currentFrameIndex={currentFrameIndex}
      totalFrames={frames.length}
      onFrameChange={setCurrentFrameIndex}
    />
  );
});

/** Stable filmstrip panel — reads frames from context */
const FilmstripPanelFromContext = memo(function FilmstripPanelFromContext() {
  const { frames } = useFrameData();
  return <FilmstripPanel frames={frames} />;
});

/** Stable info panel — reads state from context */
const InfoPanelFromContext = memo(function InfoPanelFromContext() {
  const { frames } = useFrameData();
  const { currentFrameIndex } = useCurrentFrame();
  const { filePath } = useFileState();
  return (
    <InfoPanel
      filePath={filePath ?? undefined}
      frameCount={frames.length}
      currentFrameIndex={currentFrameIndex}
      currentFrame={frames[currentFrameIndex] || null}
    />
  );
});

/** Stable details panel — reads current frame from context */
const DetailsPanelFromContext = memo(function DetailsPanelFromContext() {
  const { frames } = useFrameData();
  const { currentFrameIndex } = useCurrentFrame();
  return <DetailsPanel frame={frames[currentFrameIndex] || null} />;
});

// Stable top panels config
const TOP_PANELS = [
  {
    id: "filmstrip",
    title: "Filmstrip",
    component: FilmstripPanelFromContext,
    icon: "media",
  },
];

// Stable bottom row panels config
const BOTTOM_ROW_PANELS = [
  {
    id: "info",
    title: "Info",
    component: InfoPanelFromContext,
    icon: "info",
  },
  {
    id: "details",
    title: "Details",
    component: DetailsPanelFromContext,
    icon: "list-tree",
  },
  {
    id: "stats",
    title: "Stats",
    component: StatisticsPanelWrapper,
    icon: "graph",
  },
];

// Stable left panels config — never changes
const LEFT_PANELS = [
  {
    id: "stream",
    title: "Stream",
    component: StreamTreePanelWrapper,
    icon: "symbol-tree",
  },
  {
    id: "syntax",
    title: "Syntax",
    component: SyntaxDetailPanelWrapper,
    icon: "code",
  },
  {
    id: "selection",
    title: "Selection",
    component: SelectionInfoPanelWrapper,
    icon: "info",
  },
  {
    id: "hex",
    title: "Unit HEX",
    component: UnitHexPanelWrapper,
    icon: "file-code",
  },
];

/**
 * Main App Content component
 * Manages file operations, keyboard navigation, and UI state
 */
function AppContent() {
  const { frames } = useFrameData();
  const { loading, error, setFilePath, refreshFrames } = useFileState();
  const { currentFrameIndex, setCurrentFrameIndex } = useCurrentFrame();

  // GoToFrame dialog state
  const [showGoToFrame, setShowGoToFrame] = useState(false);

  // Layout context for save/load/reset
  const { saveLayout, loadLayout, resetLayout } = useLayout();

  // Recent files
  const { recentFiles, addRecentFile } = useRecentFiles();

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
  const { setMode, setActiveCodec, handleFKey, toggleOverlay, clearOverlays } =
    useMode();
  const { loadFile: loadDebugYuv } = useYuvDiff();

  const {
    fileInfo,
    setFileInfo,
    openError,
    handleOpenFile,
    handleCloseFile,
    handleOpenDependentFile,
  } = useAppFileOperations({
    onError: showErrorDialog,
    onCodecChange: setActiveCodec,
  });

  // ── Frame navigation callbacks ────────────────────────────────────────
  const onPreviousFrame = useCallback(() => {
    if (currentFrameIndex > 0) setCurrentFrameIndex(currentFrameIndex - 1);
  }, [currentFrameIndex, setCurrentFrameIndex]);

  const onNextFrame = useCallback(() => {
    if (frames.length > 0 && currentFrameIndex < frames.length - 1) {
      setCurrentFrameIndex(currentFrameIndex + 1);
    }
  }, [currentFrameIndex, frames.length, setCurrentFrameIndex]);

  const onFirstFrame = useCallback(() => {
    setCurrentFrameIndex(0);
  }, [setCurrentFrameIndex]);

  const onLastFrame = useCallback(() => {
    if (frames.length > 0) setCurrentFrameIndex(frames.length - 1);
  }, [frames.length, setCurrentFrameIndex]);

  // I-frame navigation
  const onPreviousKeyFrame = useCallback(() => {
    for (let i = currentFrameIndex - 1; i >= 0; i--) {
      const f = frames[i];
      if (f && isKeyframe(f.frame_type, f.key_frame)) {
        setCurrentFrameIndex(i);
        return;
      }
    }
  }, [currentFrameIndex, frames, setCurrentFrameIndex]);

  const onNextKeyFrame = useCallback(() => {
    for (let i = currentFrameIndex + 1; i < frames.length; i++) {
      const f = frames[i];
      if (f && isKeyframe(f.frame_type, f.key_frame)) {
        setCurrentFrameIndex(i);
        return;
      }
    }
  }, [currentFrameIndex, frames, setCurrentFrameIndex]);

  const onShowShortcuts = useCallback(
    () => setShowShortcuts(true),
    [setShowShortcuts],
  );

  const onGoToFrame = useCallback(() => setShowGoToFrame(true), []);

  const onSaveFrame = useCallback(() => {
    // Dispatch synthetic event to trigger export of current frame
    window.dispatchEvent(new CustomEvent("save-current-frame"));
  }, []);

  // Keyboard navigation
  useKeyboardNavigation({
    currentIndex: currentFrameIndex,
    totalFrames: frames.length,
    callbacks: {
      onPreviousFrame,
      onNextFrame,
      onFirstFrame,
      onLastFrame,
      onPreviousKeyFrame,
      onNextKeyFrame,
    },
    onShowShortcuts,
    onGoToFrame,
    onOpenFile: handleOpenFile,
    onCloseFile: handleCloseFile,
    onShowExport: useCallback(
      () => setShowExportDialog(true),
      [setShowExportDialog],
    ),
    onSaveFrame,
    onFKey: handleFKey,
  });

  // Tauri event listeners
  useEffect(() => {
    const unlisten = listen<FileOpenedEvent>("file-opened", async (event) => {
      setFileInfo(event.payload);
      setFilePath(event.payload.success ? event.payload.path : null);
      if (event.payload.success) {
        setCurrentFrameIndex(0);
        addRecentFile(event.payload.path);
        await refreshFrames();
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
          console.warn("Failed to unlisten from file-opened event:", err);
        });
    };
  }, [
    refreshFrames,
    setFileInfo,
    setFilePath,
    showErrorDialog,
    setCurrentFrameIndex,
    addRecentFile,
  ]);

  // Layout menu events
  useEffect(() => {
    const handleSaveLayout = () => saveLayout();
    const handleLoadLayout = () => loadLayout();
    const handleResetLayout = () => resetLayout();
    window.addEventListener("menu-save-layout", handleSaveLayout);
    window.addEventListener("menu-load-layout", handleLoadLayout);
    window.addEventListener("menu-reset-layout", handleResetLayout);
    return () => {
      window.removeEventListener("menu-save-layout", handleSaveLayout);
      window.removeEventListener("menu-load-layout", handleLoadLayout);
      window.removeEventListener("menu-reset-layout", handleResetLayout);
    };
  }, [saveLayout, loadLayout, resetLayout]);

  // Overlay toggle / clear events (from View → Info Overlays submenu)
  useEffect(() => {
    const handleToggleOverlay = (e: Event) => {
      const mode = (e as CustomEvent<string>).detail;
      if (mode)
        toggleOverlay(
          mode as import("./utils/codecModeRegistry").VisualizationMode,
        );
    };
    const handleClearOverlays = () => clearOverlays();
    window.addEventListener("menu-toggle-overlay", handleToggleOverlay);
    window.addEventListener("menu-clear-overlays", handleClearOverlays);
    return () => {
      window.removeEventListener("menu-toggle-overlay", handleToggleOverlay);
      window.removeEventListener("menu-clear-overlays", handleClearOverlays);
    };
  }, [toggleOverlay, clearOverlays]);

  // Recent files menu: open the selected path
  useEffect(() => {
    const handleOpenRecent = (e: Event) => {
      const path = (e as CustomEvent<string>).detail;
      if (path) void invoke("open_file", { path });
    };
    window.addEventListener("menu-open-recent-file", handleOpenRecent);
    return () => {
      window.removeEventListener("menu-open-recent-file", handleOpenRecent);
    };
  }, []);

  // File menu events
  useEffect(() => {
    const handleExportListener = () => setShowExportDialog(true);
    const handleOpenBitstream = () => {
      void handleOpenFile();
    };
    const handleOpenDebugYuv = async () => {
      const { open: openDialog } = await import("@tauri-apps/plugin-dialog");
      const selected = await openDialog({
        title: "Load Reference YUV File",
        filters: [{ name: "Raw YUV", extensions: ["yuv", "raw", "y4m"] }],
        multiple: false,
      });
      if (!selected || typeof selected !== "string") return;
      // Prompt for resolution from the file name heuristic (e.g. 1920x1080_420_8.yuv)
      const nameMatch = selected.match(/(\d{3,4})[x_](\d{3,4})/i);
      const w = nameMatch ? parseInt(nameMatch[1], 10) : 1920;
      const h = nameMatch ? parseInt(nameMatch[2], 10) : 1080;
      await loadDebugYuv({
        path: selected,
        width: w,
        height: h,
        format: "i420",
        bitdepth: 8,
      });
    };
    const handleCloseBitstream = () => {
      void handleCloseFile();
    };
    const handleOpenDependentBitstream = () => {
      void handleOpenDependentFile();
    };
    const handleQuit = () => {
      void invoke("close_window");
    };
    const handleShowShortcuts = () => setShowShortcuts(true);

    window.addEventListener("menu-open-bitstream", handleOpenBitstream);
    window.addEventListener("menu-open-as-av1", handleOpenBitstream);
    window.addEventListener("menu-open-as-hevc", handleOpenBitstream);
    window.addEventListener("menu-open-as-avc", handleOpenBitstream);
    window.addEventListener("menu-open-as-vp9", handleOpenBitstream);
    window.addEventListener("menu-open-as-vvc", handleOpenBitstream);
    window.addEventListener("menu-open-as-mpeg2", handleOpenBitstream);
    window.addEventListener(
      "menu-open-dependent",
      handleOpenDependentBitstream,
    );
    window.addEventListener("menu-close-bitstream", handleCloseBitstream);
    window.addEventListener("menu-quit", handleQuit);
    window.addEventListener("menu-shortcuts", handleShowShortcuts);
    window.addEventListener("menu-export", handleExportListener);
    const handleOpenDebugYuvEvent = () => void handleOpenDebugYuv();
    window.addEventListener("menu-open-debug-yuv", handleOpenDebugYuvEvent);
    return () => {
      window.removeEventListener("menu-open-bitstream", handleOpenBitstream);
      window.removeEventListener("menu-open-as-av1", handleOpenBitstream);
      window.removeEventListener("menu-open-as-hevc", handleOpenBitstream);
      window.removeEventListener("menu-open-as-avc", handleOpenBitstream);
      window.removeEventListener("menu-open-as-vp9", handleOpenBitstream);
      window.removeEventListener("menu-open-as-vvc", handleOpenBitstream);
      window.removeEventListener("menu-open-as-mpeg2", handleOpenBitstream);
      window.removeEventListener(
        "menu-open-dependent",
        handleOpenDependentBitstream,
      );
      window.removeEventListener("menu-close-bitstream", handleCloseBitstream);
      window.removeEventListener("menu-quit", handleQuit);
      window.removeEventListener("menu-shortcuts", handleShowShortcuts);
      window.removeEventListener("menu-export", handleExportListener);
      window.removeEventListener(
        "menu-open-debug-yuv",
        handleOpenDebugYuvEvent,
      );
    };
  }, [
    handleCloseFile,
    handleOpenDependentFile,
    handleOpenFile,
    loadDebugYuv,
    setShowExportDialog,
    setShowShortcuts,
  ]);

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
        color: "var(--text-muted, #888)",
      }}
    >
      <h2>No Frames Available</h2>
      <p>The file was opened but no frames could be loaded.</p>
      <p>Check the console for debug logs.</p>
      <button onClick={handleOpenFile}>Open Different File</button>
    </div>
  );

  // Use stable left panels config defined outside component
  const leftPanels = LEFT_PANELS;

  // Stable main view component — reads from context directly to avoid remounting
  const mainView = MainViewFromContext;

  // Stable top panels — component reads from context directly
  const topPanels = TOP_PANELS;

  // Stable bottom panels — components read from context directly
  const bottomRowPanels = BOTTOM_ROW_PANELS;

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
              onCloseFile={handleCloseFile}
              onQuit={() => invoke("close_window")}
              onShowShortcuts={() => setShowShortcuts(true)}
              onModeChange={setMode}
            />
          )}

          <div className="app-container">
            {fileInfo?.success && frames.length > 0
              ? mainContent
              : fileInfo?.success && frames.length === 0
                ? noFramesError
                : welcomeScreen}
          </div>

          {/* Status Bar */}
          <StatusBar
            fileInfo={fileInfo}
            frameCount={frames.length}
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

      {/* Go To Frame Dialog */}
      <GoToFrameDialog
        isOpen={showGoToFrame}
        onClose={() => setShowGoToFrame(false)}
        currentIndex={currentFrameIndex}
        totalFrames={frames.length}
        onGoTo={setCurrentFrameIndex}
      />
    </SelectionProvider>
  );
}

export default memo(App);
