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

import { app, BrowserWindow, dialog, ipcMain } from "electron";
import { existsSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { SidecarClient } from "../src/sidecarClient.js";

const here = path.dirname(fileURLToPath(import.meta.url));
// dist/electron/main.js -> dist/electron -> dist -> bitvue-desktop -> repo root
const repoRoot = path.resolve(here, "..", "..", "..");
const defaultBinaryName = process.platform === "win32" ? "bitvue-sidecar.exe" : "bitvue-sidecar";
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

function requireSidecar(): SidecarClient {
  if (!sidecar) throw new Error("sidecar not started yet — this shouldn't happen post-app.whenReady()");
  return sidecar;
}

function registerIpcHandlers(): void {
  ipcMain.handle("bitvue:hello", async (_event, clientVersion?: string) => {
    return requireSidecar().hello(clientVersion);
  });

  ipcMain.handle("bitvue:openStream", async (_event, stream: string, filePath: string) => {
    return requireSidecar().request("open_stream", { stream, path: filePath });
  });

  ipcMain.handle("bitvue:getHexRange", async (_event, stream: string, offset: number, len: number) => {
    const { bytes, ...metadata } = await requireSidecar().getHexRange({ stream, offset, len });
    // Buffer survives Electron's structured-clone IPC as-is (renderer sees a Uint8Array) —
    // no base64/JSON-array encoding. This is the whole point of the migration; see the
    // `YUVFrameData` anti-pattern note in DEVELOPMENT_PHASES.md.
    return { ...metadata, bytes };
  });

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

  ipcMain.handle("bitvue:closeStream", async (_event, stream: string) => {
    return requireSidecar().request("close_stream", { stream });
  });

  ipcMain.handle("bitvue:selectFrame", async (_event, stream: string, frameIndex: number) => {
    return requireSidecar().request("select_frame", { stream, frame_index: frameIndex });
  });

  ipcMain.handle("bitvue:indexStream", async (_event, stream: string) => {
    return requireSidecar().request("index_stream", { stream });
  });

  ipcMain.handle("bitvue:getStreamInfo", async (_event, stream: string) => {
    return requireSidecar().request("get_stream_info", { stream });
  });

  ipcMain.handle(
    "bitvue:getFramesChunk",
    async (_event, stream: string, offset: number, limit: number) => {
      return requireSidecar().request("get_frames_chunk", { stream, offset, limit });
    },
  );

  ipcMain.handle("bitvue:getFrameSyntax", async (_event, stream: string, frameIndex: number) => {
    return requireSidecar().request("get_frame_syntax", { stream, frame_index: frameIndex });
  });

  ipcMain.handle("bitvue:getTimeline", async (_event, stream: string) => {
    return requireSidecar().request("get_timeline", { stream });
  });

  ipcMain.handle(
    "bitvue:selectUnit",
    async (_event, stream: string, unitType: string, offset: number, size: number) => {
      return requireSidecar().request("select_unit", { stream, unit_type: unitType, offset, size });
    },
  );

  ipcMain.handle(
    "bitvue:selectSyntax",
    async (_event, stream: string, nodeId: string, startBit: number, endBit: number) => {
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
    async (_event, stream: string, x: number, y: number, w: number, h: number) => {
      return requireSidecar().request("select_spatial_block", { stream, x, y, w, h });
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
      const result = win ? await dialog.showOpenDialog(win, options) : await dialog.showOpenDialog(options);
      if (result.canceled || result.filePaths.length === 0) return null;
      return result.filePaths[0];
    },
  );
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
  if (existsSync(frontendDistIndex)) return { kind: "file", target: frontendDistIndex };
  console.warn(
    `[bitvue-desktop] frontend dist not found (${frontendDistIndex}) and BITVUE_FRONTEND_URL not set — ` +
      "loading the placeholder shell instead of the real Bitvue Analyzer UI. " +
      "Run `cd frontend && npm run build`, or set BITVUE_FRONTEND_URL to a dev server.",
  );
  return { kind: "file", target: path.join(here, "index.html") };
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
  win.webContents.on("console-message", (_event, _level, message, line, sourceId) => {
    console.log(`[renderer console] ${sourceId}:${line} ${message}`);
  });
  win.webContents.on("did-fail-load", (_event, errorCode, errorDescription, validatedURL) => {
    console.error(`[bitvue-desktop] renderer failed to load ${validatedURL}: ${errorDescription} (${errorCode})`);
  });

  const renderer = resolveRendererTarget();
  console.log(`[bitvue-desktop] loading renderer (${renderer.kind}): ${renderer.target}`);
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
    await win.webContents.executeJavaScript("new Promise((r) => setTimeout(r, 50))"); // let preload settle
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
        };
      })()
    `);
    const expectedHex = knownBytes.subarray(10, 26).toString("hex");
    const selectOk = result.selectEvents?.[0]?.type === "SelectionUpdated";
    const closeOk = result.closeEvents?.[0]?.type === "ModelUpdated";
    const indexOk =
      result.indexEvents?.length === 2 &&
      result.indexEvents[0]?.kind === "Container" &&
      result.indexEvents[1]?.kind === "Units";
    const streamInfoOk = result.streamInfo?.indexed === true && result.streamInfo?.container?.codec === "av1";
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
    const forbiddenBitNode = findSyntaxNode(result.frameSyntax, "obu_forbidden_bit");
    const frameSyntaxOk = forbiddenBitNode?.bit_range?.start_bit === 472;
    const timelineOk =
      result.timeline?.stream_id === "B" &&
      result.timeline?.frames?.length > 0 &&
      result.timeline?.frames?.[0]?.marker === "Key";
    const rootMountedOk = (result.rootChildCount ?? 0) > 0;
    console.log("[selftest] result:", JSON.stringify(result, null, 2));
    console.log("[selftest] document title (real frontend's <title>, not the placeholder's):", result.documentTitle);
    console.log("[selftest] #root child count (proves the React bundle actually executed, not just that the HTML shell loaded):", result.rootChildCount);
    console.log("[selftest] expected hex bytes:", expectedHex);
    console.log("[selftest] actual   hex bytes:", result.hexBytesHex);
    console.log("[selftest] BYTE_EXACT_MATCH:", expectedHex === result.hexBytesHex);
    console.log("[selftest] select_frame OK:", selectOk, " close_stream OK:", closeOk);
    console.log(
      "[selftest] index_stream OK:", indexOk,
      " get_stream_info OK:", streamInfoOk,
      " get_frames_chunk OK:", framesChunkOk,
      " get_frame_syntax OK:", frameSyntaxOk,
      " get_timeline OK:", timelineOk,
    );
    console.log("[selftest] #root mounted OK:", rootMountedOk);
    process.exitCode =
      expectedHex === result.hexBytesHex &&
      selectOk &&
      closeOk &&
      indexOk &&
      streamInfoOk &&
      framesChunkOk &&
      frameSyntaxOk &&
      timelineOk &&
      rootMountedOk
        ? 0
        : 1;
  } catch (err) {
    console.error("[selftest] FAILED:", err);
    process.exitCode = 1;
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
    app.quit();
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
 * `BITVUE_ELECTRON_SCREENSHOT_CLICK_TAB=<label>` (optional): after the file settles, click a
 * left-panel tab by its visible label (e.g. `"Syntax"`) before capturing -- the left dock's
 * tabs (`App.tsx`'s `LEFT_PANELS`) default to whichever was active last, so this is the only
 * way to reliably screenshot a non-default tab like Syntax instead of guessing at prior state.
 */
async function runScreenshotAndExit(win: BrowserWindow, outputPath: string): Promise<void> {
  try {
    await win.webContents.executeJavaScript("new Promise((r) => setTimeout(r, 50))"); // let preload settle
    await win.webContents.executeJavaScript(
      'window.dispatchEvent(new CustomEvent("menu-open-bitstream"))',
    );
    // Real IPC round-trips here have all been sub-second in prior selftest runs -- a generous
    // fixed wait for the open -> index -> refreshFrames chain + React re-render to settle,
    // rather than guessing at a DOM-text heuristic that could false-positive on unrelated text.
    await win.webContents.executeJavaScript("new Promise((r) => setTimeout(r, 3000))");
    const clickTab = process.env.BITVUE_ELECTRON_SCREENSHOT_CLICK_TAB;
    if (clickTab) {
      // Tab markup varies by dock (DockableLayout's TabbedPanelContainer renders a bare
      // `<button class="sidebar-tab"><span>{title}</span></button>` with no dedicated label
      // class), so match on any button's trimmed text rather than a specific class name.
      await win.webContents.executeJavaScript(`
        (() => {
          const button = Array.from(document.querySelectorAll("button"))
            .find((el) => el.textContent?.trim() === ${JSON.stringify(clickTab)});
          if (!button) throw new Error(${JSON.stringify(`tab not found: ${clickTab}`)});
          button.click();
        })()
      `);
      await win.webContents.executeJavaScript("new Promise((r) => setTimeout(r, 500))");
    }
    const image = await win.webContents.capturePage();
    writeFileSync(outputPath, image.toPNG());
    console.log(`[screenshot] saved to ${outputPath}`);
    process.exitCode = 0;
  } catch (err) {
    console.error("[screenshot] FAILED:", err);
    process.exitCode = 1;
  } finally {
    app.quit();
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
  sidecar = new SidecarClient(sidecarBinaryPath, { restart: { maxAttempts: 3, backoffMs: 500 } });
  sidecar.on("exit", (code, signal) => {
    if (shuttingDown) return; // expected — we called sidecar.close() ourselves
    console.error(`[bitvue-desktop] sidecar exited unexpectedly (code=${code} signal=${signal})`);
  });
  sidecar.on("restart_failed", (attempts: number) => {
    console.error(`[bitvue-desktop] sidecar would not stay up after ${attempts} restart attempt(s), giving up`);
  });

  const hello = await sidecar.hello("bitvue-desktop/0.0.1");
  console.log(
    `[bitvue-desktop] sidecar handshake ok, protocol_version=${hello.protocol_version}, pid=${sidecar.pid}`,
  );

  registerIpcHandlers();
  const win = createWindow();

  sidecar.on("restarted", (attempt: number) => {
    console.log(`[bitvue-desktop] sidecar restarted (attempt ${attempt}) — application state was lost`);
    win.webContents.send("bitvue:sidecar-restarted");
  });

  if (process.env.BITVUE_ELECTRON_SELFTEST === "1") {
    win.webContents.once("did-finish-load", () => {
      runSelfTestAndExit(win);
    });
  }

  if (process.env.BITVUE_ELECTRON_SCREENSHOT) {
    win.webContents.once("did-finish-load", () => {
      runScreenshotAndExit(win, process.env.BITVUE_ELECTRON_SCREENSHOT as string);
    });
  }
}

app.whenReady().then(main).catch((err) => {
  console.error("[bitvue-desktop] fatal startup error:", err);
  app.exit(1);
});

app.on("window-all-closed", () => {
  shuttingDown = true;
  sidecar?.close();
  if (process.platform !== "darwin") app.quit();
});

app.on("before-quit", () => {
  shuttingDown = true;
  sidecar?.close();
});
