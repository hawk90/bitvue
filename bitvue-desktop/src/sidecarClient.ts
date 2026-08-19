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
 *  - Crash recovery (`options.restart`) restarts the *process*, not application state.
 *    `bitvue_engine::Core` lives entirely in-process in the sidecar with no persistence, so a
 *    crash loses whatever streams were open and whatever was selected — there is no command
 *    log to replay. Pending requests at crash time are rejected (`SidecarExitedError`), never
 *    silently retried, because a request might have partially mutated state before the crash
 *    and blind retry isn't safe to assume is idempotent. After a `'restarted'` event, it's the
 *    *caller's* job to re-establish whatever state it cares about (e.g. re-issue `open_stream`
 *    for whatever file was open) — this client doesn't track or replay that automatically.
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

/** See `SidecarClientOptions.restart` — process-only recovery, no state replay. */
export interface SidecarRestartOptions {
  /** Max consecutive restart attempts before giving up. Default 3. */
  maxAttempts?: number;
  /** Delay before each restart attempt, in ms. Default 500. Fixed, not exponential — see module doc. */
  backoffMs?: number;
}

export interface SidecarClientOptions {
  /** Extra argv passed to the sidecar binary. Defaults to none. */
  args?: string[];
  /** Working directory for the spawned process. Defaults to the current process's cwd. */
  cwd?: string;
  /** Client version string sent in the `hello` handshake. Defaults to this package's version. */
  clientVersion?: string;
  /**
   * Auto-restart the sidecar process if it exits unexpectedly (crash — not via `.close()`).
   * Omit to disable (default), matching prior behavior for existing callers/tests. See the
   * module doc's "Crash recovery" note for exactly what this does and doesn't restore.
   */
  restart?: SidecarRestartOptions;
}

type SidecarChildProcess = ChildProcessByStdio<Writable, Readable, Readable>;

/**
 * Events:
 *  - `'data'`  (correlationId: number, payload: Buffer)  — a `Data`-kind frame arrived.
 *  - `'event'` (correlationId: number, payload: Buffer)  — an `Event`-kind frame arrived
 *    (sidecar-initiated push; JSON-decode the payload yourself, shape is method-specific).
 *  - `'stderr'` (line: string) — one line of the child's stderr (also always logged via
 *    `console.error`; listen to this only if you need to capture it yourself too).
 *  - `'exit'` (code: number | null, signal: NodeJS.Signals | null) — child process exited
 *    (fires on every exit, including ones that trigger a restart).
 *  - `'restarting'` (attempt: number) — only with `options.restart`: about to respawn after an
 *    unexpected exit.
 *  - `'restarted'` (attempt: number) — respawn succeeded and the new process answered `hello()`.
 *  - `'restart_failed'` (attempts: number) — gave up after `maxAttempts`; the client is now
 *    permanently dead, same as without `options.restart`.
 */
export class SidecarClient extends EventEmitter {
  private child: SidecarChildProcess;
  private decoder = new FrameDecoder();
  private readonly pending = new Map<number, PendingRequest>();
  private nextCorrelationId = 1;
  private exited = false;
  private stderrRemainder = "";
  private readonly binaryPath: string;
  private readonly spawnArgs: string[];
  private readonly spawnCwd: string | undefined;
  private readonly restartConfig: Required<SidecarRestartOptions> | undefined;
  private restartAttempts = 0;
  /** Set by `close()` — distinguishes an intentional shutdown from a crash, so we know not to restart. */
  private manualClose = false;

  constructor(binaryPath: string, options: SidecarClientOptions = {}) {
    super();
    this.binaryPath = binaryPath;
    this.spawnArgs = options.args ?? [];
    this.spawnCwd = options.cwd;
    this.restartConfig = options.restart
      ? {
          maxAttempts: options.restart.maxAttempts ?? 3,
          backoffMs: options.restart.backoffMs ?? 500,
        }
      : undefined;
    this.child = this.spawnChild();
  }

  private spawnChild(): SidecarChildProcess {
    const child = spawn(this.binaryPath, this.spawnArgs, {
      cwd: this.spawnCwd,
      stdio: ["pipe", "pipe", "pipe"],
    }) as SidecarChildProcess;

    child.stdout.on("data", (chunk: Buffer) => this.onStdout(chunk));
    child.stderr.on("data", (chunk: Buffer) => this.onStderr(chunk));
    child.on("error", (err) => this.onChildGone(`spawn error: ${err.message}`));
    child.on("exit", (code, signal) => {
      this.exited = true;
      this.emit("exit", code, signal);
      this.onChildGone(`exited with code=${code} signal=${signal}`);
      this.maybeRestart(code, signal);
    });
    return child;
  }

