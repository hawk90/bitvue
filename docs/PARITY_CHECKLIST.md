# Bitvue VQA Parity Checklist

Phase 12 tracking for VQ Analyzer feature parity (V14 spec §7.3): validation strategy, test sources, and
regression gating. **Feature/status items no longer live here.** Every former checklist row is now an item in
`docs/specs/features.yaml`, which is the only status source of truth. Legacy IDs (L1-*, IA-*, OV-*, CMP-*, CTX-*,
PERF-*, EVB-*, DIV-*, INT-*, EDGE-*, PNL-*) are kept in each item's `legacy_ids`.

See also: `docs/specs/features.yaml` (status source of truth) · `VQA_PARITY_SPEC_V3.md` (backend/codec spec) ·
`COMPETITOR_FEATURE_MATRIX.md` (per-product matrix) · `UX_PARITY_MATRIX.md` (UI/UX contracts, overlay colour
scale, shortcuts) · `DEVELOPMENT_PHASES.md` (roadmap).

## Status summary (features.yaml, 2026-10-05)

Computed with PyYAML over `items[]` in `docs/specs/features.yaml`: total by `status`, then by `priority` × `status`.

377 items — done 103 · partial 162 · todo 106 · dropped 6. By priority: P0 33 (24 done / 8 partial / 1 todo) ·
P1 117 (47/62/8) · P2 146 (26/71/49) · P3 81 (6/21/48, 6 dropped). The single biggest blocker is `INFRA-001`
(desktop sidecar/indexer only wired for AV1/IVF).
Dated fix/verification notes from the old Layer tables: `docs/history/parity-checklist-log.md`.

## Ground rules

- Code wins over docs. Before you mark anything done, grep for the sidecar command
  (`crates/bitvue-sidecar/src/request_dispatch.rs`), the mounted UI (`frontend/App.tsx`) or the CLI subcommand.
- `src-tauri` was deleted on 2026-08-08 (Tauri→Electron). A status that rests only on Tauri-era code
  (`src-tauri/src/commands/*.rs`, frontend `@tauri-apps/api` `invoke()` calls) shows that the design was done, not
  that the feature ships.
- Codec scope (`INFRA-001`; true on 2026-10-05): the desktop indexing and overlay pipeline (`index_stream`, `get_frame_analysis`) handles
  IVF/AV1 only. HEVC/AVC/VP9 parsing and overlay extraction exist in their crates and are exercised through the
  CLI, but they are not wired into the desktop app.

## Parity validation strategy & test sources

### Reference test-bitstream sources

| Codec | Source | URL / note |
|---|---|---|
| HEVC | JCT-VC HM official test set | MPEG/ITU-T FTP |
| VVC | VTM official test set | https://vcgit.hhi.fraunhofer.de/jvet/VVCSoftware_VTM |
| AV1 | AOM test vectors | https://storage.googleapis.com/aom-test-data/ |
| VP9 | Chrome/WebM test vectors | https://chromium.googlesource.com/webm/vp9-test-vectors |
| AVC | JM/x264 test set | self-generate recommended |
| MPEG-2 | MPEG official | self-generate recommended |
| AVS3 | AVS official | http://www.avs.org.cn/ |

Self-generated sequences (ffmpeg), for codecs with no public vector:
```bash
ffmpeg -i input.mp4 -c:v libx265 -x265-params "ctu=64:qp=28" output_hevc.mkv   # HEVC
ffmpeg -i input.mp4 -c:v libaom-av1 -cpu-used 4 output_av1.mkv                  # AV1
ffmpeg -i input.mp4 -c:v libvpx-vp9 -b:v 2M output_vp9.webm                     # VP9
ffmpeg -i input.mp4 -c:v libx264 -profile:v high output_avc.mp4                 # AVC
```

### Verification tiers

| Tier | Goal | Method | Tolerance |
|---|---|---|---|
| 1. Parsing accuracy | Syntax values match the reference 100% | Parse the same file with HM/VTM/dav1d, dump QP/MV/partitions to JSON, diff against Bitvue | exact |
| 2. Overlay visual accuracy | Overlay colour/position matches VQ Analyzer | Screenshot the same frame in both tools, pixel-compare | ±2 RGB, ±1px |
| 3. Value accuracy | PSNR/SSIM/QP match the reference | Compute in Debug YUV mode, compare against the ffmpeg `psnr`/`ssim` filters | ±0.01 dB PSNR, ±0.0001 SSIM |
| 4. UX behaviour | Click/keyboard/zoom match the original | Checklist-based manual test | — |

