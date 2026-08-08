/**
 * Electron sidecar bridge — the replacement for the old `@tauri-apps/api` `invoke()`-based
 * command layer (`tauriCommandService.ts`, and the various files that called `invoke()`
 * directly). Thin wrappers around `window.bitvue.*`, exposed by
 * `bitvue-desktop/electron/preload.cjs` via `contextBridge`.
 *
 * IMPORTANT — the old Tauri command surface (~40 commands, `src-tauri/src/commands/*.rs`,
 * deleted 2026-08-08) and the new `bitvue-sidecar` surface (9 commands, see
 * `docs/DEVELOPMENT_PHASES.md` § "제품 아키텍처 확정") are NOT the same API — different names,
 * different param/result shapes, and most of the old capability (frame decode/YUV pixel data,
 * compare workspaces, export, quality metrics) has no sidecar equivalent implemented yet (only
 * `bitvue_engine::Core`'s 7 real `Command` handlers are ported — see `bitvue-sidecar`'s module
 * doc). This file only wraps what actually exists today: open/close a stream, select a frame
 * (selection-sync only — no decoded pixel data comes back), read a raw hex byte range, and the
 * native open-file dialog. Don't add wrappers here for capabilities the sidecar doesn't have;
 * that would silently promise something broken.
 */

export type StreamId = "A" | "B";

/** Shape of the JSON-mapped `bitvue_engine::Event` values the sidecar sends back — see
 *  `bitvue-sidecar/src/main.rs`'s `event_to_json` for the authoritative field set per type. */
export interface BridgeEvent {
  type: string;
  stream?: string;
  [key: string]: unknown;
}

export interface OpenStreamResult {
  /** False if the only event back was a `DiagnosticAdded` (severity Error) — see
   *  `Core::handle_command`'s design note: failures aren't a wire-level error, they're an event,
   *  same as the UI would eventually see them. */
  success: boolean;
  path: string;
  events: BridgeEvent[];
  /** Present only when `success` is false. */
  error?: string;
}

declare global {
  interface Window {
    bitvue?: {
      hello: (
        clientVersion?: string,
      ) => Promise<{ protocol_version: string; capabilities: string[] }>;
      openStream: (
        stream: StreamId,
        filePath: string,
      ) => Promise<{ events: BridgeEvent[] }>;
      closeStream: (stream: StreamId) => Promise<{ events: BridgeEvent[] }>;
      selectFrame: (
        stream: StreamId,
        frameIndex: number,
      ) => Promise<{ events: BridgeEvent[] }>;
      getHexRange: (
        stream: StreamId,
        offset: number,
        len: number,
      ) => Promise<{ offset: number; len: number; bytes: Uint8Array }>;
      showOpenDialog: (
        filters?: Array<{ name: string; extensions: string[] }>,
      ) => Promise<string | null>;
      onSidecarRestarted: (callback: () => void) => () => void;
    };
  }
}

function requireBridge(): NonNullable<Window["bitvue"]> {
  if (!window.bitvue) {
    throw new Error(
      "window.bitvue is unavailable — this page isn't running inside the bitvue-desktop Electron " +
        "shell (preload didn't run), or it's a plain browser tab.",
    );
  }
  return window.bitvue;
}

/** Whether the Electron bridge is available at all — for callers that want to branch/skip. */
export function hasElectronBridge(): boolean {
  return typeof window !== "undefined" && Boolean(window.bitvue);
}

export async function openStream(
  stream: StreamId,
  path: string,
): Promise<OpenStreamResult> {
  const { events } = await requireBridge().openStream(stream, path);
  const diagnostic = events.find((e) => e.type === "DiagnosticAdded");
  return {
    success: !diagnostic,
    path,
    events,
    error: diagnostic ? String(diagnostic.diagnostic) : undefined,
  };
}

export async function closeStream(stream: StreamId): Promise<BridgeEvent[]> {
  const { events } = await requireBridge().closeStream(stream);
  return events;
}

export async function selectFrame(
  stream: StreamId,
  frameIndex: number,
): Promise<BridgeEvent[]> {
  const { events } = await requireBridge().selectFrame(stream, frameIndex);
  return events;
}

export async function getHexRange(
  stream: StreamId,
  offset: number,
  len: number,
): Promise<{ offset: number; len: number; bytes: Uint8Array }> {
  return requireBridge().getHexRange(stream, offset, len);
}

export interface OpenFileDialogFilter {
  name: string;
  extensions: string[];
}

/** Native "open file" dialog via the main process. Returns the selected path, or null if cancelled. */
export async function showOpenDialog(
  filters?: OpenFileDialogFilter[],
): Promise<string | null> {
  return requireBridge().showOpenDialog(filters);
}
