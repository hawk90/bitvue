---
name: optimize
description: Profile and optimize Bitvue performance - Rust parsing/decoding/overlay hot paths (criterion benches, flamegraphs) and the React renderer in the Electron shell. Use for performance regressions, slow frame navigation, or before/after benchmark comparisons.
allowed-tools: Read, Grep, Glob, Edit, Bash
---

# Optimize (Bitvue)

Measure first, change one thing, re-measure on the same bench. Never quote a number you did not just measure.

## 1. Benchmarks (criterion 0.5)

| Bench | Command | Covers |
|---|---|---|
| `bitreader` | `cargo bench -p bitvue-benchmarks --bench bitreader` | bit reading |
| `frame_parsing` | `cargo bench -p bitvue-benchmarks --bench frame_parsing` | IVF header, AV1 OBU iterator, PSNR |
| `magic_bytes` | `cargo bench -p bitvue-benchmarks --bench magic_bytes` | magic-byte matching (container detection) |
| `export` | `cargo bench -p bitvue-benchmarks --bench export` | frame/metrics CSV + JSON export |
| `overlay_extraction` | `cargo bench -p bitvue-av1-codec --bench overlay_extraction` | AV1 QP/MV/partition extraction |

Before/after comparison:

```bash
cargo bench -p bitvue-benchmarks --bench bitreader -- --save-baseline before
# ...apply change...
cargo bench -p bitvue-benchmarks --bench bitreader -- --baseline before
```

Reports land in `target/criterion/`. Add a bench (in `crates/bitvue-benchmarks/benches/` + `[[bench]]` entry, `harness = false`) for any hot path you optimize that has none.

## 2. CPU profiling (Rust)

- Profile build: `--profile release-with-debug` (workspace `Cargo.toml`; release + symbols, not stripped).
- `cargo flamegraph` / `samply` are **not installed** by default — `cargo install flamegraph` or `cargo install samply` first, or use macOS Instruments.

```bash
cargo build --profile release-with-debug -p bitvue-cli
samply record target/release-with-debug/bitvue decode test_data/av1_test.ivf --av1 --stats
# or: cargo flamegraph --profile release-with-debug -p bitvue-cli -- decode test_data/av1_test.ivf --av1 --stats
```

The CLI's `info/frames/analyze/validate` are AV1-only; `decode` covers AV1/HEVC/AVC/VP9/AVS3/JPEG XS/VC-3 (H.264/HEVC/VP9 decode need the `ffmpeg` feature). To profile what the app actually does, profile `bitvue-sidecar` (the Electron main process spawns it; it is synchronous, no tokio).

## 3. Renderer profiling (Electron + React)

- Renderer = `frontend/` (Vite, React 18). The custom native menu (`bitvue-desktop/electron/nativeMenu.ts`) has no DevTools entry, so attach remotely: `cargo build -p bitvue-sidecar && cd bitvue-desktop && npm run build:electron && npx electron . --remote-debugging-port=9222`, then open `chrome://inspect` in Chrome → Performance / Memory tabs.
- For HMR while iterating: `cd frontend && npm run dev`, then launch Electron with `BITVUE_FRONTEND_URL=http://localhost:5173`.
- Watch for: long tasks on frame step, repeated renders of filmstrip/overlay components, large `ArrayBuffer`s retained after stream close (YUV frames, thumbnails).

## 4. Known hotspots

| Area | Path |
|---|---|
| AV1 OBU / bit reading | `crates/bitvue-av1-codec/src/{obu.rs,bitreader.rs,leb128.rs}` |
| AV1 overlay extraction | `crates/bitvue-av1-codec/src/overlay_extraction/` (`qp_extractor.rs`, `mv_extractor.rs`, `partition.rs`, `cache.rs`) |
| YUV→RGB / decode SIMD | `crates/bitvue-decode/src/yuv.rs` (`yuv_to_rgb`), `src/strategy/{avx2,neon,scalar}.rs`, runtime dispatch in `strategy/registry.rs` |
| Metrics SIMD | `crates/bitvue-metrics/src/simd.rs` (feature `parallel` enables rayon) |
| Engine caches | `crates/bitvue-engine/src/{byte_cache.rs,stream_state.rs}` (`lru`) |
| Filmstrip | `frontend/components/VirtualizedFilmstrip.tsx` (custom virtualization, `memo`) |
| YUV rendering | `frontend/utils/yuv/`, WebGPU path `frontend/utils/gpu/` |
| Frame stats | `frontend/workers/frameStatsWorker.ts` (spawned from `contexts/FrameDataContext.tsx`) |
| Frontend cache | `frontend/utils/lruCache.ts` (`LRUCache`, `FrameDataCache`) |

