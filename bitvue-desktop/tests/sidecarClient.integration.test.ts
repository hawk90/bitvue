/**
 * Real cross-language interop test: spawns the actual compiled `bitvue-sidecar` Rust binary
 * (not a mock) and talks to it over stdio through `SidecarClient`. This is the whole point of
 * `bitvue-desktop` — proving the TypeScript half of the bridge works against the real Rust half,
 * not just that it type-checks against a hand-written fixture.
 *
 * Binary location: by default this resolves `<repoRoot>/target/debug/bitvue-sidecar` (i.e.
 * `bitvue-desktop/../target/debug/bitvue-sidecar` — the standard `cargo build -p bitvue-sidecar`
 * output location once `crates/bitvue-protocol`/`crates/bitvue-sidecar` and `bitvue-desktop` live
 * side by side in the same repo checkout). Override with `BITVUE_SIDECAR_BIN=/path/to/binary` if
 * your build output lives elsewhere (e.g. a different checkout/worktree than this package).
 *
 * If the binary can't be found, this test fails loudly with instructions rather than silently
 * skipping — a skipped interop test would defeat the purpose of having it.
 */

import { existsSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, beforeAll, describe, expect, it } from "vitest";

import { SidecarClient } from "../src/sidecarClient.js";

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, "..", ".."); // bitvue-desktop/tests -> bitvue-desktop -> repo root
const defaultBinaryName = process.platform === "win32" ? "bitvue-sidecar.exe" : "bitvue-sidecar";
const binaryPath = process.env.BITVUE_SIDECAR_BIN ?? path.join(repoRoot, "target", "debug", defaultBinaryName);

let client: SidecarClient | undefined;
let tempDir: string | undefined;

beforeAll(() => {
  if (!existsSync(binaryPath)) {
    throw new Error(
      `bitvue-sidecar binary not found at ${binaryPath}.\n` +
        `Build it first: cargo build -p bitvue-sidecar (from the repo root that contains crates/bitvue-sidecar).\n` +
        `Or point BITVUE_SIDECAR_BIN at an already-built binary from another checkout.`,
    );
  }
});

afterEach(() => {
  client?.close();
  client = undefined;
  if (tempDir) {
    rmSync(tempDir, { recursive: true, force: true });
    tempDir = undefined;
  }
});

describe("SidecarClient <-> real bitvue-sidecar binary", () => {
  it("completes the hello handshake with the real protocol version", async () => {
    client = new SidecarClient(binaryPath);
    const result = await client.hello("bitvue-desktop-integration-test/0.0.1");

    expect(result).toEqual({ protocol_version: "0.1.0", capabilities: [] });
  });

  it("opens a stream and receives a real ModelUpdated event from bitvue-engine", async () => {
    client = new SidecarClient(binaryPath);
    await client.hello("bitvue-desktop-integration-test/0.0.1");

    tempDir = mkdtempSync(path.join(tmpdir(), "bitvue-desktop-sidecar-test-"));
    const filePath = path.join(tempDir, "fake-stream.ivf");
    // Matches crates/bitvue-sidecar/src/main.rs's own `open_stream_success_emits_model_updated`
    // test fixture: Core::handle_command only needs the file to exist, not to be a valid
    // bitstream, to emit ModelUpdated.
    writeFileSync(filePath, "not a real bitstream, just needs to exist");

    const result = (await client.request("open_stream", { stream: "A", path: filePath })) as {
      events: Array<{ type: string; stream?: string }>;
    };

    expect(result.events).toHaveLength(1);
    expect(result.events[0].type).toBe("ModelUpdated");
    expect(result.events[0].stream).toBe("A");
  });

  it("rejects still-pending requests if the sidecar process exits unexpectedly", async () => {
    client = new SidecarClient(binaryPath);
    await client.hello("bitvue-desktop-integration-test/0.0.1");

    // Fire a request but kill the process before the sidecar can reply, proving pending
    // promises don't hang forever on an unexpected exit.
    const pending = client.request("open_stream", { stream: "A", path: "/irrelevant" });
    client.close();

    await expect(pending).rejects.toThrow(/exited/i);
  });
});