  private maybeRestart(code: number | null, signal: NodeJS.Signals | null): void {
    if (this.manualClose || !this.restartConfig) return;
    if (this.restartAttempts >= this.restartConfig.maxAttempts) {
      console.error(
        `[bitvue-sidecar] gave up restarting after ${this.restartAttempts} attempt(s) ` +
          `(last exit: code=${code} signal=${signal})`,
      );
      this.emit("restart_failed", this.restartAttempts);
      return;
    }
    this.restartAttempts += 1;
    const attempt = this.restartAttempts;
    console.error(
      `[bitvue-sidecar] crashed (code=${code} signal=${signal}), restart attempt ` +
        `${attempt}/${this.restartConfig.maxAttempts} in ${this.restartConfig.backoffMs}ms...`,
    );
    this.emit("restarting", attempt);

    setTimeout(() => {
      if (this.manualClose) return; // closed while the backoff timer was pending
      this.decoder = new FrameDecoder(); // old buffer belongs to the dead process's byte stream
      this.child = this.spawnChild();
      this.exited = false;
      this.hello()
        .then(() => {
          this.restartAttempts = 0; // recovered — give the next crash (if any) a fresh budget
          this.emit("restarted", attempt);
        })
        .catch((err) => {
          // The new child's own 'exit' handler will trigger another maybeRestart if it also
          // dies; if it's alive but hello() itself failed for some other reason, we don't spin
          // retrying hello() specifically — that's an unexpected-enough state to just surface.
          console.error(`[bitvue-sidecar] restart attempt ${attempt}: post-restart hello() failed: ${err}`);
        });
    }, this.restartConfig.backoffMs);
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

  /**
   * Convenience wrapper for `get_hex_range` — the one command today whose response spans two
   * frames (a `Control` metadata response, then a `Data` frame with the raw bytes, both sharing
   * the same correlation id per the wire schema). `request()` alone only surfaces the `Control`
   * side; this method also listens for the matching `'data'` event so callers (like the Electron
   * main-process IPC handler) don't need to know about frame-kind internals to use a data-plane
   * command.
   */
  async getHexRange(params: {
    stream: string;
    offset: number;
    len: number;
  }): Promise<{ offset: number; len: number; bytes: Buffer }> {
    if (this.exited) {
      return Promise.reject(new SidecarExitedError("getHexRange() called after process exit"));
    }
    const id = this.nextCorrelationId++;
    const body = Buffer.from(JSON.stringify({ id, method: "get_hex_range", params }), "utf8");
    const frame = encodeFrame(FrameKind.Control, id, body);

    let onData: (correlationId: number, payload: Buffer) => void = () => {};
    const dataPromise = new Promise<Buffer>((resolve) => {
      onData = (correlationId, payload) => {
        if (correlationId !== id) return;
        this.off("data", onData);
        resolve(payload);
      };
      this.on("data", onData);
    });

    const metadataPromise = new Promise<{ offset: number; len: number }>((resolve, reject) => {
      this.pending.set(id, { resolve: resolve as (result: unknown) => void, reject });
      this.child.stdin.write(frame, (err) => {
        if (err) {
          this.pending.delete(id);
          reject(err);
        }
      });
    });

    try {
      const [metadata, bytes] = await Promise.all([metadataPromise, dataPromise]);
      return { offset: metadata.offset, len: metadata.len, bytes };
    } catch (err) {
      // Error path (e.g. NotFound/InvalidRange): no Data frame follows, so the listener would
      // otherwise leak — clean it up regardless of which promise rejected.
      this.off("data", onData);
      throw err;
    }
  }

  /**
   * Convenience wrapper for `get_decoded_frame_yuv` — second data-plane command, same two-frame
   * shape as `getHexRange` (`Control` metadata, then a `Data` frame of raw bytes). Here the Data
   * frame is the concatenated Y+U+V planes; callers slice `bytes` using `yLen`/`uLen`/`vLen`.
   */
  async getDecodedFrameYuv(params: {
    stream: string;
    frameIndex: number;
  }): Promise<{
    width: number;
    height: number;
    bitDepth: number;
    chromaSubsampling: "420" | "422" | "444";
    yStride: number;
    uStride: number;
    vStride: number;
    yLen: number;
    uLen: number;
    vLen: number;
    bytes: Buffer;
  }> {
    if (this.exited) {
      return Promise.reject(
        new SidecarExitedError("getDecodedFrameYuv() called after process exit"),
      );
    }
    const id = this.nextCorrelationId++;
    const body = Buffer.from(
      JSON.stringify({
        id,
        method: "get_decoded_frame_yuv",
        params: { stream: params.stream, frame_index: params.frameIndex },
      }),
      "utf8",
    );
    const frame = encodeFrame(FrameKind.Control, id, body);

    let onData: (correlationId: number, payload: Buffer) => void = () => {};
    const dataPromise = new Promise<Buffer>((resolve) => {
      onData = (correlationId, payload) => {
        if (correlationId !== id) return;
        this.off("data", onData);
        resolve(payload);
      };
      this.on("data", onData);
    });

    type DecodedFrameYuvMetadata = {
      width: number;
      height: number;
      bit_depth: number;
      chroma_subsampling: "420" | "422" | "444";
      y_stride: number;
      u_stride: number;
      v_stride: number;
      y_len: number;
      u_len: number;
      v_len: number;
    };
    const metadataPromise = new Promise<DecodedFrameYuvMetadata>((resolve, reject) => {
      this.pending.set(id, { resolve: resolve as (result: unknown) => void, reject });
      this.child.stdin.write(frame, (err) => {
        if (err) {
          this.pending.delete(id);
          reject(err);
        }
      });
    });

    try {
      const [metadata, bytes] = await Promise.all([metadataPromise, dataPromise]);
      return {
        width: metadata.width,
        height: metadata.height,
        bitDepth: metadata.bit_depth,
        chromaSubsampling: metadata.chroma_subsampling,
        yStride: metadata.y_stride,
        uStride: metadata.u_stride,
        vStride: metadata.v_stride,
        yLen: metadata.y_len,
        uLen: metadata.u_len,
        vLen: metadata.v_len,
        bytes,
      };
    } catch (err) {
      this.off("data", onData);
      throw err;
    }
  }

  /**
   * Convenience wrapper for `get_debug_yuv_frame` — third data-plane command, same two-frame
   * shape as `getDecodedFrameYuv`. Requires a debug YUV session to already be loaded via
   * `request("load_debug_yuv", ...)`; the sidecar returns a `NotFound` wire error otherwise.
   */
  async getDebugYuvFrame(params: {
    frameIndex: number;
    mode: "decoded" | "reference" | "diff" | "amplified";
    amplify?: number;
  }): Promise<{
    width: number;
    height: number;
    bitDepth: number;
    chromaSubsampling: "420" | "422" | "444";
    yStride: number;
    uStride: number;
    vStride: number;
    yLen: number;
    uLen: number;
    vLen: number;
    bytes: Buffer;
  }> {
    if (this.exited) {
      return Promise.reject(new SidecarExitedError("getDebugYuvFrame() called after process exit"));
    }
    const id = this.nextCorrelationId++;
    const body = Buffer.from(
      JSON.stringify({
        id,
        method: "get_debug_yuv_frame",
        params: { frame_index: params.frameIndex, mode: params.mode, amplify: params.amplify },
      }),
      "utf8",
    );
    const frame = encodeFrame(FrameKind.Control, id, body);

    let onData: (correlationId: number, payload: Buffer) => void = () => {};
    const dataPromise = new Promise<Buffer>((resolve) => {
      onData = (correlationId, payload) => {
        if (correlationId !== id) return;
        this.off("data", onData);
        resolve(payload);
      };
      this.on("data", onData);
    });

    type DebugYuvFrameMetadata = {
      width: number;
      height: number;
      bit_depth: number;
      chroma_subsampling: "420" | "422" | "444";
      y_stride: number;
      u_stride: number;
      v_stride: number;
      y_len: number;
      u_len: number;
      v_len: number;
    };
    const metadataPromise = new Promise<DebugYuvFrameMetadata>((resolve, reject) => {
      this.pending.set(id, { resolve: resolve as (result: unknown) => void, reject });
      this.child.stdin.write(frame, (err) => {
        if (err) {
          this.pending.delete(id);
          reject(err);
        }
      });
    });

    try {
      const [metadata, bytes] = await Promise.all([metadataPromise, dataPromise]);
      return {
        width: metadata.width,
        height: metadata.height,
        bitDepth: metadata.bit_depth,
        chromaSubsampling: metadata.chroma_subsampling,
        yStride: metadata.y_stride,
        uStride: metadata.u_stride,
        vStride: metadata.v_stride,
        yLen: metadata.y_len,
        uLen: metadata.u_len,
        vLen: metadata.v_len,
        bytes,
      };
    } catch (err) {
      this.off("data", onData);
      throw err;
    }
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

  /** PID of the currently-running sidecar process (changes across a restart). For diagnostics/tests. */
  get pid(): number | undefined {
    return this.child.pid;
  }

  /**
   * Kills the child process for good — sets a flag so a subsequent `exit` is treated as
   * intentional and does NOT trigger `options.restart`. Any still-pending requests reject via
   * the `exit` handler.
   */
  close(): void {
    this.manualClose = true;
    if (this.exited) return;
    this.child.stdin.end();
    this.child.kill();
  }
}
