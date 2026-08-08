// Electron preload script — runs in an isolated context with access to Node + a restricted
// `ipcRenderer`, bridged into the renderer's `window.bitvue` via `contextBridge`.
//
// Deliberately plain CommonJS, not compiled from TypeScript: Electron's preload loading has
// historically been the least forgiving part of the process model around module systems
// (sandboxed preload + ESM has version-dependent caveats), and this file's logic is small
// enough that hand-writing it avoids that whole class of build problem. Keep it in sync with
// `electron/main.ts`'s `ipcMain.handle` channel names by hand — there are only 3 today.
//
// This is a prototype proving the bridge, not a hardened renderer boundary: it exposes a
// thin, fixed set of named methods (not a generic "invoke any channel" passthrough), which is
// the right shape long-term too — just don't mistake the current lack of input validation on
// the main-process side for something safe to expose more broadly without adding it.

const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("bitvue", {
  hello: (clientVersion) => ipcRenderer.invoke("bitvue:hello", clientVersion),
  openStream: (stream, filePath) => ipcRenderer.invoke("bitvue:openStream", stream, filePath),
  getHexRange: (stream, offset, len) => ipcRenderer.invoke("bitvue:getHexRange", stream, offset, len),
});
