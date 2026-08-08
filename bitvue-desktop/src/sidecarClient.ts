/**
 * TypeScript client for `bitvue-sidecar` — spawns the compiled Rust binary and speaks
 * `bitvue-protocol` over its stdio. This is the Electron-main-process half of the
 * napi-rs-vs-sidecar bridge decision recorded in `docs/DEVELOPMENT_PHASES.md`
 * § "제품 아키텍처 확정"; the Rust half lives in `crates/bitvue-protocol` + `crates/bitvue-sidecar`.
 *
 * Deliberately built on `child_process.spawn` (Node core, no `electron` dependency) — Electron's
 * main process uses the identical API, so this module works unchanged once actually imported
 * from a real Electron main process. No `BrowserWindow`/IPC-to-renderer code here by design;
 * that's a separate, later piece of work.
 *
 * Design choices (documented per the task's "your call" points):
 *  - `Data`- and `Event`-kind frames aren't part of the request/response flow (the sidecar can
 *    send a `Data` frame unprompted after a `Control` response, or an `Event` frame at any time
 *    with `correlation_id = 0`). Rather than have `request()` awkwardly return a
 *    `{result, dataFrames}` tuple that's `undefined` for the overwhelming majority of methods,
 *    this client extends `EventEmitter` and emits `'data'` / `'event'` for those frame kinds.
 *    Callers that expect a data-plane follow-up (e.g. `get_hex_range`) listen for `'data'` keyed
 *    by the same `correlationId` they got back from `request()`.
 *  - `hello()`'s protocol-version mismatch is a loud `console.error`, not a thrown exception —
 *    for wire schema v0 hard-failing startup over a version string felt premature (the sidecar
 *    binary is dev-built alongside this client right now); revisit once the pair actually ships
 *    independently versioned.
 */

import { type ChildProcessByStdio, spawn } from "node:child_process";
import { EventEmitter } from "node:events";
import type { Readable, Writable } from "node:stream";

import {
  type DecodedFrame,
  FrameDecoder,
  FrameKind,
  PROTOCOL_VERSION,
  encodeFrame,
  type HelloResult,
  type WireError,
  type WireResponse,
} from "./protocol.js";

/** Rejection type for a request that got back `{ok: false, error: ...}` from the sidecar. */
export class SidecarRequestError extends Error {
  readonly code: string;
  readonly offset?: number;

  constructor(error: WireError) {
    super(`bitvue-sidecar request failed [${error.code}]: ${error.message}`);
    this.name = "SidecarRequestError";
    this.code = error.code;
    this.offset = error.offset;
  }
}

/** Thrown when the sidecar process exits (or fails to spawn) while requests are still pending. */
export class SidecarExitedError extends Error {
  constructor(detail: string) {
    super(`bitvue-sidecar process exited: ${detail}`);
    this.name = "SidecarExitedError";
  }
}

interface PendingRequest {
  resolve: (result: unknown) => void;
  reject: (error: Error) => void;
}

export interface SidecarClientOptions {
  /** Extra argv passed to the sidecar binary. Defaults to none. */
  args?: string[];
  /** Working directory for the spawned process. Defaults to the current process's cwd. */
  cwd?: string;
  /** Client version string sent in the `hello` handshake. Defaults to this package's version. */
  clientVersion?: string;
}

type SidecarChildProcess = ChildProcessByStdio<Writable, Readable, Readable>;

/**
 * Events:
 *  - `'data'`  (correlationId: number, payload: Buffer)  — a `Data`-kind frame arrived.
 *  - `'event'` (correlationId: number, payload: Buffer)  — an `Event`-kind frame arrived
 *    (sidecar-initiated push; JSON-decode the payload yourself, shape is method-specific).
 *  - `'stderr'` (line: string) — one line of the child's stderr (also always logged via
 *    `console.error`; listen to this only if you need to capture it yourself too).
 *  - `'exit'` (code: number | null, signal: NodeJS.Signals | null) — child process exited.
 */
export class SidecarClient extends EventEmitter {
  private readonly child: SidecarChildProcess;
  private readonly decoder = new FrameDecoder();
  private readonly pending = new Map<number, PendingRequest>();
  private nextCorrelationId = 1;
  private exited = false;
  private stderrRemainder = "";

