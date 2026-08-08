/**
 * Real crash-recovery test: force-kills the actual compiled `bitvue-sidecar` process (SIGKILL,
 * not `.close()` — a real unexpected exit, not an intentional shutdown) and verifies
 * `SidecarClient`'s `options.restart` actually respawns it and the new process is usable. This
 * is the sidecar crash-recovery policy flagged unresolved since the sidecar bridge decision in
 * `docs/DEVELOPMENT_PHASES.md` — see that doc and `sidecarClient.ts`'s module doc for what this
 * does and (deliberately) doesn't restore (process only, not `bitvue_core::Core`'s in-memory
 * state).
 */

import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, beforeAll, describe, expect, it } from "vitest";

import { SidecarClient } from "../src/sidecarClient.js";

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, "..", "..");
const defaultBinaryName = process.platform === "win32" ? "bitvue-sidecar.exe" : "bitvue-sidecar";
const binaryPath = process.env.BITVUE_SIDECAR_BIN ?? path.join(repoRoot, "target", "debug", defaultBinaryName);

let client: SidecarClient | undefined;

beforeAll(() => {
  if (!existsSync(binaryPath)) {
    throw new Error(
      `bitvue-sidecar binary not found at ${binaryPath}.\n` +
        `Build it first: cargo build -p bitvue-sidecar (from the repo root that contains crates/bitvue-sidecar).`,
    );
  }
});

afterEach(() => {
  client?.close();
  client = undefined;
});

function waitForEvent<T extends unknown[]>(emitter: SidecarClient, event: string, timeoutMs = 5000): Promise<T> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(`timed out waiting for '${event}'`)), timeoutMs);
    emitter.once(event, (...args: T) => {
      clearTimeout(timer);
      resolve(args);
    });
  });
}

describe("SidecarClient crash recovery (real process, real SIGKILL)", () => {
  it("respawns after an unexpected exit and the new process answers requests", async () => {
    client = new SidecarClient(binaryPath, { restart: { maxAttempts: 3, backoffMs: 100 } });
    await client.hello("restart-test/0.0.1");
    const originalPid = client.pid;
    expect(originalPid).toBeGreaterThan(0);

    const restartedPromise = waitForEvent<[number]>(client, "restarted");
    process.kill(originalPid!, "SIGKILL"); // a real crash, not client.close()
    const [attempt] = await restartedPromise;
    expect(attempt).toBe(1);

    // New OS process — different pid — but the client object is the same and usable again.
    expect(client.pid).toBeGreaterThan(0);
    expect(client.pid).not.toBe(originalPid);

    const result = await client.hello("restart-test-post-recovery/0.0.1");
    expect(result).toEqual({ protocol_version: "0.1.0", capabilities: [] });
  });

  it("rejects pending requests at crash time instead of silently retrying them", async () => {
    client = new SidecarClient(binaryPath, { restart: { maxAttempts: 3, backoffMs: 100 } });
    await client.hello("restart-test/0.0.1");
    const pid = client.pid!;

    // Fire a request, then kill before the sidecar can possibly have replied.
    const pending = client.request("open_stream", { stream: "A", path: "/irrelevant" });
    process.kill(pid, "SIGKILL");

    // Rejected, not hung and not silently re-sent to the restarted process — the caller must
    // decide whether to retry, since the request might have partially applied before the crash.
    await expect(pending).rejects.toThrow(/exited/i);
  });

  it("gives up after maxAttempts if the process keeps crashing immediately", async () => {
    // A path that spawns successfully as a *process* (any executable) but isn't bitvue-sidecar,
    // so every 'exit' is immediate and restart-eligible without ever answering hello() —
    // exercises the "give up" path deterministically rather than needing a real repeated crash.
    const bogusBinary = process.platform === "win32" ? "cmd.exe" : "true";
    const bogusArgs = process.platform === "win32" ? ["/c", "exit 1"] : [];

    client = new SidecarClient(bogusBinary, {
      args: bogusArgs,
      restart: { maxAttempts: 2, backoffMs: 20 },
    });

    const failedPromise = waitForEvent<[number]>(client, "restart_failed", 3000);
    const [attempts] = await failedPromise;
    expect(attempts).toBe(2);
  });
});
