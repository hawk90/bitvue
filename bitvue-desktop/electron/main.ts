/**
 * Electron main process entry point. Spawns the `bitvue-sidecar` Rust binary via
 * `SidecarClient` and exposes a small, named set of IPC channels to the renderer (via
 * `electron/preload.cjs`'s `contextBridge`) — deliberately not a generic "invoke any sidecar
 * method" passthrough, matching the "typed, query-shaped IPC" principle from
 * `docs/DEVELOPMENT_PHASES.md` § "제품 아키텍처 확정".
 *
 * This is the last previously-unproven link in the Tauri→Electron bridge: `bitvue-protocol`
 * (wire schema) → `bitvue-sidecar` (Rust process) → `SidecarClient` (Node client, `../src/`)
 * were all verified independently; this file is what actually turns that into a running
 * Electron app that a renderer can talk to.
 *
 * Known simplifications (prototype proving the architecture, not production-hardened):
 *  - `sandbox: false` on the preload's `webPreferences` — sidesteps Electron's
 *    sandboxed-preload ESM/CommonJS restrictions for now. Revisit before shipping.
 *  - No input validation on the IPC handler params beyond what the sidecar itself rejects.
 *  - Single global `SidecarClient` instance, no multi-window/multi-stream-session story yet.
 */

import {
  app,
  BrowserWindow,
  dialog,
  ipcMain,
  Menu,
  session,
  type MenuItemConstructorOptions,
} from "electron";
import { existsSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { SidecarClient } from "../src/sidecarClient.js";
import { isQuitConfirmed, requestQuit, setMainWindow } from "./quitGuard.js";

const here = path.dirname(fileURLToPath(import.meta.url));
// dist/electron/main.js -> dist/electron -> dist -> bitvue-desktop -> repo root
const repoRoot = path.resolve(here, "..", "..", "..");
const defaultBinaryName =
  process.platform === "win32" ? "bitvue-sidecar.exe" : "bitvue-sidecar";
/**
 * Packaged builds (`app.isPackaged`) don't have a `target/`/`frontend/` checkout next to the
 * app — electron-builder's `extraResources` (see `bitvue-desktop/package.json`'s `build` field)
 * copies the release sidecar binary and built frontend into `process.resourcesPath` instead
 * (`Contents/Resources` on macOS, `resources/` next to the exe on Windows/Linux). Dev mode keeps
 * resolving against the repo checkout so `npm run electron` works without packaging first.
 */
const sidecarBinaryPath =
  process.env.BITVUE_SIDECAR_BIN ??
  (app.isPackaged
    ? path.join(process.resourcesPath, "bin", defaultBinaryName)
    : path.join(repoRoot, "target", "debug", defaultBinaryName));

let sidecar: SidecarClient | undefined;
let shuttingDown = false;

/**
 * Error/warning-level renderer console messages, collected by `createWindow`'s `console-message`
 * forwarder below. `runSelfTestAndExit` fails the run if this is non-empty at the end -- found
 * necessary the hard way: macOS's menu system was completely broken (two separate Tauri-leftover
 * calls throwing an unhandled rejection on *every* launch) for the entire session without
 * BITVUE_ELECTRON_SELFTEST ever catching it, because the selftest checked IPC *data* correctness
 * but never looked at whether the renderer logged any errors along the way. A real bug report
 * ("영상 디코딩도 제대로 안되는구만") is what surfaced it, not this test suite.
 */
const consoleErrors: string[] = [];

/**
 * Content-Security-Policy (SEC-011): no CSP existed anywhere post-Electron-migration -- Tauri's
 * built-in CSP was dropped and never replaced, so Electron prints its own "Electron Security
 * Warning (Insecure Content-Security-Policy)" on every launch. Applied via
 * `session.defaultSession.webRequest.onHeadersReceived` (main()'s registration, below) rather
 * than a `<meta http-equiv="Content-Security-Policy">` tag, because it's the one place that
 * covers every way the renderer's document actually gets loaded: packaged `file://`
 * (win.loadFile against resources/frontend/index.html), the repo-checkout dev flow (also
 * `file://`, same code path, just frontend/dist/index.html -- see `resolveRendererTarget`), and
 * the `BITVUE_FRONTEND_URL` override (a live Vite dev server, win.loadURL). A <meta> tag would
 * have to be duplicated across this package's own placeholder index.html *and* frontend's built
 * dist/index.html (regenerated on every `vite build`, so anything checked into
 * frontend/index.html wouldn't survive into the shipped artifact anyway), and couldn't apply at
 * all to the BITVUE_FRONTEND_URL case since that document's <head> comes from Vite's dev server,
 * not this repo.
 *
 * The renderer never does fetch/XHR to a remote origin -- all real data comes over Electron IPC
 * via `window.bitvue` (see electronBridgeService.ts's module doc) -- and never loads remote
 * images or fonts (codicon.ttf is bundled under frontend/public/fonts; thumbnails/screenshots
 * are `data:` URLs, see e.g. useFilmstripState.ts and captureScreenshot's doc below). So this is
 * a real, restrictive policy, not a rubber-stamped `default-src *`.
 *
 * `style-src` needs 'unsafe-inline': React's `style={{...}}` prop compiles to a DOM `style`
 * attribute, which CSP's style-src covers (not just <style> tags/<link rel=stylesheet>) -- this
 * codebase uses inline styles pervasively (e.g. App.tsx's noFramesError block), so disallowing
 * it would break large parts of the UI, not just an edge case. There's no user-controlled HTML
 * ever injected this way (no dangerouslySetInnerHTML in the tree, verified by grep), so the XSS
 * risk 'unsafe-inline' style normally carries doesn't apply here the way it would for
 * script-src, which stays locked down.
 *
 * `script-src`/`connect-src` are relaxed (`'unsafe-eval'`, `ws:`/`wss:`) only when
 * `BITVUE_FRONTEND_URL` is set -- Vite's dev-server client/HMR runtime needs both (module
 * transform eval, and a WebSocket back to the dev server for live reload). The packaged and
 * default repo-checkout dev flows (both `file://`) never hit this branch, so production and the
 * normal `scripts/dev.sh` flow both get the strict policy.
 */
function contentSecurityPolicy(): string {
  const devServerUrl = process.env.BITVUE_FRONTEND_URL;
  const scriptSrc = devServerUrl ? "'self' 'unsafe-eval'" : "'self'";
  const connectSrc = devServerUrl ? "'self' ws: wss:" : "'self'";
  return [
    "default-src 'self'",
    `script-src ${scriptSrc}`,
    "style-src 'self' 'unsafe-inline'",
    "img-src 'self' data:",
    "font-src 'self' data:",
    `connect-src ${connectSrc}`,
    "object-src 'none'",
    "base-uri 'none'",
    "form-action 'none'",
    "frame-src 'none'",
  ].join("; ");
}

function requireSidecar(): SidecarClient {
  if (!sidecar)
    throw new Error(
      "sidecar not started yet — this shouldn't happen post-app.whenReady()",
    );
  return sidecar;
}

function registerIpcHandlers(): void {
  ipcMain.handle("bitvue:hello", async (_event, clientVersion?: string) => {
    return requireSidecar().hello(clientVersion);
  });

  ipcMain.handle(
    "bitvue:openStream",
    async (_event, stream: string, filePath: string) => {
      return requireSidecar().request("open_stream", {
        stream,
        path: filePath,
      });
    },
  );

  ipcMain.handle(
    "bitvue:getHexRange",
    async (_event, stream: string, offset: number, len: number) => {
      const { bytes, ...metadata } = await requireSidecar().getHexRange({
        stream,
        offset,
        len,
      });
      // Buffer survives Electron's structured-clone IPC as-is (renderer sees a Uint8Array) —
      // no base64/JSON-array encoding. This is the whole point of the migration; see the
      // `YUVFrameData` anti-pattern note in DEVELOPMENT_PHASES.md.
      return { ...metadata, bytes };
    },
  );

  ipcMain.handle(
    "bitvue:getDecodedFrameYuv",
    async (_event, stream: string, frameIndex: number) => {
      const { bytes, ...metadata } = await requireSidecar().getDecodedFrameYuv({
        stream,
        frameIndex,
      });
      // Same raw-Buffer-over-structured-clone approach as getHexRange above -- no base64. The
      // renderer slices `bytes` into Y/U/V planes itself using `yLen`/`uLen`/`vLen`.
      return { ...metadata, bytes };
    },
  );

  ipcMain.handle(
    "bitvue:getDebugYuvFrame",
    async (_event, frameIndex: number, mode: string, amplify?: number) => {
      const { bytes, ...metadata } = await requireSidecar().getDebugYuvFrame({
        frameIndex,
        mode: mode as "decoded" | "reference" | "diff" | "amplified",
        amplify,
      });
      // Same raw-Buffer-over-structured-clone approach as getDecodedFrameYuv above.
      return { ...metadata, bytes };
    },
  );

  ipcMain.handle(
    "bitvue:loadDebugYuv",
    async (
      _event,
      params: {
        path: string;
        width: number;
        height: number;
        format: string;
        bitdepth: number;
        picture_offset?: number;
        crop?: { left: number; right: number; top: number; bottom: number };
      },
    ) => {
      return requireSidecar().request("load_debug_yuv", params);
    },
  );

  ipcMain.handle("bitvue:unloadDebugYuv", async () => {
    return requireSidecar().request("unload_debug_yuv");
  });

  ipcMain.handle("bitvue:setDebugYuvOffset", async (_event, offset: number) => {
    return requireSidecar().request("set_debug_yuv_offset", { offset });
  });

  ipcMain.handle(
    "bitvue:setDebugYuvCrop",
    async (
      _event,
      crop: { left: number; right: number; top: number; bottom: number },
    ) => {
      return requireSidecar().request("set_debug_yuv_crop", { crop });
    },
  );

  ipcMain.handle(
    "bitvue:getYuvDiffMetrics",
    async (_event, frameIndex: number) => {
      return requireSidecar().request("get_yuv_diff_metrics", {
        frame_index: frameIndex,
      });
    },
  );

  ipcMain.handle("bitvue:findFirstDiffFrame", async () => {
    return requireSidecar().request("find_first_diff_frame");
  });

  // Dual-stream compare workspace (docs/DEVELOPMENT_PHASES.md Phase 7.5) -- PTS-based A/B
  // alignment + real diff/heatmap overlay. All plain JSON control-frame commands (no raw-Buffer
  // payload), unlike getDecodedFrameYuv/getHexRange above.
  ipcMain.handle("bitvue:createCompareWorkspace", async () => {
    return requireSidecar().request("create_compare_workspace");
  });

  ipcMain.handle(
    "bitvue:getAlignedFrame",
    async (_event, streamAFrameIdx: number) => {
      return requireSidecar().request("get_aligned_frame", {
        stream_a_frame_idx: streamAFrameIdx,
      });
    },
  );

  ipcMain.handle(
    "bitvue:setSyncMode",
    async (_event, mode: "Off" | "Playhead" | "Full") => {
      return requireSidecar().request("set_sync_mode", { mode });
    },
  );

  ipcMain.handle(
    "bitvue:setManualOffset",
    async (_event, offset: number) => {
      return requireSidecar().request("set_manual_offset", { offset });
    },
  );

  ipcMain.handle("bitvue:resetOffset", async () => {
    return requireSidecar().request("reset_offset");
  });

  ipcMain.handle(
    "bitvue:getDiffFrame",
    async (_event, streamAFrameIdx: number, mode: "abs" | "signed") => {
      return requireSidecar().request("get_diff_frame", {
        stream_a_frame_idx: streamAFrameIdx,
        mode,
      });
    },
  );

  // PARITY_CHECKLIST.md CMP-04 -- distinct wire method name from bitvue:findFirstDiffFrame
  // above (that one is the debug-YUV single-stream-vs-reference-file command).
  ipcMain.handle("bitvue:findFirstDiffFrameAb", async () => {
    return requireSidecar().request("find_first_diff_frame_ab");
  });

  ipcMain.handle(
    "bitvue:getFrameAnalysis",
    async (_event, frameIndex: number) => {
      return requireSidecar().request("get_frame_analysis", {
        frame_index: frameIndex,
      });
    },
  );

  ipcMain.handle(
    "bitvue:getAv1Features",
    async (_event, frameIndex: number) => {
      return requireSidecar().request("get_av1_features", {
        frame_index: frameIndex,
      });
    },
  );

  ipcMain.handle(
    "bitvue:getCodingFlowAnalysis",
    async (_event, frameIndex: number) => {
      return requireSidecar().request("get_coding_flow_analysis", {
        frame_index: frameIndex,
      });
    },
  );

  ipcMain.handle(
    "bitvue:getDeblockingAnalysis",
    async (_event, frameIndex: number) => {
      return requireSidecar().request("get_deblocking_analysis", {
        frame_index: frameIndex,
      });
    },
  );

  ipcMain.handle(
    "bitvue:getCodecExtendedInfo",
    async (_event, frameIndex: number) => {
      return requireSidecar().request("get_codec_extended_info", {
        frame_index: frameIndex,
      });
    },
  );

  ipcMain.handle(
    "bitvue:getResidualAnalysis",
    async (_event, frameIndex: number) => {
      return requireSidecar().request("get_residual_analysis", {
        frame_index: frameIndex,
      });
    },
  );

  ipcMain.handle(
    "bitvue:getContextMenuItems",
    async (
      _event,
      scope: string,
      hasSelection: boolean,
      hasByteRange: boolean,
    ) => {
      return requireSidecar().request("get_context_menu_items", {
        scope,
        has_selection: hasSelection,
        has_byte_range: hasByteRange,
      });
    },
  );

  ipcMain.handle(
    "bitvue:exportEvidenceBundle",
    async (
      _event,
      params: {
        outputDir: string;
        workspace?: string;
        mode?: string;
        orderType?: string;
        screenshotDataUrl?: string;
      },
    ) => {
      return requireSidecar().request("export_evidence_bundle", {
        output_dir: params.outputDir,
        workspace: params.workspace,
        mode: params.mode,
        order_type: params.orderType,
        screenshot_data_url: params.screenshotDataUrl,
      });
    },
  );

  ipcMain.handle("bitvue:closeStream", async (_event, stream: string) => {
    return requireSidecar().request("close_stream", { stream });
  });

  ipcMain.handle(
    "bitvue:selectFrame",
    async (_event, stream: string, frameIndex: number) => {
      return requireSidecar().request("select_frame", {
        stream,
        frame_index: frameIndex,
      });
    },
  );

  ipcMain.handle("bitvue:indexStream", async (_event, stream: string) => {
    return requireSidecar().request("index_stream", { stream });
  });

  ipcMain.handle("bitvue:getStreamInfo", async (_event, stream: string) => {
    return requireSidecar().request("get_stream_info", { stream });
  });

  ipcMain.handle(
    "bitvue:getFramesChunk",
    async (_event, stream: string, offset: number, limit: number) => {
      return requireSidecar().request("get_frames_chunk", {
        stream,
        offset,
        limit,
      });
    },
  );

  ipcMain.handle(
    "bitvue:getFrameSyntax",
    async (_event, stream: string, frameIndex: number) => {
      return requireSidecar().request("get_frame_syntax", {
        stream,
        frame_index: frameIndex,
      });
    },
  );

  ipcMain.handle("bitvue:getTimeline", async (_event, stream: string) => {
    return requireSidecar().request("get_timeline", { stream });
  });

  ipcMain.handle(
    "bitvue:getThumbnails",
    async (
      _event,
      stream: string,
      frameIndices: number[],
      targetWidth?: number,
    ) => {
      return requireSidecar().request("get_thumbnails", {
        stream,
        frame_indices: frameIndices,
        target_width: targetWidth,
      });
    },
  );

  ipcMain.handle(
    "bitvue:selectUnit",
    async (
      _event,
      stream: string,
      unitType: string,
      offset: number,
      size: number,
    ) => {
      return requireSidecar().request("select_unit", {
        stream,
        unit_type: unitType,
        offset,
        size,
      });
    },
  );

  ipcMain.handle(
    "bitvue:selectSyntax",
    async (
      _event,
      stream: string,
      nodeId: string,
      startBit: number,
      endBit: number,
    ) => {
      return requireSidecar().request("select_syntax", {
        stream,
        node_id: nodeId,
        start_bit: startBit,
        end_bit: endBit,
      });
    },
  );

  ipcMain.handle(
    "bitvue:selectBitRange",
    async (_event, stream: string, startBit: number, endBit: number) => {
      return requireSidecar().request("select_bit_range", {
        stream,
        start_bit: startBit,
        end_bit: endBit,
      });
    },
  );

  ipcMain.handle(
    "bitvue:selectSpatialBlock",
    async (
      _event,
      stream: string,
      x: number,
      y: number,
      w: number,
      h: number,
    ) => {
      return requireSidecar().request("select_spatial_block", {
        stream,
        x,
        y,
        w,
        h,
      });
    },
  );

  ipcMain.handle(
    "bitvue:showOpenDialog",
    async (event, filters?: Array<{ name: string; extensions: string[] }>) => {
      // Test-only bypass: a real native OS dialog can't be scripted in an automated/offscreen
      // run. Only takes effect when this env var is explicitly set (never in normal usage) --
      // lets BITVUE_ELECTRON_SCREENSHOT drive the actual production handleOpenFile() code path
      // (real dialog call site, just answered without a human) instead of calling bridge
      // functions directly the way BITVUE_ELECTRON_SELFTEST does.
      if (process.env.BITVUE_ELECTRON_SELFTEST_FIXTURE_PATH) {
        return process.env.BITVUE_ELECTRON_SELFTEST_FIXTURE_PATH;
      }
      const win = BrowserWindow.fromWebContents(event.sender);
      const options: Electron.OpenDialogOptions = {
        properties: ["openFile"],
        filters: filters ?? [{ name: "All Files", extensions: ["*"] }],
      };
      const result = win
        ? await dialog.showOpenDialog(win, options)
        : await dialog.showOpenDialog(options);
      if (result.canceled || result.filePaths.length === 0) return null;
      return result.filePaths[0];
    },
  );

  ipcMain.handle("bitvue:showDirectoryDialog", async (event) => {
    // Same SELFTEST_FIXTURE_PATH test bypass reasoning as showOpenDialog above -- reused here as
    // an arbitrary writable directory (a temp dir would need its own env var; the fixture path's
    // parent directory is always writable in the test environment this runs in).
    if (process.env.BITVUE_ELECTRON_SELFTEST_FIXTURE_PATH) {
      return path.dirname(process.env.BITVUE_ELECTRON_SELFTEST_FIXTURE_PATH);
    }
    const win = BrowserWindow.fromWebContents(event.sender);
    const options: Electron.OpenDialogOptions = {
      properties: ["openDirectory", "createDirectory"],
    };
    const result = win
      ? await dialog.showOpenDialog(win, options)
      : await dialog.showOpenDialog(options);
    if (result.canceled || result.filePaths.length === 0) return null;
    return result.filePaths[0];
  });

  // Captures the current window as a PNG, base64-encoded as a data: URL -- the same
  // `capturePage()`/`toPNG()` pair `runScreenshotAndExit` already uses for the
  // BITVUE_ELECTRON_SCREENSHOT dev/CI mode, exposed here as an on-demand handler for real
  // in-app features (currently: Evidence Bundle export's `screenshots/` artifact). Returns null
  // if there's no window to capture from (shouldn't happen in practice -- the renderer calling
  // this is itself running inside a window).
  ipcMain.handle("bitvue:captureScreenshot", async (event) => {
    const win = BrowserWindow.fromWebContents(event.sender);
    if (!win) return null;
    const image = await win.webContents.capturePage();
    return `data:image/png;base64,${image.toPNG().toString("base64")}`;
  });

  // Quit menu item / TitleBar's Quit button / TitleBar's in-window close (X) button. Routed
  // through requestQuit() (TAURI_WEB-006) instead of calling app.quit() directly -- see its doc
  // for why (confirms first when a real bitstream is open) and for how this stays "quit the app"
  // cross-platform, not just "close the current window", same as before this fix.
  ipcMain.handle("bitvue:closeWindow", async () => {
    await requestQuit();
  });
}

// Packaged builds ship the built frontend under resources/frontend/ (extraResources, see
// sidecarBinaryPath's doc above); dev mode resolves against the repo checkout.
const frontendDistIndex = app.isPackaged
  ? path.join(process.resourcesPath, "frontend", "index.html")
  : path.join(repoRoot, "frontend", "dist", "index.html");

/**
 * Where to load the renderer content from, in priority order:
 * 1. `BITVUE_FRONTEND_URL` (explicit override — e.g. the Vite dev server, `http://localhost:5173`,
 *    for `npm run dev` workflows where `frontend/` is served separately, not built).
 * 2. `frontend/dist/index.html` (dev) or the bundled `resources/frontend/index.html` (packaged),
 *    if it exists — this is the *real* Bitvue Analyzer UI.
 * 3. This package's own placeholder `index.html` — only reached if neither of the above is
 *    available, e.g. a bare `bitvue-desktop` checkout with `frontend/` never built. Logs a
 *    warning so it's not mistaken for the real app.
 */
function resolveRendererTarget(): { kind: "url" | "file"; target: string } {
  const overrideUrl = process.env.BITVUE_FRONTEND_URL;
  if (overrideUrl) return { kind: "url", target: overrideUrl };
  if (existsSync(frontendDistIndex))
    return { kind: "file", target: frontendDistIndex };
  console.warn(
    `[bitvue-desktop] frontend dist not found (${frontendDistIndex}) and BITVUE_FRONTEND_URL not set — ` +
      "loading the placeholder shell instead of the real Bitvue Analyzer UI. " +
      "Run `cd frontend && npm run build`, or set BITVUE_FRONTEND_URL to a dev server.",
  );
  return { kind: "file", target: path.join(here, "index.html") };
}

/**
 * Native macOS menu bar (`Menu.setApplicationMenu`, darwin only).
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
function installNativeMacMenu(win: BrowserWindow): void {
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

function createWindow(): BrowserWindow {
  const win = new BrowserWindow({
    width: 1280,
    height: 800,
    show: process.env.BITVUE_ELECTRON_OFFSCREEN !== "1",
    webPreferences: {
      preload: path.join(here, "preload.cjs"),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: false, // see module doc — simplification, revisit before shipping
      offscreen: process.env.BITVUE_ELECTRON_OFFSCREEN === "1",
    },
  });
  // Renderer console/load errors don't surface in this process's stdout by default -- a blank
  // page (e.g. a JS asset 404 under file://, an uncaught exception during React mount) is
  // otherwise silent here. Forward them so `npm run electron`'s terminal output is actually
  // useful for debugging instead of just showing an unexplained blank window.
  win.webContents.on(
    "console-message",
    (_event, level, message, line, sourceId) => {
      console.log(`[renderer console] ${sourceId}:${line} ${message}`);
      // level: 0=verbose, 1=info, 2=warning, 3=error (Electron's MessageDetails.level) -- only
      // error-level fails the selftest; warnings are noisy but not indicative of a real bug the
      // way an uncaught exception or unhandled rejection is. (Used to also catch Electron's own
      // "Insecure Content-Security-Policy" warning here pre-SEC-011 fix -- contentSecurityPolicy()
      // above means that warning shouldn't fire anymore.)
      if (level >= 3) {
        consoleErrors.push(`${sourceId}:${line} ${message}`);
      }
    },
  );
  win.webContents.on(
    "did-fail-load",
    (_event, errorCode, errorDescription, validatedURL) => {
      console.error(
        `[bitvue-desktop] renderer failed to load ${validatedURL}: ${errorDescription} (${errorCode})`,
      );
    },
  );

  // TAURI_WEB-006: the OS-native window-chrome close button (the red traffic light on macOS, the
  // title-bar X on Windows/Linux) fires this event directly -- it never goes through
  // `bitvue:closeWindow`'s IPC path at all, so without this listener it bypassed
  // requestQuit()'s confirmation entirely. Prevent the default close and route through the same
  // choke point every other quit trigger uses; once requestQuit() actually confirms, it calls
  // app.quit() itself, which re-fires this same 'close' event with quitConfirmed already true,
  // letting it through for real (no infinite loop).
  win.on("close", (event) => {
    if (isQuitConfirmed()) return;
    event.preventDefault();
    void requestQuit();
  });

  setMainWindow(win);

  const renderer = resolveRendererTarget();
  console.log(
    `[bitvue-desktop] loading renderer (${renderer.kind}): ${renderer.target}`,
  );
  if (renderer.kind === "url") {
    win.loadURL(renderer.target);
  } else {
    win.loadFile(renderer.target);
  }
  return win;
}

/**
 * Gated end-to-end proof (`BITVUE_ELECTRON_SELFTEST=1`): drives the renderer's `window.bitvue`
 * bridge via `executeJavaScript` — i.e. through the *real* contextBridge/ipcRenderer/ipcMain
 * path, not by calling `SidecarClient` directly from the main process — then prints the result
 * and quits. This is what actually proves the last unproven link (renderer ↔ preload ↔ main),
 * on top of the independently-verified `bitvue-protocol`/`bitvue-sidecar`/`SidecarClient` pieces.
 */
async function runSelfTestAndExit(win: BrowserWindow): Promise<void> {
  const tempDir = mkdtempSync(path.join(tmpdir(), "bitvue-desktop-selftest-"));
  const tempFile = path.join(tempDir, "fixture.bin");
  const knownBytes = Buffer.from(Array.from({ length: 64 }, (_, i) => i));
  writeFileSync(tempFile, knownBytes);
  // Real AV1/IVF fixture, opened on stream "B" so it doesn't disturb stream "A"'s synthetic
  // byte-exact hex-range proof above -- indexStream/getFramesChunk need real IVF bytes to
  // produce anything (bitvue-indexer's diagnostic-only path for non-IVF input is already
  // covered by its own unit tests, not re-proven here).
  const realFixturePath = path.join(repoRoot, "test_data", "av1_test.ivf");

  try {
    await win.webContents.executeJavaScript(
      "new Promise((r) => setTimeout(r, 50))",
    ); // let preload settle
    // Poll for the React app to actually mount, rather than racing a fixed delay -- the app's
    // JS bundle has to load/parse/execute and do its initial render, which isn't bounded by the
    // preload-settle wait above (that's just for window.bitvue to exist, not for the page's own
    // script to have run). Times out at 5s so a real mount failure still fails loudly instead of
    // hanging.
    await win.webContents.executeJavaScript(`
      (async () => {
        const deadline = Date.now() + 5000;
        while ((document.getElementById("root")?.childElementCount ?? 0) === 0 && Date.now() < deadline) {
          await new Promise((r) => setTimeout(r, 50));
        }
      })()
    `);
    const result = await win.webContents.executeJavaScript(`
      (async () => {
        const hello = await window.bitvue.hello("selftest/0.0.1");
        const openResult = await window.bitvue.openStream("A", ${JSON.stringify(tempFile)});
        const selectResult = await window.bitvue.selectFrame("A", 3);
        const hexResult = await window.bitvue.getHexRange("A", 10, 16);
        const closeResult = await window.bitvue.closeStream("A");

        await window.bitvue.openStream("B", ${JSON.stringify(realFixturePath)});
        const indexResult = await window.bitvue.indexStream("B");
        const streamInfo = await window.bitvue.getStreamInfo("B");
        const framesChunk = await window.bitvue.getFramesChunk("B", 0, 5);
        const frameSyntax = await window.bitvue.getFrameSyntax("B", 0);
        const timeline = await window.bitvue.getTimeline("B");
        await window.bitvue.closeStream("B");

        // Debug YUV (VQ Analyzer "Load Reference YUV") always compares against stream "A" (the
        // primary analyzer file) -- reopen the real fixture there (stream "A" was already freed
        // by closeStream("A") above) to prove that path through the real renderer/preload/main
        // path too, not just bitvue-sidecar's own unit tests.
        await window.bitvue.openStream("A", ${JSON.stringify(realFixturePath)});
        const decoded = await window.bitvue.getDecodedFrameYuv("A", 0);
        const decodedBytesHex = Array.from(new Uint8Array(Object.values(decoded.bytes)))
          .map(b => b.toString(16).padStart(2, "0")).join("");
        const frameAnalysis = await window.bitvue.getFrameAnalysis(0);
        const av1Features = await window.bitvue.getAv1Features(0);
        const codingFlow = await window.bitvue.getCodingFlowAnalysis(0);
        const deblocking = await window.bitvue.getDeblockingAnalysis(0);
        const codecExtendedInfo = await window.bitvue.getCodecExtendedInfo(5);
        const residualAnalysis = await window.bitvue.getResidualAnalysis(5);
        const contextMenuItems = await window.bitvue.getContextMenuItems("Player", false, false);
        const screenshotDataUrl = await window.bitvue.captureScreenshot();
        const evidenceBundle = await window.bitvue.exportEvidenceBundle({
          outputDir: ${JSON.stringify(tempDir)},
          workspace: "player",
          mode: "normal",
          orderType: "display",
          screenshotDataUrl: screenshotDataUrl ?? undefined,
        });

        return {
          documentTitle: document.title,
          // document.title alone doesn't prove the React bundle actually executed -- it's a
          // static <title> tag that loads even if the app's JS 404s (found the hard way: a
          // Vite base-path bug meant the bundle silently failed to load under file://, leaving
          // a blank page, while this exact selftest was still reporting the right title). Check
          // real rendered DOM content too.
          rootChildCount: document.getElementById("root")?.childElementCount ?? -1,
          hello,
          openEvents: openResult.events,
          selectEvents: selectResult.events,
          closeEvents: closeResult.events,
          hexOffset: hexResult.offset,
          hexLen: hexResult.len,
          hexBytesHex: Array.from(new Uint8Array(Object.values(hexResult.bytes))).map(b => b.toString(16).padStart(2,"0")).join(""),
          indexEvents: indexResult.events,
          streamInfo,
          framesChunk,
          frameSyntax,
          timeline,
          decodedWidth: decoded.width,
          decodedHeight: decoded.height,
          decodedBytesHex,
          frameAnalysis,
          av1Features,
          codingFlow,
          deblocking,
          codecExtendedInfo,
          residualAnalysis,
          contextMenuItems,
          evidenceBundle,
        };
      })()
    `);

    // Debug YUV verification, part 2: write the exact bytes stream A just decoded out to a raw
    // I420 file (real Node fs, main process side -- the renderer can't touch the filesystem
    // directly) and use it as the reference, so "identical to what's already open" is a real,
    // independently reproducible fact instead of a fabricated fixture (same approach
    // bitvue-sidecar's own debug_yuv tests use, just proven here through the full IPC path).
    const debugYuvRefPath = path.join(tempDir, "debug_ref.i420");
    writeFileSync(debugYuvRefPath, Buffer.from(result.decodedBytesHex, "hex"));
    const debugYuvResult = await win.webContents.executeJavaScript(`
      (async () => {
        const load = await window.bitvue.loadDebugYuv({
          path: ${JSON.stringify(debugYuvRefPath)},
          width: ${result.decodedWidth},
          height: ${result.decodedHeight},
          format: "i420",
          bitdepth: 8,
        });
        const referenceFrame = await window.bitvue.getDebugYuvFrame(0, "reference");
        const referenceBytesHex = Array.from(new Uint8Array(Object.values(referenceFrame.bytes)))
          .map(b => b.toString(16).padStart(2, "0")).join("");
        const metrics = await window.bitvue.getYuvDiffMetrics(0);
        const diffFrame = await window.bitvue.getDebugYuvFrame(0, "diff");
        // "diff" mode is luma-only (see debug_yuv::get_frame's doc) -- U/V are intentionally
        // neutral 128, not 0, so only the Y portion should be all-zero here.
        const diffYBytes = Array.from(new Uint8Array(Object.values(diffFrame.bytes))).slice(0, diffFrame.yLen);
        const firstDiff = await window.bitvue.findFirstDiffFrame();
        await window.bitvue.setDebugYuvOffset(0);
        await window.bitvue.setDebugYuvCrop({ left: 0, right: 0, top: 0, bottom: 0 });
        await window.bitvue.unloadDebugYuv();
        await window.bitvue.closeStream("A");
        return { load, referenceBytesHex, metrics, diffYAllZero: diffYBytes.every((b) => b === 0), firstDiff };
      })()
    `);
    const expectedHex = knownBytes.subarray(10, 26).toString("hex");
    const selectOk = result.selectEvents?.[0]?.type === "SelectionUpdated";
    const closeOk = result.closeEvents?.[0]?.type === "ModelUpdated";
    const indexOk =
      result.indexEvents?.length === 2 &&
      result.indexEvents[0]?.kind === "Container" &&
      result.indexEvents[1]?.kind === "Units";
    const streamInfoOk =
      result.streamInfo?.indexed === true &&
      result.streamInfo?.container?.codec === "av1";
    const framesChunkOk =
      result.framesChunk?.indexed === true &&
      result.framesChunk?.units?.length === 5 &&
      result.framesChunk?.units?.[0]?.frame_type === "I";
    // Recursively find a node by name to prove real nested tree data came back -- and check its
    // bit_range against the known real file layout. Frame 0's chunk is TemporalDelimiter(2B) +
    // SequenceHeader(13B) + the real Frame OBU starting 15 bytes in (byte 44+15=59, bit 472) --
    // see bitvue-indexer's find_frame_obu doc for the real-OBU-parsing bug this pins against
    // (an earlier, wrong value here, 352, was byte 44*8 -- the Temporal Delimiter's position,
    // not the real Frame OBU's).
    interface SyntaxNodeResult {
      name: string;
      bit_range: { start_bit: number; end_bit: number };
      children?: SyntaxNodeResult[];
    }
    function findSyntaxNode(
      node: SyntaxNodeResult | undefined,
      name: string,
    ): SyntaxNodeResult | undefined {
      if (!node) return undefined;
      if (node.name === name) return node;
      for (const child of node.children ?? []) {
        const found = findSyntaxNode(child, name);
        if (found) return found;
      }
      return undefined;
    }
    const forbiddenBitNode = findSyntaxNode(
      result.frameSyntax,
      "obu_forbidden_bit",
    );
    const frameSyntaxOk = forbiddenBitNode?.bit_range?.start_bit === 472;
    const timelineOk =
      result.timeline?.stream_id === "B" &&
      result.timeline?.frames?.length > 0 &&
      result.timeline?.frames?.[0]?.marker === "Key";
    const rootMountedOk = (result.rootChildCount ?? 0) > 0;
    // Debug YUV: reference file was built from stream A's own exact decoded bytes, so reference
    // mode must round-trip those bytes unchanged, and diff/metrics must report "no difference at
    // all" -- not just "small" -- since this isn't lossy re-encoding, it's the same bytes twice.
    const debugYuvLoadOk =
      debugYuvResult.load?.success === true &&
      debugYuvResult.load?.frame_count === 1;
    const debugYuvReferenceOk =
      debugYuvResult.referenceBytesHex === result.decodedBytesHex;
    const debugYuvMetricsOk =
      debugYuvResult.metrics?.has_mismatch === false &&
      debugYuvResult.metrics?.max_diff_y === 0;
    const debugYuvDiffOk = debugYuvResult.diffYAllZero === true;
    const debugYuvFindFirstDiffOk =
      debugYuvResult.firstDiff?.frame_index === null &&
      debugYuvResult.firstDiff?.total_checked === 1;
    const frameAnalysisOk =
      result.frameAnalysis?.width === 320 &&
      result.frameAnalysis?.height === 240 &&
      (result.frameAnalysis?.qp_grid?.qp?.length ?? 0) > 0 &&
      (result.frameAnalysis?.partition_grid?.blocks?.length ?? 0) > 0;
    console.log("[selftest] get_frame_analysis OK:", frameAnalysisOk);
    const av1FeaturesOk =
      result.av1Features?.frame_index === 0 &&
      result.av1Features?.cdef?.width === 320 &&
      result.av1Features?.cdef?.height === 240 &&
      (result.av1Features?.cdef?.blocks?.length ?? 0) > 0;
    console.log("[selftest] get_av1_features OK:", av1FeaturesOk);
    const codingFlowOk =
      result.codingFlow?.frame_index === 0 &&
      result.codingFlow?.current_stage === "quantization" &&
      (result.codingFlow?.stages?.length ?? 0) === 6 &&
      (result.codingFlow?.codec_features?.length ?? 0) > 0;
    console.log("[selftest] get_coding_flow_analysis OK:", codingFlowOk);
    const deblockingOk =
      result.deblocking?.frame_index === 0 &&
      result.deblocking?.width === 320 &&
      result.deblocking?.height === 240 &&
      (result.deblocking?.edges?.length ?? 0) > 0 &&
      result.deblocking?.stats?.total_edges ===
        result.deblocking?.edges?.length;
    console.log("[selftest] get_deblocking_analysis OK:", deblockingOk);
    const codecExtendedInfoOk =
      result.codecExtendedInfo?.frame_index === 5 &&
      (result.codecExtendedInfo?.l0_refs?.length ?? 0) > 0 &&
      result.codecExtendedInfo?.l0_refs?.[0]?.frame_index < 5 &&
      result.codecExtendedInfo?.l0_refs?.[0]?.long_term === false &&
      (result.codecExtendedInfo?.qp_histogram?.length ?? 0) > 0;
    console.log("[selftest] get_codec_extended_info OK:", codecExtendedInfoOk);
    const residualAnalysisOk =
      result.residualAnalysis?.frame_index === 5 &&
      (result.residualAnalysis?.block_residuals?.length ?? 0) > 0 &&
      result.residualAnalysis?.coefficient_stats?.energy >= 0;
    console.log("[selftest] get_residual_analysis OK:", residualAnalysisOk);
    const contextMenuItemsOk =
      (result.contextMenuItems?.items?.length ?? 0) === 3 &&
      result.contextMenuItems?.items?.find(
        (i: { id: string; enabled: boolean }) => i.id === "export_bundle",
      )?.enabled === true;
    console.log("[selftest] get_context_menu_items OK:", contextMenuItemsOk);
    const evidenceBundleOk =
      result.evidenceBundle?.success === true &&
      (result.evidenceBundle?.files_created?.length ?? 0) > 0;
    // Real end-to-end proof of the 2026-08-11 screenshot-capture addition: not just that the
    // export succeeded, but that a real captured screenshot (via the actual
    // bitvue:captureScreenshot -> capturePage() path, not a mock) made it into the bundle as an
    // actual file on disk.
    const evidenceBundleScreenshotOk =
      result.evidenceBundle?.files_created?.some(
        (f: string) => f === "screenshots/screenshot_0000.png",
      ) === true &&
      existsSync(
        path.join(
          result.evidenceBundle.bundle_path,
          "screenshots/screenshot_0000.png",
        ),
      );
    console.log(
      "[selftest] export_evidence_bundle OK:",
      evidenceBundleOk,
      " screenshot included:",
      evidenceBundleScreenshotOk,
    );
    console.log(
      "[selftest] debug YUV: load OK:",
      debugYuvLoadOk,
      " reference bytes match decoded OK:",
      debugYuvReferenceOk,
      " metrics (no mismatch) OK:",
      debugYuvMetricsOk,
      " diff (all-zero) OK:",
      debugYuvDiffOk,
      " find_first_diff (none found) OK:",
      debugYuvFindFirstDiffOk,
    );
    console.log("[selftest] result:", JSON.stringify(result, null, 2));
    console.log(
      "[selftest] document title (real frontend's <title>, not the placeholder's):",
      result.documentTitle,
    );
    console.log(
      "[selftest] #root child count (proves the React bundle actually executed, not just that the HTML shell loaded):",
      result.rootChildCount,
    );
    console.log("[selftest] expected hex bytes:", expectedHex);
    console.log("[selftest] actual   hex bytes:", result.hexBytesHex);
    console.log(
      "[selftest] BYTE_EXACT_MATCH:",
      expectedHex === result.hexBytesHex,
    );
    console.log(
      "[selftest] select_frame OK:",
      selectOk,
      " close_stream OK:",
      closeOk,
    );
    console.log(
      "[selftest] index_stream OK:",
      indexOk,
      " get_stream_info OK:",
      streamInfoOk,
      " get_frames_chunk OK:",
      framesChunkOk,
      " get_frame_syntax OK:",
      frameSyntaxOk,
      " get_timeline OK:",
      timelineOk,
    );
    console.log("[selftest] #root mounted OK:", rootMountedOk);
    const noConsoleErrorsOk = consoleErrors.length === 0;
    if (!noConsoleErrorsOk) {
      console.log("[selftest] renderer console errors (FAIL):", consoleErrors);
    }
    console.log("[selftest] no renderer console errors OK:", noConsoleErrorsOk);
    process.exitCode =
      expectedHex === result.hexBytesHex &&
      selectOk &&
      closeOk &&
      indexOk &&
      streamInfoOk &&
      framesChunkOk &&
      frameSyntaxOk &&
      timelineOk &&
      rootMountedOk &&
      debugYuvLoadOk &&
      debugYuvReferenceOk &&
      debugYuvMetricsOk &&
      debugYuvDiffOk &&
      debugYuvFindFirstDiffOk &&
      frameAnalysisOk &&
      av1FeaturesOk &&
      codingFlowOk &&
      deblockingOk &&
      codecExtendedInfoOk &&
      residualAnalysisOk &&
      contextMenuItemsOk &&
      evidenceBundleOk &&
      evidenceBundleScreenshotOk &&
      noConsoleErrorsOk
        ? 0
        : 1;
  } catch (err) {
    console.error("[selftest] FAILED:", err);
    process.exitCode = 1;
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
    // app.quit() is a graceful, cancelable *request* to quit and does not take an exit code --
    // it does not reliably propagate a previously-set process.exitCode to the actual OS exit
    // code. app.exit(code) is the immediate, forceful API that actually honors it.
    app.exit(process.exitCode === undefined ? 0 : Number(process.exitCode));
  }
}

/**
 * Visual proof mode (`BITVUE_ELECTRON_SCREENSHOT=<output.png>`): drives the REAL production
 * "open file" code path -- App.tsx's `handleOpenFile`, triggered the same way a native menu
 * click would (dispatching the `"menu-open-bitstream"` DOM event it already listens for) -- with
 * the native OS dialog answered via `BITVUE_ELECTRON_SELFTEST_FIXTURE_PATH` (see
 * `registerIpcHandlers`'s `showOpenDialog` handler) instead of a human, since a real dialog can't
 * be scripted. Captures a real screenshot after the open/index/refresh chain settles, so an
 * actual human (or an agent with vision, via the Read tool) can inspect what's on screen --
 * closes a real gap none of `BITVUE_ELECTRON_SELFTEST`'s checks cover: proving *data* comes back
 * correctly says nothing about whether it actually renders visibly.
 *
 * `BITVUE_ELECTRON_SCREENSHOT_CLICK_TAB=<label>[,<label>...]` (optional): after the file
 * settles, click one or more tabs by their visible label (e.g. `"Syntax"`, or
 * `"Unit HEX,Hex"` to open a left-panel tab then a nested sub-tab within it) before capturing,
 * in order, waiting for each to render before the next -- the left dock's tabs (`App.tsx`'s
 * `LEFT_PANELS`) default to whichever was active last, so this is the only way to reliably
 * screenshot a non-default (or nested) tab instead of guessing at prior state.
 *
 * `BITVUE_ELECTRON_SCREENSHOT_CLICK_SELECTOR=<css selector>[,<css selector>...]` (optional):
 * same idea as CLICK_TAB but for elements that aren't `<button>`s with matchable text (e.g. a
 * specific filmstrip thumbnail, which is a `<div data-frame-index="N">`) -- runs after all
 * CLICK_TAB clicks, in order, one `document.querySelector(selector)` + mousedown/click per
 * entry, waiting for each to render before the next.
 *
 * `BITVUE_ELECTRON_SCREENSHOT_OPEN_DEPENDENT=1` (optional, Phase 7.5 compare workspace): after
 * stream A settles, also dispatches `"menu-open-dependent"` (same real code path as the "Open
 * dependent bitstream..." menu item) -- `BITVUE_ELECTRON_SELFTEST_FIXTURE_PATH` answers this
 * second dialog too, so the resulting compare workspace is the same fixture opened as both A
 * and B (a real, useful state: exact resolution match, `diff_enabled: true`).
 */
async function runScreenshotAndExit(
  win: BrowserWindow,
  outputPath: string,
): Promise<void> {
  try {
    await win.webContents.executeJavaScript(
      "new Promise((r) => setTimeout(r, 50))",
    ); // let preload settle
    await win.webContents.executeJavaScript(
      'window.dispatchEvent(new CustomEvent("menu-open-bitstream"))',
    );
    // Real IPC round-trips here have all been sub-second in prior selftest runs -- a generous
    // fixed wait for the open -> index -> refreshFrames chain + React re-render to settle,
    // rather than guessing at a DOM-text heuristic that could false-positive on unrelated text.
    await win.webContents.executeJavaScript(
      "new Promise((r) => setTimeout(r, 3000))",
    );
    // `BITVUE_ELECTRON_SCREENSHOT_OPEN_DEPENDENT=1` (optional, Phase 7.5 compare workspace):
    // dispatches "menu-open-dependent" the same way -- `showOpenDialog`'s
    // `BITVUE_ELECTRON_SELFTEST_FIXTURE_PATH` bypass answers this second dialog too (same path
    // as stream A, which is fine: opening a stream against itself as A/B is a real, useful
    // compare-workspace state -- exact resolution match, `diff_enabled: true`), so this needs no
    // separate env var for a second path.
    if (process.env.BITVUE_ELECTRON_SCREENSHOT_OPEN_DEPENDENT) {
      await win.webContents.executeJavaScript(
        'window.dispatchEvent(new CustomEvent("menu-open-dependent"))',
      );
      await win.webContents.executeJavaScript(
        "new Promise((r) => setTimeout(r, 3000))",
      );
    }
    const clickTabs = process.env.BITVUE_ELECTRON_SCREENSHOT_CLICK_TAB?.split(
      ",",
    )
      .map((s) => s.trim())
      .filter(Boolean);
    for (const clickTab of clickTabs ?? []) {
      // Tab markup varies by dock (DockableLayout's TabbedPanelContainer renders a bare
      // `<button class="sidebar-tab"><span>{title}</span></button>` with no dedicated label
      // class), so match on any button's trimmed text rather than a specific class name.
      await win.webContents.executeJavaScript(`
        (() => {
          const button = Array.from(document.querySelectorAll("button"))
            .find((el) => el.textContent?.trim() === ${JSON.stringify(clickTab)});
          if (!button) throw new Error(${JSON.stringify(`tab not found: ${clickTab}`)});
          // Some buttons (e.g. FilmstripDropdown's trigger) open on mousedown, not click, to
          // match native menu-button feel -- element.click() only synthesizes a "click" event
          // per the DOM spec, never "mousedown"/"mouseup", so those handlers silently never fire
          // through this harness even though a real user's click works fine (found while
          // screenshot-sweeping the filmstrip's view-mode dropdown: the trigger appeared to do
          // nothing, then the next chained click failed with "tab not found" since the dropdown
          // never actually opened). Dispatching a real mousedown before click() covers both
          // conventions without needing to know which one a given button uses.
          button.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true }));
          button.click();
        })()
      `);
      await win.webContents.executeJavaScript(
        "new Promise((r) => setTimeout(r, 500))",
      );
    }
    const clickSelectors =
      process.env.BITVUE_ELECTRON_SCREENSHOT_CLICK_SELECTOR?.split(",")
        .map((s) => s.trim())
        .filter(Boolean);
    for (const selector of clickSelectors ?? []) {
      await win.webContents.executeJavaScript(`
        (() => {
          const el = document.querySelector(${JSON.stringify(selector)});
          if (!el) throw new Error(${JSON.stringify(`selector not found: ${selector}`)});
          el.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true }));
          el.click();
        })()
      `);
      await win.webContents.executeJavaScript(
        "new Promise((r) => setTimeout(r, 500))",
      );
    }
    const image = await win.webContents.capturePage();
    writeFileSync(outputPath, image.toPNG());
    console.log(`[screenshot] saved to ${outputPath}`);
    // Same reasoning as runSelfTestAndExit's noConsoleErrorsOk check -- a screenshot can look
    // completely fine while the renderer silently threw (that's exactly how macOS's broken menu
    // system went unnoticed all session: this mode is what a human/agent uses to visually
    // inspect the app, and a visual inspection alone won't reveal an off-screen unhandled
    // rejection).
    if (consoleErrors.length > 0) {
      console.error(
        "[screenshot] renderer console errors (FAIL):",
        consoleErrors,
      );
      process.exitCode = 1;
    } else {
      process.exitCode = 0;
    }
  } catch (err) {
    console.error("[screenshot] FAILED:", err);
    process.exitCode = 1;
  } finally {
    // See runSelfTestAndExit's finally block: app.quit() does not reliably propagate
    // process.exitCode to the actual OS exit code, app.exit(code) does.
    app.exit(process.exitCode === undefined ? 0 : Number(process.exitCode));
  }
}

async function main(): Promise<void> {
  if (!existsSync(sidecarBinaryPath)) {
    throw new Error(
      `bitvue-sidecar binary not found at ${sidecarBinaryPath}.\n` +
        `Build it first: cargo build -p bitvue-sidecar (from the repo root).\n` +
        `Or point BITVUE_SIDECAR_BIN at an already-built binary.`,
    );
  }

  // See sidecarClient.ts's module doc: this restarts the *process* on an unexpected crash, not
  // bitvue_engine::Core's in-memory state (no open streams/selection survive a crash — there's
  // nothing to replay them from). The renderer is told via 'bitvue:sidecar-restarted' below so
  // real UI can react (e.g. prompt the user to re-open their file); this shell doesn't do that
  // itself yet since there's no real UI to prompt.
  sidecar = new SidecarClient(sidecarBinaryPath, {
    restart: { maxAttempts: 3, backoffMs: 500 },
  });
  sidecar.on("exit", (code, signal) => {
    if (shuttingDown) return; // expected — we called sidecar.close() ourselves
    console.error(
      `[bitvue-desktop] sidecar exited unexpectedly (code=${code} signal=${signal})`,
    );
  });
  sidecar.on("restart_failed", (attempts: number) => {
    console.error(
      `[bitvue-desktop] sidecar would not stay up after ${attempts} restart attempt(s), giving up`,
    );
  });

  const hello = await sidecar.hello("bitvue-desktop/0.0.1");
  console.log(
    `[bitvue-desktop] sidecar handshake ok, protocol_version=${hello.protocol_version}, pid=${sidecar.pid}`,
  );

  // See contentSecurityPolicy's doc above -- must be registered before createWindow()'s
  // loadFile/loadURL call so the very first document response already carries the header.
  const csp = contentSecurityPolicy();
  session.defaultSession.webRequest.onHeadersReceived((details, callback) => {
    callback({
      responseHeaders: {
        ...details.responseHeaders,
        "Content-Security-Policy": [csp],
      },
    });
  });

  registerIpcHandlers();
  const win = createWindow();
  installNativeMacMenu(win);

  sidecar.on("restarted", (attempt: number) => {
    console.log(
      `[bitvue-desktop] sidecar restarted (attempt ${attempt}) — application state was lost`,
    );
    win.webContents.send("bitvue:sidecar-restarted");
  });

  if (process.env.BITVUE_ELECTRON_SELFTEST === "1") {
    win.webContents.once("did-finish-load", () => {
      runSelfTestAndExit(win);
    });
  }

  if (process.env.BITVUE_ELECTRON_SCREENSHOT) {
    win.webContents.once("did-finish-load", () => {
      runScreenshotAndExit(
        win,
        process.env.BITVUE_ELECTRON_SCREENSHOT as string,
      );
    });
  }
}

app
  .whenReady()
  .then(main)
  .catch((err) => {
    console.error("[bitvue-desktop] fatal startup error:", err);
    app.exit(1);
  });

app.on("window-all-closed", () => {
  // Only reached once a window has actually closed for real -- with the TAURI_WEB-006 gate in
  // createWindow()'s 'close' listener, that only happens after requestQuit() has confirmed (or
  // decided confirmation wasn't needed), so this cleanup can't fire while a "Quit Bitvue?" dialog
  // is still pending / could still be cancelled.
  shuttingDown = true;
  sidecar?.close();
  if (process.platform !== "darwin") app.quit();
});

// TAURI_WEB-006: the other half of requestQuit()'s choke point (alongside createWindow()'s
// 'close' listener) -- catches every app.quit() call, including `bitvue:closeWindow`'s IPC
// handler and the native macOS menu's "Quit" item (installNativeMacMenu's `menu-quit` dispatch,
// via App.tsx's closeWindow() -> requestQuit() -> app.quit()). Before this fix, `before-quit`
// closed the sidecar unconditionally and immediately, which -- once requestQuit() could
// legitimately delay or cancel a quit -- would have killed the sidecar out from under a still-
// running app if the user hit Cancel in the confirmation dialog. Now it only ever runs cleanup
// once quitConfirmed is true (i.e. after requestQuit() already decided to actually quit);
// otherwise it defers to requestQuit() the same way the window 'close' listener does.
app.on("before-quit", (event) => {
  if (isQuitConfirmed()) {
    shuttingDown = true;
    return;
  }
  event.preventDefault();
  void requestQuit();
});
