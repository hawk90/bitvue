---
name: test-bitvue
description: Run and write Bitvue tests - Rust workspace tests (unit, integration, proptest), regression/parity scripts, frontend and bitvue-desktop vitest suites, cargo llvm-cov coverage. Use when adding tests, checking a change didn't break anything, or investigating coverage.
allowed-tools: Read, Grep, Glob, Edit, Write, Bash
---

# Test Bitvue

## 1. Run

| Scope | Command |
|---|---|
| Rust, default check | `cargo test --workspace --exclude abseil --lib --tests` |
| One crate (unit + integration) | `cargo test -p bitvue-hevc --lib --tests` |
| One test | `cargo test -p bitvue-av1-codec --test resilient_parser_test` / `cargo test -p <crate> <name_filter> -- --nocapture` |
| Regression gate | `scripts/run_regression_suite.sh` (`--fast` skips doc-tests; `--parity-full` adds downloads) |
| CLI parity only | `scripts/parity_check.sh --local` (fixtures in `test_data/` only) |
| Frontend | `cd frontend && npm run test:run` (watch: `npm test`; root `npm run test:run` delegates here) |
| Desktop (Electron/protocol) | `cd bitvue-desktop && npm run test:run` (`test:unit` = protocol only) |

- `-- --nocapture` shows test output; `--release` speeds up slow fixture-heavy tests (not a substitute for benches).
- **Never** use `cargo test -p <crate> --lib` alone as a "nothing broke" check — it skips every `tests/` binary.
- Avoid `--all-features`: it enables `ffmpeg` (bitvue-decode H.264/HEVC/VP9) and `vvdec`, which need system libs. Test those features explicitly only when touching them: `cargo test -p bitvue-decode --features ffmpeg`.
- `bitvue-desktop/tests/sidecarClient.*.test.ts` spawn the real binary: `cargo build -p bitvue-sidecar` first (default `target/debug/bitvue-sidecar`, override `BITVUE_SIDECAR_BIN`). Desktop tests are not in CI.
- CI (`.github/workflows/ci.yml`): fmt, clippy, per-crate `cargo test -p <crate> --no-fail-fast` matrix, parity-regression, coverage, frontend fmt/lint/test (`npm run test:run`).

## 2. Layout

**Rust**
- Inline `#[cfg(test)] mod tests` in source files.
- `crates/bitvue-engine/src/*_test.rs` (e.g. `selection_test.rs`, `mv_overlay_test.rs`, `error_test.rs`).
- `crates/<crate>/tests/` integration binaries exist for: `bitvue-av1-codec`, `bitvue-avc`, `bitvue-hevc`, `bitvue-vp9`, `bitvue-vvc`, `bitvue-mpeg2-codec`, `bitvue-formats`, `bitvue-decode`, `bitvue-metrics`, `bitvue-engine` (`critical_edge_cases_test.rs`, `integration_tests.rs`), `bitvue-cli` (`parity_test.rs`), `bitvue-codecs-parser`, `bitvue-sidecar` (`subprocess_smoke.rs`). No `tests/` dir yet for `bitvue-avs3`, `bitvue-jpegxs`, `bitvue-vc3` (inline tests only).
- Property tests (proptest, dev-dep of `bitvue-av1-codec` only): `crates/bitvue-av1-codec/tests/{bitreader,ivf,leb128}_prop_tests.rs`; regressions persist in `*.proptest-regressions` (commit them).
- Fuzz: `fuzz/fuzz_targets/{av1_decode,ivf_parser,leb128,obu_parser,fuzz_target_1}.rs`. Currently broken: `fuzz/Cargo.toml` depends on nonexistent `../crates/bitvue-av1` (should be `bitvue-av1-codec`).

**Fixtures** — `test_data/{av1_test.ivf, avc_test.h264, hevc_test.hevc, vp9_test.ivf}` (flat). `samples/foreman_*` are app demo samples, not test fixtures.

**Frontend** (`frontend/`, vitest 1.6 + jsdom)
- Tests only under `frontend/tests/{components,hooks,utils,contexts,services,types,constants}/` and `tests/App.test.tsx` — `vitest.config.ts` includes `tests/**` only; a test placed next to a component will not run.
- Setup `frontend/test/setup.ts` (mocks `@tauri-apps/*`, `react-resizable-panels`).
- Helpers `frontend/test/test-utils.tsx`: `render` (wraps all providers), `renderWithoutProviders`, `userEvent`, `mockFrames`.
- Fixtures `frontend/test/fixtures/{analysisData,frames,videos}.ts` (`mockFrames`, `generateMockFrames(n)`, `mockStreamInfo`, `generateMockAnalysisData`, ...).
- `@` aliases the `frontend/` root.

