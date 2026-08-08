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

import { app, BrowserWindow, ipcMain } from "electron";
import { existsSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { SidecarClient } from "../src/sidecarClient.js";

const here = path.dirname(fileURLToPath(import.meta.url));
// dist/electron/main.js -> dist/electron -> dist -> bitvue-desktop -> repo root
const repoRoot = path.resolve(here, "..", "..", "..");
const defaultBinaryName = process.platform === "win32" ? "bitvue-sidecar.exe" : "bitvue-sidecar";
const sidecarBinaryPath = process.env.BITVUE_SIDECAR_BIN ?? path.join(repoRoot, "target", "debug", defaultBinaryName);

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
}

function createWindow(): BrowserWindow {
  const win = new BrowserWindow({
    width: 1024,
    height: 768,
    show: process.env.BITVUE_ELECTRON_OFFSCREEN !== "1",
    webPreferences: {
      preload: path.join(here, "preload.cjs"),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: false, // see module doc — simplification, revisit before shipping
      offscreen: process.env.BITVUE_ELECTRON_OFFSCREEN === "1",
    },
  });
  win.loadFile(path.join(here, "index.html"));
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
        const hexResult = await window.bitvue.getHexRange("A", 10, 16);
        return {
          hello,
          openEvents: openResult.events,
          hexOffset: hexResult.offset,
          hexLen: hexResult.len,
          hexBytesHex: Array.from(new Uint8Array(Object.values(hexResult.bytes))).map(b => b.toString(16).padStart(2,"0")).join(""),
        };
      })()
    `);
    const expectedHex = knownBytes.subarray(10, 26).toString("hex");
    console.log("[selftest] result:", JSON.stringify(result, null, 2));
    console.log("[selftest] expected hex bytes:", expectedHex);
    console.log("[selftest] actual   hex bytes:", result.hexBytesHex);
    console.log("[selftest] BYTE_EXACT_MATCH:", expectedHex === result.hexBytesHex);
    process.exitCode = expectedHex === result.hexBytesHex ? 0 : 1;
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

  sidecar = new SidecarClient(sidecarBinaryPath);
  sidecar.on("exit", (code, signal) => {
    if (shuttingDown) return; // expected — we called sidecar.close() ourselves
    console.error(`[bitvue-desktop] sidecar exited unexpectedly (code=${code} signal=${signal})`);
  });

  const hello = await sidecar.hello("bitvue-desktop/0.0.1");
  console.log(`[bitvue-desktop] sidecar handshake ok, protocol_version=${hello.protocol_version}`);

  registerIpcHandlers();
  const win = createWindow();

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