Planned automation per tier (names from the original plan): Tier 1 `cargo test --test syntax_parity -- --test-threads=1`,
Tier 2 `scripts/parity_screenshot_compare.py`, Tier 3 manual/scripted, Tier 4 Playwright E2E. Status: `QA-009`
(on 2026-10-05 none of these existed and Playwright was not a dependency). UX
behaviour is verified today with vitest component tests, plus real-Electron screenshot runs driven by the
`BITVUE_ELECTRON_SCREENSHOT*` env hooks in `bitvue-desktop/electron/main.ts` (CLICK_TAB, CLICK_SELECTOR, KEY,
MOUSEMOVE, CTRL_WHEEL, CONTEXT_MENU, WINDOW_SIZE, OPEN_DEPENDENT). Known harness limit: CLICK_SELECTOR sends
`mousedown` + `.click()` but no real `mouseup`/`mouseenter` sequence, so you can't screenshot-verify drag
interactions. Cover those with component tests instead.

### Pre-release per-codec checks and release gates

> Feature/status items for this section live in `docs/specs/features.yaml` (area: qa/perf/infra/decode; source
> "PARITY_CHECKLIST.md §Pre-release parity checklist" / "§Release-gate checklist").

Checks that already have a feature ID point to that item: NAL/OBU/frame-header match → the L1 codec items;
partition/QP/MV/CDEF/LR/film grain → the OV overlay items. The checks with no ID are separate QA items: HEVC
SPS/PPS/slice header, SAO, deblocking, Stats tab; VVC dual-tree/LMCS/ALF/CCLM (blocked on vvdec); VP9 segment map
and probability tables. The release gates are also items: no unwrap in data paths, typed errors + never-blank
panels, decoder-failure degradation, stable IDs, deterministic Hex→Syntax, overlay toggle under 50 ms, release-notes
template, lockcheck in CI.

`VERSIONING_POLICY.md` from the `_import_v14` pack was skipped as low-value: it is generic semver and doesn't
map to Bitvue's 0.x versioning.

## Feature layers (index only)

> Feature/status items for this section live in `docs/specs/features.yaml`.

| Former layer | Scope | Legacy IDs |
|---|---|---|
| 1 Codec parsing & frame extraction | CLI-level AV1/HEVC/AVC/VP9 extraction, stats, MD5, resilience | L1-* |
| 2 Visualization modes (IA) | coding flow, timeline, syntax tree, selection info, hex, status | IA-01..06 |
| 3 Overlay modes | QP/MV/partition/CBF/transform/prediction/CDEF/LR/film grain/super-res | OV-01..10 |
| 4 Keyboard shortcuts | navigation, file, fullscreen, selection, channel, F-key modes, zoom | — (grouped items) |
| 5 Export & CLI | YUV/Y4M dump, MD5, PSNR, batch, JSON/CSV/MD export | — |
| 6 Dual-stream compare & metrics | A/B sync, split, diff, first-diff, BD-rate, VMAF, APV, AV3, CABAC | CMP-01..10 |
| 7 UX interaction contracts | context menus, display/decode order, perf budget, evidence bundle | CTX-01/02, PERF-01, EVB-01 |
| 8 Data/viz completeness | timestamps, diagnostics, SEI/HDR, complexity, ref depth, HRD, concealment, PSNR series | DIV-01..08 |
| 9 Per-panel interaction contracts | Player/Timeline/Hex/Metrics/RefGraph/Trees + deferred Tier 3 candidates | INT-01..06 |
| 10 Robustness & edge cases | panel error isolation, request cap, PTS badge, coverage legend, index gating, selection z-order | EDGE-01..06 |
| 11 Panel visual parity (Timeline/Filmstrip vs VQ Analyzer) | resizable filmstrip, reference arrows, Frame Sizes/HRD/Enhanced/B-Pyramid fixes | PNL-01..13 |

Out of scope, by design: broadcast QC (live TS conformance TR 101290, SCTE-35, CableLabs), captions
(EIA-608/708, DVB), and audio codec/loudness analysis. See `VQA_PARITY_SPEC_V3.md` §1.5.

