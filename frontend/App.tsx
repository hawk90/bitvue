import { useEffect, memo, lazy, Suspense, useCallback, useState } from "react";
import { closeWindow, showOpenDialog } from "./services/electronBridgeService";
import { useOpenFileStatus } from "./hooks/useOpenFileStatus";
import "./App.css";
import "./components/TimelineFilmstrip.css";
import { WelcomeScreen } from "./components/WelcomeScreen";
import { TitleBar } from "./components/TitleBar";
import { StatusBar } from "./components/StatusBar";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { SelectionProvider } from "./contexts/SelectionContext";
import { FrameSyncBridge } from "./contexts/FrameSyncBridge";
import { ModeProvider, useMode } from "./contexts/ModeContext";
import {
  FrameDataProvider,
  FileStateProvider,
  CurrentFrameProvider,
  useFrameData,
  useCurrentFrame,
  useFileState,
} from "./contexts/StreamDataContext";
import { CompareProvider, useCompare } from "./contexts/CompareContext";
import CompareWorkspace from "./components/CompareWorkspace/CompareWorkspace";
import { YuvDiffProvider, useYuvDiff } from "./contexts/YuvDiffContext";
import { SyntaxHexLinkProvider } from "./contexts/SyntaxHexLinkContext";
import { useTheme } from "./contexts/ThemeContext";
import { useLayout } from "./contexts/LayoutContext";
import { shouldShowTitleBar } from "./utils/platform";
import type { ThemeChangeEvent } from "./types/video";
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
  YuvDiffPanel,
  DiagnosticsPanel,
} from "./components/panels";
import { GoToFrameDialog } from "./components/GoToFrameDialog";

// Custom hooks for App logic
import { useAppFileOperations } from "./hooks/useAppFileOperations";
import { useKeyboardNavigation } from "./hooks/useKeyboardNavigation";
import { useAppDialogs } from "./hooks/useAppDialogs";
import { useRecentFiles } from "./hooks/useRecentFiles";
import { useExportEvidenceBundle } from "./hooks/useExportEvidenceBundle";

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

