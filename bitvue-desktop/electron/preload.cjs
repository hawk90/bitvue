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
  getDecodedFrameYuv: (stream, frameIndex) =>
    ipcRenderer.invoke("bitvue:getDecodedFrameYuv", stream, frameIndex),
  // Debug YUV (VQ Analyzer "Load Reference YUV" workflow) -- one global reference-file session,
  // not per-stream. See bitvue-sidecar's debug_yuv module doc.
  loadDebugYuv: (params) => ipcRenderer.invoke("bitvue:loadDebugYuv", params),
  unloadDebugYuv: () => ipcRenderer.invoke("bitvue:unloadDebugYuv"),
  setDebugYuvOffset: (offset) => ipcRenderer.invoke("bitvue:setDebugYuvOffset", offset),
  setDebugYuvCrop: (crop) => ipcRenderer.invoke("bitvue:setDebugYuvCrop", crop),
  getYuvDiffMetrics: (frameIndex) => ipcRenderer.invoke("bitvue:getYuvDiffMetrics", frameIndex),
  findFirstDiffFrame: () => ipcRenderer.invoke("bitvue:findFirstDiffFrame"),
  getDebugYuvFrame: (frameIndex, mode, amplify) =>
    ipcRenderer.invoke("bitvue:getDebugYuvFrame", frameIndex, mode, amplify),
  getFrameAnalysis: (frameIndex) => ipcRenderer.invoke("bitvue:getFrameAnalysis", frameIndex),
  getAv1Features: (frameIndex) => ipcRenderer.invoke("bitvue:getAv1Features", frameIndex),
  getCodingFlowAnalysis: (frameIndex) =>
    ipcRenderer.invoke("bitvue:getCodingFlowAnalysis", frameIndex),
  getDeblockingAnalysis: (frameIndex) =>
    ipcRenderer.invoke("bitvue:getDeblockingAnalysis", frameIndex),
  getCodecExtendedInfo: (frameIndex) =>
    ipcRenderer.invoke("bitvue:getCodecExtendedInfo", frameIndex),
  getResidualAnalysis: (frameIndex) =>
    ipcRenderer.invoke("bitvue:getResidualAnalysis", frameIndex),
  getContextMenuItems: (scope, hasSelection, hasByteRange) =>
    ipcRenderer.invoke("bitvue:getContextMenuItems", scope, hasSelection, hasByteRange),
  exportEvidenceBundle: (params) => ipcRenderer.invoke("bitvue:exportEvidenceBundle", params),
  showDirectoryDialog: () => ipcRenderer.invoke("bitvue:showDirectoryDialog"),
  indexStream: (stream) => ipcRenderer.invoke("bitvue:indexStream", stream),
  getStreamInfo: (stream) => ipcRenderer.invoke("bitvue:getStreamInfo", stream),
  getFramesChunk: (stream, offset, limit) =>
    ipcRenderer.invoke("bitvue:getFramesChunk", stream, offset, limit),
  getFrameSyntax: (stream, frameIndex) => ipcRenderer.invoke("bitvue:getFrameSyntax", stream, frameIndex),
  getTimeline: (stream) => ipcRenderer.invoke("bitvue:getTimeline", stream),
  getThumbnails: (stream, frameIndices, targetWidth) =>
    ipcRenderer.invoke("bitvue:getThumbnails", stream, frameIndices, targetWidth),
  // Structural (multi-sync) selection commands -- independent of selectFrame's temporal cursor.
  // See bitvue-sidecar's module doc / bitvue_engine::selection for the tri-sync design.
  selectUnit: (stream, unitType, offset, size) =>
    ipcRenderer.invoke("bitvue:selectUnit", stream, unitType, offset, size),
  selectSyntax: (stream, nodeId, startBit, endBit) =>
    ipcRenderer.invoke("bitvue:selectSyntax", stream, nodeId, startBit, endBit),
  selectBitRange: (stream, startBit, endBit) =>
    ipcRenderer.invoke("bitvue:selectBitRange", stream, startBit, endBit),
  selectSpatialBlock: (stream, x, y, w, h) =>
    ipcRenderer.invoke("bitvue:selectSpatialBlock", stream, x, y, w, h),
  // Native "open file" dialog, proxied through main (renderers can't call Electron's dialog API
  // directly). Returns the selected path, or null if the user cancelled. `filters` matches
  // Electron's `dialog.showOpenDialog` FileFilter shape: [{name, extensions}].
  showOpenDialog: (filters) => ipcRenderer.invoke("bitvue:showOpenDialog", filters),
  closeWindow: () => ipcRenderer.invoke("bitvue:closeWindow"),
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
