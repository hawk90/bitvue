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

  ipcMain.handle("bitvue:closeStream", async (_event, stream: string) => {
    return requireSidecar().request("close_stream", { stream });
  });

  ipcMain.handle("bitvue:selectFrame", async (_event, stream: string, frameIndex: number) => {
    return requireSidecar().request("select_frame", { stream, frame_index: frameIndex });
  });

  ipcMain.handle(
    "bitvue:showOpenDialog",
    async (event, filters?: Array<{ name: string; extensions: string[] }>) => {
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

  try {
    await win.webContents.executeJavaScript("new Promise((r) => setTimeout(r, 50))"); // let preload settle
    const result = await win.webContents.executeJavaScript(`
      (async () => {
        const hello = await window.bitvue.hello("selftest/0.0.1");
        const openResult = await window.bitvue.openStream("A", ${JSON.stringify(tempFile)});
        const selectResult = await window.bitvue.selectFrame("A", 3);
        const hexResult = await window.bitvue.getHexRange("A", 10, 16);
        const closeResult = await window.bitvue.closeStream("A");
        return {
          documentTitle: document.title,
          hello,
          openEvents: openResult.events,
          selectEvents: selectResult.events,
          closeEvents: closeResult.events,
          hexOffset: hexResult.offset,
          hexLen: hexResult.len,
          hexBytesHex: Array.from(new Uint8Array(Object.values(hexResult.bytes))).map(b => b.toString(16).padStart(2,"0")).join(""),
        };
      })()
    `);
    const expectedHex = knownBytes.subarray(10, 26).toString("hex");
    const selectOk = result.selectEvents?.[0]?.type === "SelectionUpdated";
    const closeOk = result.closeEvents?.[0]?.type === "ModelUpdated";
    console.log("[selftest] result:", JSON.stringify(result, null, 2));
    console.log("[selftest] document title (proves the real frontend loaded, not the placeholder):", result.documentTitle);
    console.log("[selftest] expected hex bytes:", expectedHex);
    console.log("[selftest] actual   hex bytes:", result.hexBytesHex);
    console.log("[selftest] BYTE_EXACT_MATCH:", expectedHex === result.hexBytesHex);
    console.log("[selftest] select_frame OK:", selectOk, " close_stream OK:", closeOk);
    process.exitCode = expectedHex === result.hexBytesHex && selectOk && closeOk ? 0 : 1;
  } catch (err) {
    console.error("[selftest] FAILED:", err);
    process.exitCode = 1;
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
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