  constructor(binaryPath: string, options: SidecarClientOptions = {}) {
    super();
    this.child = spawn(binaryPath, options.args ?? [], {
      cwd: options.cwd,
      stdio: ["pipe", "pipe", "pipe"],
    }) as SidecarChildProcess;

    this.child.stdout.on("data", (chunk: Buffer) => this.onStdout(chunk));
    this.child.stderr.on("data", (chunk: Buffer) => this.onStderr(chunk));
    this.child.on("error", (err) => this.onChildGone(`spawn error: ${err.message}`));
    this.child.on("exit", (code, signal) => {
      this.exited = true;
      this.emit("exit", code, signal);
      this.onChildGone(`exited with code=${code} signal=${signal}`);
    });
  }

  /** stdout carries ONLY protocol frames — decode and route them, never treat as text. */
  private onStdout(chunk: Buffer): void {
    let frames: DecodedFrame[];
    try {
      frames = this.decoder.push(chunk);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      console.error(`[bitvue-sidecar] protocol framing error, dropping connection: ${message}`);
      this.onChildGone(`protocol framing error: ${message}`);
      return;
    }
    for (const frame of frames) {
      switch (frame.kind) {
        case FrameKind.Control:
          this.handleControlFrame(frame.correlationId, frame.payload);
          break;
        case FrameKind.Data:
          this.emit("data", frame.correlationId, frame.payload);
          break;
        case FrameKind.Event:
          this.emit("event", frame.correlationId, frame.payload);
          break;
      }
    }
  }

  private handleControlFrame(correlationId: number, payload: Buffer): void {
    let response: WireResponse;
    try {
      response = JSON.parse(payload.toString("utf8")) as WireResponse;
    } catch (err) {
      console.error(
        `[bitvue-sidecar] malformed control response (correlation_id=${correlationId}), dropping: ${
          err instanceof Error ? err.message : String(err)
        }`,
      );
      return;
    }
    const pending = this.pending.get(correlationId);
    if (!pending) {
      console.error(
        `[bitvue-sidecar] response for unknown/already-settled correlation_id=${correlationId}, ignoring`,
      );
      return;
    }
    this.pending.delete(correlationId);
    if (response.ok) {
      pending.resolve(response.result);
    } else {
      pending.reject(new SidecarRequestError(response.error ?? { code: "INTERNAL", message: "unknown error" }));
    }
  }

  /** stderr is diagnostics/logs only — forward line-by-line, never parsed as protocol. */
  private onStderr(chunk: Buffer): void {
    this.stderrRemainder += chunk.toString("utf8");
    const lines = this.stderrRemainder.split("\n");
    this.stderrRemainder = lines.pop() ?? "";
    for (const line of lines) {
      console.error(`[bitvue-sidecar] ${line}`);
      this.emit("stderr", line);
    }
  }

  private onChildGone(detail: string): void {
    if (this.pending.size === 0) return;
    const err = new SidecarExitedError(detail);
    for (const { reject } of this.pending.values()) reject(err);
    this.pending.clear();
  }

  /**
   * Sends a control-plane request and resolves with `result` once the matching response
   * arrives, or rejects with a `SidecarRequestError` (on `ok: false`) or `SidecarExitedError`
   * (if the process dies first).
   */
  request(method: string, params: unknown = {}): Promise<unknown> {
    if (this.exited) {
      return Promise.reject(new SidecarExitedError("request() called after process exit"));
    }
    const id = this.nextCorrelationId++;
    const body = Buffer.from(JSON.stringify({ id, method, params }), "utf8");
    const frame = encodeFrame(FrameKind.Control, id, body);

    return new Promise<unknown>((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.child.stdin.write(frame, (err) => {
        if (err) {
          this.pending.delete(id);
          reject(err);
        }
      });
    });
  }

  /** Convenience wrapper for the startup handshake described in the protocol spec. */
  async hello(clientVersion = "bitvue-desktop/0.0.0"): Promise<HelloResult> {
    const result = (await this.request("hello", { client_version: clientVersion })) as HelloResult;
    if (result.protocol_version !== PROTOCOL_VERSION) {
      console.error(
        `[bitvue-sidecar] protocol version mismatch: client expects ${PROTOCOL_VERSION}, sidecar reports ${result.protocol_version}`,
      );
    }
    return result;
  }

  /** Kills the child process. Any still-pending requests reject via the `exit` handler. */
  close(): void {
    if (this.exited) return;
    this.child.stdin.end();
    this.child.kill();
  }
}