**Desktop** — `bitvue-desktop/tests/{protocol.test.ts, sidecarClient.integration.test.ts, sidecarClient.restart.test.ts}`, vitest node env, 20 s timeout.

No Playwright / e2e / jest in this repo.

## 3. Writing tests

### Rust unit / integration

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncated_input_is_error_not_panic() {
        assert!(parse_ivf_header(&[0u8; 4]).is_err());
    }
}
```

Integration test loading a fixture (resolve from the crate dir, as `bitvue-av1-codec/tests/mv_extraction_test.rs` does):

```rust
let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../test_data/av1_test.ivf");
let data = std::fs::read(&path).expect("fixture");
let (header, frames) = bitvue_av1_codec::parse_ivf_frames(&data).unwrap();
assert!(!frames.is_empty());
```

Integration scenarios worth covering per codec/container:
- Full-file parse: frame count equals `IvfHeader.frame_count` (a shortfall means silent truncation)
- Same stream through different containers (IVF vs MP4/MKV sample extraction) yields the same frames
- Overlay extraction on a real fixture frame > 0 (exercises ref-state threading)
- Truncated/corrupted copy of a fixture: resilient parse recovers the intact frames and reports diagnostics

Rules: parsers must return `Err` (e.g. `BitvueError::UnexpectedEof`, `InsufficientData { needed, available }`) on malformed input, never panic — add a truncated/corrupt-input case with every parser change. Use `*_resilient` variants' tests (`resilient_parser_test.rs`) as the model for recovery behaviour.

### Property test (av1-codec)

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn decode_never_panics(data in prop::collection::vec(any::<u8>(), 0..64)) {
        let _ = bitvue_av1_codec::decode_uleb128(&data);
    }
}
```

To use proptest in another crate, add it to that crate's `[dev-dependencies]` first.

### Frontend (vitest + Testing Library)

Mock the bridge, not Electron/IPC. Partial mock keeps pure helpers real:

```ts
import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@/test/test-utils";
import { mockFrames } from "@/test/fixtures/frames"; // or from "@/test/test-utils"

vi.mock("@/services/electronBridgeService", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@/services/electronBridgeService")>()),
  indexStream: vi.fn().mockResolvedValue([]),
}));

describe("FrameSizesView", () => {
  it("renders one bar per frame", () => { /* render(<... frames={mockFrames} />); expect(...) */ });
});
```

- Also commonly mocked: `@/contexts/{SelectionContext,StreamDataContext,FrameDataContext,FileStateContext}`.
- Hooks: `renderHook` from `@testing-library/react` (example: `tests/hooks/useKeyboardNavigation.test.ts`).
- Async: `await screen.findBy...` / `waitFor`; restore with `vi.restoreAllMocks()` in `afterEach`.

## 4. Coverage

- Rust: `cargo llvm-cov` (install: `cargo install cargo-llvm-cov`, needs `llvm-tools-preview`).
  - `cargo llvm-cov -p bitvue-av1-codec --html` → `target/llvm-cov/html/`
  - CI runs it per crate with `--codecov` and uploads to Codecov (PRs only).
- Frontend: `npm run test:coverage` uses the v8 provider, but `@vitest/coverage-v8` is **not installed** — `npm i -D @vitest/coverage-v8@^1.6` in `frontend/` first.

### Coverage targets (goals)

| Area | Target |
|---|---|
| Rust core parsing (bitreader, OBU/NAL, headers) | 90%+ |
| Rust error paths (`BitvueError` returns on malformed input) | 100% of variants exercised |
| Rust overlay extraction | 85%+ |
| Rust public API (engine, formats, CLI) | 85%+ |
| Frontend components | 80%+ |
| Frontend hooks | 85%+ |
| Frontend utils | 75%+ |
| Frontend services (bridge wrappers) | 70%+ |

## 5. Before reporting done

1. `cargo test --workspace --exclude abseil --lib --tests` (or `scripts/run_regression_suite.sh --fast`).
2. `cd frontend && npm run test:run` if any TS changed; desktop tests if `bitvue-desktop/` or `bitvue-protocol` changed.
3. Rust edits: `cargo fmt --all -- --check` and `cargo clippy --workspace --lib --exclude abseil -- -D warnings` (same as CI).
