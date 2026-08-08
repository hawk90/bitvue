// Electron preload script — runs in an isolated context with access to Node + a restricted
// `ipcRenderer`, bridged into the renderer's `window.bitvue` via `contextBridge`.
//
// Deliberately plain CommonJS, not compiled from TypeScript: Electron's preload loading has
// historically been the least forgiving part of the process model around module systems
// (sandboxed preload + ESM has version-dependent caveats), and this file's logic is small
// enough that hand-writing it avoids that whole class of build problem. Keep it in sync with
// `electron/main.ts`'s `ipcMain.handle` channel names by hand, plus one main-to-renderer push
// (`bitvue:sidecar-restarted`, see `onSidecarRestarted` below).
//
// This is a prototype proving the bridge, not a hardened renderer boundary: it exposes a
// thin, fixed set of named methods (not a generic "invoke any channel" passthrough), which is
// the right shape long-term too — just don't mistake the current lack of input validation on
// the main-process side for something safe to expose more broadly without adding it.

const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("bitvue", {
  hello: (clientVersion) => ipcRenderer.invoke("bitvue:hello", clientVersion),
  openStream: (stream, filePath) => ipcRenderer.invoke("bitvue:openStream", stream, filePath),
  closeStream: (stream) => ipcRenderer.invoke("bitvue:closeStream", stream),
  selectFrame: (stream, frameIndex) => ipcRenderer.invoke("bitvue:selectFrame", stream, frameIndex),
  getHexRange: (stream, offset, len) => ipcRenderer.invoke("bitvue:getHexRange", stream, offset, len),
  indexStream: (stream) => ipcRenderer.invoke("bitvue:indexStream", stream),
  getStreamInfo: (stream) => ipcRenderer.invoke("bitvue:getStreamInfo", stream),
  getFramesChunk: (stream, offset, limit) =>
    ipcRenderer.invoke("bitvue:getFramesChunk", stream, offset, limit),
  getFrameSyntax: (stream, frameIndex) => ipcRenderer.invoke("bitvue:getFrameSyntax", stream, frameIndex),
  // Native "open file" dialog, proxied through main (renderers can't call Electron's dialog API
  // directly). Returns the selected path, or null if the user cancelled. `filters` matches
  // Electron's `dialog.showOpenDialog` FileFilter shape: [{name, extensions}].
  showOpenDialog: (filters) => ipcRenderer.invoke("bitvue:showOpenDialog", filters),
  // Fires after the sidecar process crashed and was automatically respawned (see
  // sidecarClient.ts's crash-recovery doc) — application state (open streams, selection) was
  // lost and is NOT restored automatically. A real UI should use this to prompt the user to
  // re-open whatever they had open. Returns an unsubscribe function.
  onSidecarRestarted: (callback) => {
    const listener = () => callback();
    ipcRenderer.on("bitvue:sidecar-restarted", listener);
    return () => ipcRenderer.removeListener("bitvue:sidecar-restarted", listener);
  },
});