## 5. Pattern catalogue

Only use what is already a dependency. Rust: `rayon`, `lru`, `parking_lot`, `memmap2`, `std::arch` (no `std::simd`). Frontend runtime deps are only React 18, `react-resizable-panels`, `@vscode/codicons` — no react-window, react-query, use-debounce.

### Rust

```rust
// Reuse one buffer across iterations instead of allocating per frame
let mut buf = Vec::with_capacity(max_size);
for f in frames {
    buf.clear();
    buf.extend_from_slice(f.data());
    process(&buf);
}
```

- `Vec::with_capacity` / `HashMap::with_capacity` when size is known.
- `Cow<'_, str>` for conditionally-owned strings; avoid `to_owned()` in log/error paths.
- Return `&Arc<T>` instead of cloning when the caller does not need ownership.
- Take a lock once per batch, not per item (`parking_lot::Mutex`).
- Cache parsed/extracted results keyed by frame with `lru::LruCache` (see engine caches) rather than re-parsing.
- Data-parallel per-frame work: `rayon` `par_iter` (precedent: `bitvue-cli/src/commands/decode.rs`).
- Large files: `memmap2` instead of reading whole file into a `Vec`.
- SIMD: `std::arch` intrinsics behind `is_x86_feature_detected!` / `is_aarch64_feature_detected!` with a scalar fallback — follow the `strategy/` pattern, and keep a test comparing SIMD vs scalar output.
- Bit reader: read whole bytes/words where possible instead of bit-by-bit loops; `#[inline]` tiny accessors.
- AV1 tiles within a frame are independently parseable, so tile-level parallelism is possible; frames are not (reference state carries across frames).
- `tracing` macros format lazily; avoid `format!`/`to_string()` in arguments on hot paths.
- When tuning a cache, measure hit/miss rate on a real navigation pattern before changing capacity.

### React (renderer)

- `useMemo` / `useCallback` for derived data and handlers passed to memoized children.
- `React.memo` on per-frame items (thumbnails, overlay cells) with stable props.
- Virtualize long lists — extend `VirtualizedFilmstrip.tsx`, don't add a library.
- `lazy()` + `Suspense` for dialogs/heavy panels (precedent: `App.tsx`).
- Move heavy computation off the main thread into a Worker (precedent: `workers/frameStatsWorker.ts`).
- Debounce slider/scrub input with a `setTimeout`-based hook; don't request a sidecar frame per pixel of drag.
- Reuse `LRUCache` from `utils/lruCache.ts` for thumbnails/decoded frames; evict on stream close.
- Batch sidecar calls (`get_frames_chunk`, `get_thumbnails`) rather than one request per frame.
- Prefetch adjacent frames/analysis after a seek so stepping ±1 hits the cache.

## 6. Performance targets (goals, not measurements)

| Operation | Goal |
|---|---|
| Parse 1080p I-frame | < 10 ms |
| Parse 1080p P-frame | < 8 ms |
| Parse 4K I-frame | < 40 ms |
| Extract QP / MV grid (per frame) | < 50 ms |
| Generate thumbnail | < 100 ms |
| Index 10K frames | < 5 s |
| Memory, 1 h stream | < 500 MB |
| Max heap per frame | < 10 MB |
| Frame navigation (renderer) | < 50 ms |
| Overlay render | < 16 ms (60 fps) |
| Window ready after launch | < 3 s |
| Renderer bundle (gzipped) | < 5 MB (low priority: loaded from disk) |

## 7. Finish

- Re-run the same bench with `--baseline`; report the measured delta.
- Run `cargo test --workspace --exclude abseil --lib --tests` (and frontend vitest if TS changed) — see the `test-bitvue` skill.