## Recurring patterns (read before diagnosing a gap)

- **Engine built but never wired.** A real backend module exists, sometimes a large and well-tested one, but it is
  constructed only in tests. Examples: `MetadataInspector`, reorder detection in `insight_feed.rs`, `HrdModel`,
  `qp_heatmap` coverage, `IndexReadyGate`, `overlay_stack.rs`, `PerfBudgets`, `lockcheck.rs`, and the
  HEVC/AVC/VP9 overlay extractors. Before you assume any engine-crate module is live, check for a production
  caller (sidecar command → bridge → mounted component).
- **Live engine, broken joints.** This is different from dead code. DIV-02's event pipeline (`event::Diagnostic`)
  was real but broke at three joints: Debug-string wire serialization, per-frame parse failures that were silently
  swallowed, and the frontend discarding the `indexStream` return value. Tell "genuinely dead" apart from "wire or
  frontend joint broken" before you scope the work.
- **Designed affordance ring around a working core.** In the per-panel contracts (Layer 9), click-select works,
  but most of the documented hover tooltips, drag-range, semantic zoom and rich context menus were never built.
  Here the frontend interaction code is what's missing, not the engine.
- **Dead Tauri-era UI.** Examples true on 2026-10-05 (see `UI-009`, `UX-006`, `EXP-002` for current status). Some components still call `@tauri-apps/api` `invoke()` and are unmounted:
  `QualityMetricsPanel`, `ReferenceGraphPanel`. Some keyboard handlers dispatch CustomEvents that nothing listens
  to (Ctrl+S, Ctrl+Z, Ctrl+C, Escape clear-selection). If a file or a key binding exists, that doesn't mean the
  feature works.
- **Static screenshots miss interaction bugs.** The PNL-02..05 reference-arrow bugs and the passive-wheel
  `preventDefault` error only showed up through live interaction or the real Electron console. jsdom
  `fireEvent.wheel` doesn't reproduce passive listeners. Screenshots at default state can't catch selection-dependent
  rendering either.
- **Unit-conversion traps in codec data.** AV1 `ref_frame_idx` holds DPB slot numbers, not frame indices (PNL-03).
  `IvfHeader::framerate_num`/`framerate_den` are swapped relative to their on-disk meaning (fixed at the use sites,
  field names left as they are). Validate against a real fixture: frame N and frame N+40 must not show identical
  references.

## Regression suite

- CI-enforced since 2026-07-31: the `.github/workflows/ci.yml` job `parity-regression` runs
  `scripts/parity_check.sh --local` on PRs that touch Rust. The CI run is the ground truth.
- Local runs: `./scripts/run_regression_suite.sh` (does `cargo test --workspace --exclude abseil --lib --tests`)
  and `./scripts/parity_check.sh --local`. Never use `cargo test -p <crate> --lib` alone, because it skips the
  `tests/` integration binaries.
- `parity_check.sh` sections: 1 AV1 local fixture, 2 auto-detect, 3 error handling, 4 HEVC, 5 AVC, 6 VP9,
  7 optional AOM public vectors (download, full run only).
- CLI parity tests: `crates/bitvue-cli/tests/parity_test.rs` (AV1 decode/stats/md5/max-frames/autodetect, plus
  empty/garbage/wrong-codec resilience for HEVC/AVC/VP9).

## Fixture files

`test_data/` is generated with ffmpeg testsrc (synthetic, 320×240):

| File | Codec | Notes |
|---|---|---|
| `test_data/av1_test.ivf` | AV1 IVF | 250 frames, IPPP (no reordering), PTS 0..249 sequential, no HDR/timing info |
| `test_data/hevc_test.hevc` | HEVC Annex B | 7.5 KB |
| `test_data/avc_test.h264` | AVC Annex B | 7.4 KB, VUI `nal_hrd_parameters_present_flag=0` |
| `test_data/vp9_test.ivf` | VP9 IVF | 11.4 KB |

These fixtures have known coverage gaps: no B-frame/reordered stream, no non-Ok PTS, no HDR metadata, no HRD
params, and no second distinct AV1 encode for A/B diff (CMP-04 used a `manual_offset` misalignment instead). Features that depend on
these (forward-ref arrows, PTS badge Warn/Bad, SEI/HRD panels, A/B diff on different encodes) are verified by
unit tests only.
