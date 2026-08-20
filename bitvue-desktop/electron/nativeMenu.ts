/**
 * Native macOS menu bar (`Menu.setApplicationMenu`, darwin only). Extracted from `main.ts`
 * (2026-08-20 axis-1 cleanup -- this was a 217-line function bundled into the same file as
 * everything else) since it's the one single-responsibility chunk of `main.ts` that was still
 * large enough to be worth its own file.
 *
 * Replaces `frontend/utils/menu/setup.ts`'s dead `initializeSystemMenu()`, which called into
 * `@tauri-apps/api/menu` -- a runtime that no longer exists post-Electron-migration (`transformCallback`
 * undefined, throws on every launch). Windows/Linux already have a *working* menu via
 * `frontend/components/TitleBar.tsx` (a real in-window menu bar, dispatching the same
 * `window` CustomEvents as this file does below) -- `shouldShowTitleBar()` gates it to those two
 * platforms only, which is why this is darwin-only too: setting a second, real Electron
 * application menu on Windows/Linux would show a duplicate menu bar alongside TitleBar's.
 *
 * Before this fix, macOS had *no* working menu at all -- not File, not Export, not Mode
 * switching, nothing menu-only-reachable (including this session's own Evidence Bundle export).
 * The WelcomeScreen's "Open" button and keyboard shortcuts (a separate `keydown` listener,
 * `frontend/hooks/useKeyboardNavigation.ts`) still worked, which is why the app wasn't *entirely*
 * unusable, just silently missing every menu action on the platform this was tested on.
 *
 * Deliberately omits `accelerator` on every item: `useKeyboardNavigation.ts` already owns
 * keyboard shortcuts (F1-F7, etc.) via its own `keydown` listener on the renderer side. Adding
 * the same accelerators here risks double-firing or fighting over which layer wins; this menu's
 * job is fixing "clicking does nothing", not shortcuts (which already work).
 *
 * Structure and event names are a direct translation of `frontend/utils/menu/creators/*.ts`
 * (the same config those Tauri-based `Submenu`/`MenuItem` builders decorate) -- kept in sync by
 * hand since Electron's `MenuItemConstructorOptions` and Tauri's `MenuItemOptions` aren't the
 * same shape and can't share a builder.
 */

import {
  BrowserWindow,
  Menu,
  type MenuItemConstructorOptions,
} from "electron";