const LoadDebugYuvDialog = lazy(() =>
  import("./components/panels/LoadDebugYuvDialog").then((m) => ({
    default: m.LoadDebugYuvDialog,
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
    <SyntaxHexLinkProvider>
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
    </SyntaxHexLinkProvider>
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
const DiagnosticsPanelWrapper = memo(function DiagnosticsPanelWrapper() {
  return <DiagnosticsPanel />;
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

/** Stable compare workspace — reads workspace/frame state from CompareContext + the main
 *  stream's own frame position (stream A IS the primary stream the rest of the app already
 *  tracks, so frame-A navigation here drives the same `currentFrameIndex` everything else uses,
 *  not a disconnected second position). */
const CompareWorkspaceFromContext = memo(
  function CompareWorkspaceFromContext() {
    const { frames } = useFrameData();
    const { currentFrameIndex, setCurrentFrameIndex } = useCurrentFrame();
    const { framesB, currentFrameB, setFrameB } = useCompare();
    return (
      <CompareWorkspace
        framesA={frames}
        framesB={framesB}
        currentFrameA={currentFrameIndex}
        currentFrameB={currentFrameB}
        onFrameChangeA={setCurrentFrameIndex}
        onFrameChangeB={setFrameB}
      />
    );
  },
);

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

/** Stable YUV diff panel — reads frame index and provides jump callback */
const YuvDiffPanelFromContext = memo(function YuvDiffPanelFromContext() {
  const { currentFrameIndex, setCurrentFrameIndex } = useCurrentFrame();
  return (
    <YuvDiffPanel
      currentFrameIndex={currentFrameIndex}
      onJumpToFrame={setCurrentFrameIndex}
    />
  );
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
  {
    id: "diagnostics",
    title: "Diagnostics",
    component: DiagnosticsPanelWrapper,
    icon: "warning",
  },
];

// Stable left panels config — never changes
// Stream Tree is pinned (always visible) rather than a tab -- per the UX_PARITY_MATRIX.md W0
// wireframe spec, Tree and Inspectors are separate simultaneous regions, not one single-select
// tab strip. The other four (Syntax/Selection/Unit HEX/YUV Diff) are genuinely interchangeable
// "inspect the current selection" views and stay tabbed below it.
const STREAM_PANEL = {
  id: "stream",
  title: "Stream",
  component: StreamTreePanelWrapper,
  icon: "symbol-tree",
};

const LEFT_PANELS = [
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
  {
    id: "yuv-diff",
    title: "YUV Diff",
    component: YuvDiffPanelFromContext,
    icon: "diff",
  },
];

/**
 * Main App Content component
 * Manages file operations, keyboard navigation, and UI state
 */
function AppContent() {
  const { frames } = useFrameData();
  const { loading, error, filePath } = useFileState();
  const { currentFrameIndex, setCurrentFrameIndex } = useCurrentFrame();
  const { workspace: compareWorkspace } = useCompare();

  // GoToFrame dialog state
  const [showGoToFrame, setShowGoToFrame] = useState(false);

  // Layout context for save/load/reset
  const { saveLayout, loadLayout, resetLayout } = useLayout();

  // Recent files
  // recentFiles itself isn't consumed here -- nothing currently syncs it to a native "Recent
  // Files" menu or any UI; pre-existing gap, not something this typecheck-restoration pass
  // implements. addRecentFile is called below whenever a file opens successfully.
  const { addRecentFile } = useRecentFiles();

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

  // Pending YUV path — set when user picks a file, cleared after dialog confirm/cancel
  const [pendingYuvPath, setPendingYuvPath] = useState<string | null>(null);

  const {
    fileInfo,
    openError,
    handleOpenFile,
    openFileAtPath,
    handleCloseFile,
    handleOpenDependentFile,
  } = useAppFileOperations({
    onError: showErrorDialog,
    onCodecChange: setActiveCodec,
    onFileOpened: addRecentFile,
  });

  const exportEvidenceBundle = useExportEvidenceBundle();

  // Same condition `mainContent` below uses to decide whether to render the real UI vs. the
  // welcome screen -- see useOpenFileStatus's doc for why the main process needs to know this.
  useOpenFileStatus(Boolean(fileInfo?.success) && frames.length > 0);

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

  const onReloadFile = useCallback(() => {
    if (filePath) void openFileAtPath(filePath);
  }, [filePath, openFileAtPath]);

  const onToggleFullscreen = useCallback(async () => {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      const win = getCurrentWindow();
      const isFs = await win.isFullscreen();
      await win.setFullscreen(!isFs);
    } catch {
      // Fallback to browser fullscreen API
      if (!document.fullscreenElement) {
        void document.documentElement.requestFullscreen();
      } else {
        void document.exitFullscreen();
      }
    }
  }, []);

  const onEscape = useCallback(async () => {
    // Exit fullscreen if active
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      const win = getCurrentWindow();
      if (await win.isFullscreen()) {
        await win.setFullscreen(false);
        return;
      }
    } catch {
      if (document.fullscreenElement) {
        void document.exitFullscreen();
        return;
      }
    }
    // Clear selection
    window.dispatchEvent(new CustomEvent("clear-selection"));
  }, []);

  const onUndoSelection = useCallback(() => {
    window.dispatchEvent(new CustomEvent("undo-selection"));
  }, []);

  const onCopyBlockInfo = useCallback(() => {
    window.dispatchEvent(new CustomEvent("copy-block-info"));
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
    onReloadFile,
    onToggleFullscreen,
    onEscape,
    onUndoSelection,
    onCopyBlockInfo,
  });

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

  // Options menu events: CPU avx2, codec settings, auto-save layout
  useEffect(() => {
    // CPU avx2 toggle — persist preference to localStorage
    const handleCpuAvx2 = () => {
      const key = "bitvue:cpu-avx2";
      const next = !(localStorage.getItem(key) === "true");
      localStorage.setItem(key, String(next));
    };
    // Codec settings — forward as viewer events so backend/panel can react
    const fwd = (eventName: string) => () =>
      window.dispatchEvent(new CustomEvent(eventName));
    const handleHevcExt = fwd("codec-hevc-extensions");
    const handleHevcIndex = fwd("codec-hevc-index");
    const handleHevcMv = fwd("codec-hevc-mv");
    const handleVvcDynamic = fwd("codec-vvc-dynamic");
    const handleVvcDetails = fwd("codec-vvc-details");
    const handleDigestForce = fwd("codec-digest-force");
    const handleDigestNone = fwd("codec-digest-none");
    const handleDigestStream = fwd("codec-digest-stream");
    // Auto-save layout toggle
    const handleAutoSaveLayout = () => {
      const key = "bitvue:auto-save-layout";
      const next = !(localStorage.getItem(key) === "true");
      localStorage.setItem(key, String(next));
    };

    window.addEventListener("menu-cpu-avx2", handleCpuAvx2);
    window.addEventListener("menu-codec-hevc-ext", handleHevcExt);
    window.addEventListener("menu-codec-hevc-index", handleHevcIndex);
    window.addEventListener("menu-codec-hevc-mv", handleHevcMv);
    window.addEventListener("menu-codec-vvc-dynamic", handleVvcDynamic);
    window.addEventListener("menu-codec-vvc-details", handleVvcDetails);
    window.addEventListener("menu-digest-force", handleDigestForce);
    window.addEventListener("menu-digest-none", handleDigestNone);
    window.addEventListener("menu-digest-stream", handleDigestStream);
    window.addEventListener("menu-auto-save-layout", handleAutoSaveLayout);
    return () => {
      window.removeEventListener("menu-cpu-avx2", handleCpuAvx2);
      window.removeEventListener("menu-codec-hevc-ext", handleHevcExt);
      window.removeEventListener("menu-codec-hevc-index", handleHevcIndex);
      window.removeEventListener("menu-codec-hevc-mv", handleHevcMv);
      window.removeEventListener("menu-codec-vvc-dynamic", handleVvcDynamic);
      window.removeEventListener("menu-codec-vvc-details", handleVvcDetails);
      window.removeEventListener("menu-digest-force", handleDigestForce);
      window.removeEventListener("menu-digest-none", handleDigestNone);
      window.removeEventListener("menu-digest-stream", handleDigestStream);
      window.removeEventListener("menu-auto-save-layout", handleAutoSaveLayout);
    };
  }, []);

  // Overlay toggle / clear events (from View → Info Overlays submenu + Ctrl+F1–F6)
  useEffect(() => {
    const handleToggleOverlay = (e: Event) => {
      const mode = (e as CustomEvent<string>).detail;
      if (mode)
        toggleOverlay(
          mode as import("./utils/codecModeRegistry").VisualizationMode,
        );
    };
    const handleClearOverlays = () => clearOverlays();
    // Ctrl+F1–F6: map to the first 6 available overlay modes
    const handleOverlayFKey = (e: Event) => {
      const fKey = (e as CustomEvent<number>).detail;
      const overlayKeys = [
        "mv-field",
        "qp-heatmap",
        "partition",
        "cbf-luma",
        "transform",
        "pred-mode",
      ] as const;
      const mode = overlayKeys[fKey - 1];
      if (mode)
        toggleOverlay(
          mode as import("./utils/codecModeRegistry").VisualizationMode,
        );
    };
    window.addEventListener("menu-toggle-overlay", handleToggleOverlay);
    window.addEventListener("menu-clear-overlays", handleClearOverlays);
    window.addEventListener("viewer-toggle-overlay-fkey", handleOverlayFKey);
    return () => {
      window.removeEventListener("menu-toggle-overlay", handleToggleOverlay);
      window.removeEventListener("menu-clear-overlays", handleClearOverlays);
      window.removeEventListener(
        "viewer-toggle-overlay-fkey",
        handleOverlayFKey,
      );
    };
  }, [toggleOverlay, clearOverlays]);

  // Recent files menu: open the selected path
  useEffect(() => {
    const handleOpenRecent = (e: Event) => {
      const path = (e as CustomEvent<string>).detail;
      if (path) void openFileAtPath(path);
    };
    window.addEventListener("menu-open-recent-file", handleOpenRecent);
    return () => {
      window.removeEventListener("menu-open-recent-file", handleOpenRecent);
    };
  }, [openFileAtPath]);

  // File menu events
  useEffect(() => {
    const handleExportListener = () => setShowExportDialog(true);
    const handleOpenBitstream = () => {
      void handleOpenFile();
    };
    const handleOpenDebugYuv = async () => {
      const selected = await showOpenDialog([
        { name: "Raw YUV", extensions: ["yuv", "raw", "y4m"] },
      ]);
      if (!selected) return;
      // Show the load dialog for the user to confirm resolution / format / bitdepth
      setPendingYuvPath(selected);
    };
    const handleCloseBitstream = () => {
      void handleCloseFile();
    };
    const handleOpenDependentBitstream = () => {
      void handleOpenDependentFile();
    };
    const handleExportEvidence = () => {
      void exportEvidenceBundle();
    };
    const handleQuit = () => {
      void closeWindow();
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
    window.addEventListener("menu-export-evidence", handleExportEvidence);
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
      window.removeEventListener("menu-export-evidence", handleExportEvidence);
      window.removeEventListener(
        "menu-open-debug-yuv",
        handleOpenDebugYuvEvent,
      );
    };
  }, [
    exportEvidenceBundle,
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
        pinnedLeftPanel={STREAM_PANEL}
        leftPanels={leftPanels}
        mainView={mainView}
        topPanels={topPanels}
        bottomRowPanels={bottomRowPanels}
      />
    ) : null;

  return (
    <SelectionProvider>
      <FrameSyncBridge />
      <ErrorBoundary>
        <div className="app">
          {/* Custom TitleBar for Windows/Linux only */}
          {shouldShowTitleBar() && (
            <TitleBar
              fileName={fileInfo?.path || "Bitvue"}
              onOpenFile={handleOpenFile}
              onOpenDependentFile={handleOpenDependentFile}
              onCloseFile={handleCloseFile}
              onQuit={() => closeWindow()}
              onShowShortcuts={() => setShowShortcuts(true)}
              onModeChange={setMode}
            />
          )}

          <div className="app-container">
            {compareWorkspace ? (
              <CompareWorkspaceFromContext />
            ) : fileInfo?.success && frames.length > 0 ? (
              mainContent
            ) : fileInfo?.success && frames.length === 0 ? (
              noFramesError
            ) : (
              welcomeScreen
            )}
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

      {/* Load Debug YUV Dialog */}
      {pendingYuvPath && (
        <Suspense fallback={null}>
          <LoadDebugYuvDialog
            filePath={pendingYuvPath}
            onConfirm={(params) => {
              setPendingYuvPath(null);
              void loadDebugYuv({ path: pendingYuvPath, ...params });
            }}
            onCancel={() => setPendingYuvPath(null)}
          />
        </Suspense>
      )}
    </SelectionProvider>
  );
}

export default memo(App);