export function installNativeMacMenu(win: BrowserWindow): void {
  if (process.platform !== "darwin") return;

  const dispatch = (event: string, detail?: unknown) => {
    const target = BrowserWindow.getFocusedWindow() ?? win;
    const script =
      detail === undefined
        ? `window.dispatchEvent(new CustomEvent(${JSON.stringify(event)}))`
        : `window.dispatchEvent(new CustomEvent(${JSON.stringify(event)}, { detail: ${JSON.stringify(detail)} }))`;
    void target.webContents.executeJavaScript(script);
  };

  const item = (
    label: string,
    event: string,
    detail?: unknown,
  ): MenuItemConstructorOptions => ({
    label,
    click: () => dispatch(event, detail),
  });
  const sep: MenuItemConstructorOptions = { type: "separator" };

  const template: MenuItemConstructorOptions[] = [
    {
      label: "About",
      submenu: [item("About Bitvue", "menu-about")],
    },
    {
      label: "File",
      submenu: [
        item("Open bitstream...", "menu-open-bitstream"),
        {
          label: "Open bitstream as...",
          submenu: [
            item("AV1", "menu-open-as-av1"),
            item("HEVC", "menu-open-as-hevc"),
            item("AVC/H.264", "menu-open-as-avc"),
            item("VP9", "menu-open-as-vp9"),
            item("VVC/H.266", "menu-open-as-vvc"),
            item("MPEG-2", "menu-open-as-mpeg2"),
          ],
        },
        item("Open dependent bitstream...", "menu-open-dependent"),
        sep,
        item("Close bitstream", "menu-close-bitstream"),
        sep,
        {
          label: "Extract...",
          submenu: [
            item("YUV frames", "menu-extract-yuv"),
            item("Prediction frames", "menu-extract-prediction"),
            item("Reconstruction frames", "menu-extract-reconstruction"),
            item("Transform coefficients", "menu-extract-transform"),
          ],
        },
        sep,
        item("Recent Files", "menu-recent-files"),
        sep,
        item("Quit", "menu-quit"),
      ],
    },
    {
      label: "Mode",
      submenu: [
        item("Overview", "menu-mode-change", "overview"),
        item("Coding Flow", "menu-mode-change", "coding"),
        item("Prediction", "menu-mode-change", "prediction"),
        item("Transform", "menu-mode-change", "transform"),
        item("QP Map", "menu-mode-change", "qp"),
        item("MV Field", "menu-mode-change", "mv"),
        item("Reference Frames", "menu-mode-change", "reference"),
        sep,
        item("Extended Modes", "menu-mode-extended"),
      ],
    },
    {
      label: "YUVDiff",
      submenu: [
        item("Open debug YUV...", "menu-open-debug-yuv"),
        item("Recent YUV files", "menu-recent-yuv"),
        item("Close debug YUV", "menu-close-debug-yuv"),
        sep,
        {
          label: "Subsampling",
          submenu: [
            item("Planar (YUV420)", "menu-subsampling-planar"),
            item("Interleaved (YUV422)", "menu-subsampling-interleaved"),
          ],
        },
        item("Display order", "menu-display-order"),
        item("Decode order", "menu-decode-order"),
        sep,
        item("Use stream crop values", "menu-stream-crop"),
        item("Set picture offset here", "menu-picture-offset"),
        sep,
        item("Use stream bitdepth", "menu-bitdepth-stream"),
        item("Use max stream bitdepth", "menu-bitdepth-max"),
        item("Use 16 bit bitdepth", "menu-bitdepth-16"),
        sep,
        item("Check for file changes", "menu-check-file-changes"),
        sep,
        item("Show PSNR Map", "menu-show-psnr"),
        item("Show SSIM Map", "menu-show-ssim"),
        item("Show Delta Image", "menu-show-delta"),
        sep,
        item("Export All Frames", "menu-export-all-frames"),
        item("Export Metrics CSV...", "menu-export-metrics"),
      ],
    },
    {
      label: "Options",
      submenu: [
        {
          label: "Color Space",
          submenu: [
            item("ITU Rec. 601", "menu-color-bt601"),
            item("ITU Rec. 709", "menu-color-bt709"),
            item("ITU Rec. 2020", "menu-color-bt2020"),
            sep,
            item("YUV as RGB", "menu-color-yuv-rgb"),
            item("YUV as GBR", "menu-color-yuv-gbr"),
          ],
        },
        {
          label: "CPU & Performance",
          submenu: [
            item("Enable CPU optimizations [avx2]", "menu-cpu-avx2"),
            sep,
            item("Loop playback", "menu-loop-playback"),
          ],
        },
        {
          label: "Codec Settings",
          submenu: [
            item("HEVC: Enable extensions", "menu-codec-hevc-ext"),
            item("HEVC: Enable stream index", "menu-codec-hevc-index"),
            item("HEVC: Show only visible CTB MV", "menu-codec-hevc-mv"),
            sep,
            item("VVC: Dynamic selection info", "menu-codec-vvc-dynamic"),
            item("VVC: Details popup window", "menu-codec-vvc-details"),
            sep,
            item("Digest: Force digest", "menu-digest-force"),
            item("Digest: No digest", "menu-digest-none"),
            item("Digest: As in bitstream", "menu-digest-stream"),
          ],
        },
        sep,
        item("Dark Theme", "menu-theme-change", "dark"),
        item("Light Theme", "menu-theme-change", "light"),
        sep,
        item("Save Layout...", "menu-save-layout"),
        item("Load Layout...", "menu-load-layout"),
        item("Reset Layout", "menu-reset-layout"),
        item("Auto-save on exit", "menu-auto-save-layout"),
      ],
    },
    {
      label: "Export",
      submenu: [
        item("Export...", "menu-export"),
        sep,
        {
          label: "Data Export",
          submenu: [
            item("Frame Sizes (CSV)", "menu-export-frame-sizes"),
            item("Unit Tree (JSON)", "menu-export-unit-tree"),
            item("Syntax Tree (JSON)", "menu-export-syntax-tree"),
          ],
        },
        sep,
        item("Evidence Bundle...", "menu-export-evidence"),
      ],
    },
    {
      label: "View",
      submenu: [
        item("Reset Layout", "menu-reset-layout"),
        sep,
        item("Stream Tree", "menu-toggle-stream-tree"),
        item("Player", "menu-toggle-player"),
        item("Diagnostics", "menu-toggle-diagnostics"),
        sep,
        {
          label: "Info Overlays",
          submenu: [
            item("QP Map", "menu-toggle-overlay", "qp-map"),
            item("Heat Map", "menu-toggle-overlay", "heat-map"),
            item("MV Heat", "menu-toggle-overlay", "mv-field"),
            item("PU Type", "menu-toggle-overlay", "pu-type"),
            item("MB Type", "menu-toggle-overlay", "mb-type"),
            item("Ref Indices", "menu-toggle-overlay", "reference-indices"),
            item("Block Type", "menu-toggle-overlay", "block-type"),
            item("Efficiency Map", "menu-toggle-overlay", "efficiency-map"),
            item("Inter Memory", "menu-toggle-overlay", "inter-memory"),
            item("Simple Motion", "menu-toggle-overlay", "simple-motion"),
            sep,
            item("PSNR Overlay", "menu-toggle-overlay", "psnr-overlay"),
            item("SSIM Overlay", "menu-toggle-overlay", "ssim-overlay"),
            sep,
            item("Clear All Overlays", "menu-clear-overlays"),
          ],
        },
      ],
    },
    {
      label: "Help",
      submenu: [
        item("Documentation", "menu-documentation"),
        item("Keyboard Shortcuts", "menu-keyboard-shortcuts"),
        sep,
        item("About Bitvue", "menu-about"),
      ],
    },
  ];

  Menu.setApplicationMenu(Menu.buildFromTemplate(template));
}
