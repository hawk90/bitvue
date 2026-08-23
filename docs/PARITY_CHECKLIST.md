# Bitvue VQA Parity Checklist

Phase 12 tracking for VQ Analyzer feature parity (V14 spec §7.3).

See also: `VQA_PARITY_SPEC_V3.md` (backend/codec parity spec), `COMPETITOR_FEATURE_MATRIX.md` (per-product
feature matrix backing Layer 6 below), `UX_PARITY_MATRIX.md` (UI/UX interaction parity), `DEVELOPMENT_PHASES.md`
(Phase 0-12 implementation roadmap).

## ⚠️ 2026-08-08: `src-tauri` 삭제됨 (Tauri→Electron 전환 완료)

이 파일의 기존 `[x]`/`[-]` 표시 중 근거가 `src-tauri/src/commands/*.rs` 파일 인용인 것들은 **그 코드가 지금 삭제되고
없음**을 뜻함 — 기능 설계/로직이 검증됐다는 기록으로는 유효하지만, 지금 코드베이스(Electron/`bitvue-sidecar`)에
그 기능이 실제로 존재한다는 뜻은 아니니 재확인 없이 신뢰하지 말 것. 자세한 배경은 `DEVELOPMENT_PHASES.md`
Phase 0 바로 위 경고 섹션 참조. `bitvue-sidecar`에 실제로 이식된 건 현재 9개 커맨드뿐(`open_stream`/
`select_frame`/`select_unit`/`select_syntax`/`select_bit_range`/`select_spatial_block`/`close_stream`/
`get_hex_range`/`cancel_request`) — 이 목록에 없는 항목은 재이식 전이라고 가정할 것.

## How to use

- `[x]` = implemented and tested
- `[-]` = partial / in progress
- `[ ]` = not started

Run the regression suite to validate:
```
./scripts/run_regression_suite.sh
./scripts/parity_check.sh --local
```

---

## Parity Validation Strategy & Test Sources

Merged from `VQA_PARITY_SPEC_V3.md` §7 (2026-07-31 doc-family split) — verification methodology belongs with
the tracking doc, not the feature spec. Renamed "Layer 1-4" (verification tiers) to "Tier 1-4" below to avoid
clashing with this doc's own "Layer 1-6" feature-category tables.

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

**Self-generated sequences** (ffmpeg, when no public vector exists):
```bash
# HEVC (x265)
ffmpeg -i input.mp4 -c:v libx265 -x265-params "ctu=64:qp=28" output_hevc.mkv
# AV1 (libaom)
ffmpeg -i input.mp4 -c:v libaom-av1 -cpu-used 4 output_av1.mkv
# VP9
ffmpeg -i input.mp4 -c:v libvpx-vp9 -b:v 2M output_vp9.webm
# AVC
ffmpeg -i input.mp4 -c:v libx264 -profile:v high output_avc.mp4
```

### Verification tiers

| Tier | Goal | Method | Tolerance | Automation |
|---|---|---|---|---|
| 1. Parsing accuracy | Syntax values match source 100% | Parse same file with public reference decoder (HM/VTM/dav1d), extract key syntax (QP/MV/partitions) → JSON, diff vs Bitvue output | 0% (parsing must be exact) | `cargo test --test syntax_parity -- --test-threads=1` |
| 2. Overlay visual accuracy | Overlay color/position pixel-near matches VQ Analyzer | Screenshot same frame in both tools, pixel-compare | ±2 RGB, ±1px boundary | `scripts/parity_screenshot_compare.py` (future) |
| 3. Value accuracy | PSNR/SSIM/QP etc. match source | Compute in Debug YUV mode, compare vs `ffmpeg psnr` filter | ±0.01dB (PSNR), ±0.0001 (SSIM) | manual / scripted |
| 4. UX behavior | Click/keyboard/zoom behavior matches original | Checklist-based manual test | — | Playwright E2E (automated) |

### Pre-release parity checklist (by codec)

Where an item already has a Layer 1-6 ID above, that ID is the source of truth — this table only adds
granular release-gate items not yet tracked by an ID.

| Codec | Check | Existing ID (if any) |
|---|---|---|
| HEVC | NAL unit count/type match | L1-HEVC-01 |
| HEVC | SPS/PPS parameter values match | — (new) |
| HEVC | Slice header values match | — (new) |
| HEVC | CU/TU/PU partition tree structure match | OV-03 |
| HEVC | QP Map heatmap color match | OV-01 |
| HEVC | MV vector magnitude/direction match (PU-level) | OV-02 |
| HEVC | SAO type/parameter match | — (new, see `COMPETITOR_FEATURE_MATRIX.md` §1 HEVC "SAO" row — not tracked) |
| HEVC | Deblocking boundary position match | — (new, see `COMPETITOR_FEATURE_MATRIX.md` §1 HEVC "Loop Filter" row — not tracked) |
| HEVC | Stats tab figures match | — (new) |
| VVC | Dual Tree luma/chroma boundary match | — (blocked on vvdec connection, spec §3.1) |
| VVC | LMCS APS parameter match | — (blocked, same) |
| VVC | ALF parameter match | — (blocked, same) |
| VVC | CCLM prediction parameter display | — (blocked, same) |
| AV1 | OBU type/size match | L1-AV1-01 |
| AV1 | Superblock partition structure match | OV-03 |
| AV1 | CDEF direction/strength match | OV-07 |
| AV1 | Loop Restoration type match | OV-08 |
| AV1 | Film Grain parameter display match | OV-09 |
| VP9 | Frame header parameter match | L1-VP9-01 |
| VP9 | Segment map match | — (new) |
| VP9 | Probability table initial-value match | — (new) |

### Release-gate checklist (from `_import_v14` product-readiness pack, 2026-07-31)

Concrete, non-generic items only — cross-checked against code where claimed.

| Category | Gate | Status/note |
|---|---|---|
| Error handling | No `unwrap()`/`expect()` in data paths (parse/decode/IO) | Verified via grep: only 2 occurrences in `src-tauri/src` (`lib.rs`, `commands/recent_files.rs`) — largely already compliant, not a gap |
| Error handling | Typed error carries `category, severity, user_message, debug_details, recovery_action`; failed viz renders placeholder + banner + Retry/Compute/Load CTA (never blank panel) | Matches `crates/bitvue-engine/src/{diagnostics,error,app_error}.rs` `DiagnosticSeverity`/`DiagnosticCategory` design — not independently verified for 100% UI coverage |
| Error handling | Decoder init fail → Player shows checkerboard, other panels stay usable; frame decode fail → ghost previous frame + jump-to-nearest-decodable | Not verified against current Player component — flag for QA pass, not confirmed done |
| Data integrity | IDs stable (`FrameKey`/`UnitKey`/`SyntaxNodeId`) across a session once full index built | Matches `crates/bitvue-engine/src/selection.rs` key types — structurally present |
| Data integrity | Hex→Syntax reverse mapping deterministic (see Tri-sync rule in `DEVELOPMENT_PHASES.md` Architecture appendix) | `crates/bitvue-engine/src/evidence.rs` implements the 4-stage chain — logic present, UI-level determinism unverified |
| Performance | Overlay toggle < 50ms | Not measured/benchmarked — add to Phase 10/12 perf test matrix |
| Release process | Every release: features added / known limitations / perf metrics / compatibility notes | Not currently enforced — add as PR/release template checklist item (Phase 12) |
| QA gate (V12_LOCKCHECK_SPEC) | Validate workspace grid + LOD/cache keys + overlay specs + MCP resource schemas + degradation rules against locked contracts, output pass/fail report | **Already implemented as code**, not just a checklist: `crates/bitvue-engine/src/lockcheck.rs` (`LockCheckResult`/`LockCheckItem`/`LockCheckCategory` incl. `Workspace/LodCache/PlayerOverlays/McpResources/Degradation/CacheCaps`) — verify it's wired into `scripts/parity_check.sh` or CI; not confirmed in this pass |

**Skipped as low-value**: `VERSIONING_POLICY.md` (generic semver description, "v12 = product-ready baseline" doesn't map
to Bitvue's actual 0.x versioning — no actionable Bitvue-specific content).

---

## Layer 1: Codec Parsing & Frame Extraction

| ID | Codec | Feature | Status | Test |
|----|-------|---------|--------|------|
| L1-AV1-01 | AV1 | IVF frame extraction | [x] | `parity_test::av1_decode_runs_without_error` |
| L1-AV1-02 | AV1 | Frame stats (type, size, pts, offset) | [x] | `parity_test::av1_stats_flag_runs_without_error` |
| L1-AV1-03 | AV1 | Per-frame MD5 | [x] | `parity_test::av1_md5_flag_runs_without_error` |
| L1-AV1-04 | AV1 | max-frames limit | [x] | `parity_test::av1_max_frames_limit_respected` |
| L1-AV1-05 | AV1 | Auto-detect IVF | [x] | `parity_test::av1_autodetect_without_force_codec` |
| L1-AV1-06 | AV1 | Stream stats | [x] | `parity_test::av1_stream_stats_flag_runs_without_error` |
| L1-HEVC-01 | HEVC | Annex B NAL extraction | [x] | `parity_check.sh §4` |
| L1-HEVC-02 | HEVC | Frame stats (IDR/CRA/TRAIL) | [x] | `parity_check.sh §4 fixture` |
| L1-HEVC-03 | HEVC | --stream-stats NAL breakdown | [x] | `parity_check.sh §4 --stream-stats` |
| L1-HEVC-04 | HEVC | Empty/garbage resilience | [x] | `parity_test::hevc_empty/garbage_*` |
| L1-AVC-01 | AVC | Annex B NAL extraction | [x] | `parity_check.sh §5` |
| L1-AVC-02 | AVC | Frame stats (IDR/I/P/B) | [x] | `parity_check.sh §5 fixture` |
| L1-AVC-03 | AVC | Empty/garbage resilience | [x] | `parity_test::avc_empty/garbage_*` |
| L1-VP9-01 | VP9 | IVF VP90 frame extraction | [x] | `parity_check.sh §6 fixture` |
| L1-VP9-02 | VP9 | Auto-detect VP90 FourCC | [x] | `parity_test::vp9_autodetect_from_ivf_fourcc_does_not_panic` |
| L1-VP9-03 | VP9 | KEY/INTER frame types | [x] | `parity_check.sh §6 KEY frame check` |
| L1-VP9-04 | VP9 | Empty/garbage resilience | [x] | `parity_test::vp9_empty/garbage_*` |

**Fixtures**: `test_data/hevc_test.hevc`, `test_data/avc_test.h264`, `test_data/vp9_test.ivf` (generated via ffmpeg testsrc).

---

## Layer 2: Visualization Modes (IA parity)

Mapped from `FULL_PARITY_MATRIX_JSON` P0/P1 items.

| ID | Item | Severity | Status | Notes |
|----|------|----------|--------|-------|
| IA-01 | Main panel: coding flow grid (CTB/CU/PU hierarchy) | P0 | [x] | `coding-flow` mode (F2) — CodingFlowRenderer.tsx |
| IA-02 | Timeline view (frame sizes, QP, filmstrip) | P0 | [x] | FrameSizesView (bars + QP axis + bitrate curve) + Timeline.tsx + Filmstrip — wired via FilmstripPanel |
| IA-03 | Syntax tree panel per codec | P0 | [x] | SyntaxPanel — per-codec tabs (Phase 8) |
| IA-04 | Selection info (CTB addr, MV, QP, pred mode) | P0 | [x] | SelectionInfoPanel — block click shows details |
| IA-05 | Hex view (raw bytes, offset, ASCII) | P0 | [x] | HexViewTab.tsx + `get_frame_hex_data` (codec-agnostic via UnitNode offset/size) |
| IA-06 | Status panel (errors, warnings, stream info) | P1 | [x] | StatusBar + error count in AppLayout |

---

## Layer 3: Overlay Modes

| ID | Mode | Codecs | Status | Frontend Key |
|----|------|--------|--------|-------------|
| OV-01 | QP Heatmap | AV1, HEVC, AVC, VP9 | [x] | F2 |
| OV-02 | MV Field | AV1, HEVC, AVC | [x] | F1 |
| OV-03 | Partition grid | AV1, HEVC | [x] | F3 |
| OV-04 | CBF Luma | AV1, HEVC | [x] | F4 (`transform` mode via TransformRenderer) |
| OV-05 | Transform type | AV1 | [x] | F4 (`transform` / TransformRenderer.tsx) |
| OV-06 | Prediction mode | AV1, HEVC, AVC | [x] | F3 (`prediction` / PredictionRenderer.tsx) |
| OV-07 | CDEF | AV1 | [x] | `cdef-filter` / Av1CdefRenderer.tsx |
| OV-08 | Loop restoration | AV1 | [x] | `loop-restoration` / Av1LoopRestorationRenderer.tsx |
| OV-09 | Film grain | AV1 | [x] | `film-grain` / Av1FilmGrainRenderer.tsx |
| OV-10 | Super-res | AV1 | [x] | `super-res` / Av1SuperResRenderer.tsx |

---

## Layer 4: Keyboard Shortcut Parity

All shortcuts verified against VQA reference (Phase 11).

| Shortcut | Action | Status |
|----------|--------|--------|
| ←/→ | Previous/next frame | [x] |
| Space | Next frame | [x] |
| Home/End | First/last frame | [x] |
| Ctrl+←/→ | Previous/next I-frame | [x] |
| [/] | Previous/next I-frame (VQA parity) | [x] |
| Ctrl+O | Open file | [x] |
| Ctrl+W | Close file | [x] |
| Ctrl+E | Export | [x] |
| Ctrl+S | Save frame PNG | [x] |
| Ctrl+G / Ctrl+F | Go to frame | [x] |
| Ctrl+R | Reload file | [x] |
| F | Toggle fullscreen | [x] |
| F11 | OS fullscreen | [x] |
| Escape | Exit fullscreen / clear selection | [x] |
| ? | Show shortcut help | [x] |
| Ctrl+Z | Undo selection | [x] |
| Ctrl+C | Copy block info | [x] |
| Y/U/V | Channel isolation | [x] |
| F1–F10 | Visualization mode switch | [x] |
| Ctrl+F1–F6 | Toggle info overlay | [x] |
| `0` | Fit to window / reset zoom (100%) | [x] |
| `+` / `=` | Zoom in | [x] |
| `-` | Zoom out | [x] |
| `Ctrl+Click` (block) | Multi-select blocks (VVC Dynamic Selection Info) | [ ] |

**Mouse-only interactions** (wheel zoom, click+drag pan, double-click reset, thumbnail click/right-click/scroll)
are out of scope for this keyboard-only table — see `UX_PARITY_MATRIX.md` §3 "Per-panel interaction contract"
and §5 "Zoom/pan policy per panel" for the general Player-surface pan/zoom/click contract these map to.
Migrated from `VQA_PARITY_SPEC_V3.md` §6 (2026-07-31 doc-family split); source also listed right-click-extract
on thumbnails, which is separately tracked in that spec's §4.1 Stream View table (not a keyboard shortcut).

---

## Layer 5: Export & CLI Parity

| Feature | Status | Command |
|---------|--------|---------|
| Frame export (PNG) | [x] | `bitvue export` |
| YUV dump (-o out.yuv) | [x] | `bitvue decode -o` |
| Y4M dump (--y4m) | [x] | `bitvue decode --y4m` |
| Per-frame MD5 (--md5) | [x] | `bitvue decode --md5` |
| PSNR (--psnr --reference) | [x] | `bitvue decode --psnr` |
| Batch analysis | [x] | `bitvue batch` |
| JSON export | [x] | `bitvue export --json` |

---

## Layer 6: Dual-Stream Compare & Quality Metrics (NEW — 2026-07-31, VQA_PARITY_SPEC_V3 §4.9/§1.5)

Discovered via competitor research (ViCueSoft VQ Probe, Elecard StreamEye, Interra VEGA) — not covered by
Layers 1-5, which is why this doc previously read "All P0/P1 complete." Full per-feature source breakdown:
`COMPETITOR_FEATURE_MATRIX.md` §3 (metrics) and §4 (compare). Priority rationale: `VQA_PARITY_SPEC_V3.md` §1.5.

| ID | Feature | Severity | Status | Notes |
|----|---------|----------|--------|-------|
| CMP-01 | Stream A/B independent load + sync playback | P0 | [x] | **2026-08-19 실구현** (Phase 7.5 MVP): `crates/bitvue-sidecar/src/compare.rs` — `create_compare_workspace`/`get_aligned_frame`/`set_sync_mode`/`set_manual_offset`/`reset_offset` 전부 sidecar 커맨드로 실제 배선 완료(이전엔 `create_compare_workspace`가 "존재하지 않음"을 검증하는 테스트 안에만 있었음), `bitvue_engine::compare::CompareWorkspace`(PTS 기반 `AlignmentEngine` 재사용, 이미 완성돼 있었으나 한 번도 호출된 적 없었음)를 실제로 구성. `CompareContext.tsx`를 죽은 `@tauri-apps/api` `invoke()`에서 real bridge로 전면 재작성, `App.tsx`에 실제 마운트(기존에 있던 "Open dependent bitstream..." 메뉴 항목+`handleOpenDependentFile`은 이미 연결돼 있었으나 백엔드가 없어 무동작이었음). Side-by-side 뷰 실동작 확인(스크린샷 검증) |
| CMP-02 | Side-by-side / Split (H/V) view | P0 | [x] | Side-by-side는 위 CMP-01과 함께 실동작 확인됨(2026-08-19). **Split(H/V) 와이프 뷰도 2026-08-19 실구현 완료** — `frontend/components/CompareWorkspace/SplitView.tsx` 신규: 단일 `<canvas>`에 스트림 A/B를 각각 오프스크린 캔버스로 렌더링(`StreamPlayer.tsx`와 동일하게 `getDecodedFrameYuv`+`bridgeYuvToFrame`, `VideoCanvas.tsx`와 동일하게 `YUVRenderer`/`putImageData` 픽셀 경로 재사용, 신규 픽셀 파이프라인 없음) 후 분할 위치 기준 clip-rect로 합성. 드래그 가능한 디바이더(Timeline.tsx 스크러버와 동일한 `mousedown`→`window.addEventListener("mousemove"/"mouseup")` 패턴), V/H 방향 전환, 키보드 방향키 nudge(접근성). `CompareWorkspace.tsx`에 Side-by-Side/Split 뷰모드 토글 추가(diff 토글/컬럼은 side-by-side 전용으로 유지). 실제 Electron 스크린샷 2장(V/H 각 1장, `BITVUE_ELECTRON_SCREENSHOT_CLICK_TAB`)으로 분할선+양쪽 스트림 렌더링 육안 확인, 콘솔 에러 0건 |
| CMP-03 | Subtraction / Temperature (diff heatmap) view | P0 | [x] | **2026-08-19 실구현**: `get_diff_frame` sidecar 커맨드 신규 — `bitvue_engine::diff_heatmap::DiffHeatmapData::from_luma_planes`(이미 완성돼 있었으나 미배선)를 실제 두 스트림의 디코드된 luma plane에 호출. `DiffOverlay.tsx`가 갖고 있던 자체 client-side 16x16 픽셀 diff 재구현을 제거하고 이 실제 엔진 호출로 교체. "Subtraction"(Signed)/"Temperature"(Abs) 두 실제 모드만 노출, 미구현 psnr/ssim/metric은 fake 컨트롤이라 제외. 동일 스트림 diff=all-zero 회귀 테스트로 stride 처리 정확성까지 검증 |
| CMP-04 | Find First Difference (stream vs stream) | P1 | [x] | **2026-08-19 실구현**: `find_first_diff_frame_ab` sidecar 커맨드 신규(디버그 YUV용 단일 스트림 `find_first_diff_frame`과는 별개 wire method) — stream A를 0..total_frames 순회하며 `resolve_diff_heatmap`(CMP-03의 `get_diff_frame`과 동일 로직 재사용, `DiffMode::Abs`)의 heatmap `max_value > 0.0`을 "진짜 픽셀 차이 있음"의 정확한 판정 기준으로 사용(모든 항이 non-negative라 평균이 0이려면 2x2 블록 전체가 동일해야 함 — 근사 아닌 정확한 동치). `NoAlignedFrame` 갭은 스킵, 해상도 불일치/디코드 실패 등 워크스페이스 전역 조건은 스캔 즉시 중단. `index_stream`/`get_thumbnails`와 동일하게 cancel_flag 협조적 체크(프레임당 1회). CompareWorkspace.tsx에 "Find First Diff" 버튼 추가, 찾으면 두 스트림을 그 프레임으로 점프(현재 sync mode 존중). 동일스트림 무diff(250프레임 전부 스캔)+manual_offset으로 고의 misalign시 실제 diff 즉시 검출(frame 0) 두 케이스 모두 실제 fixture로 검증(두 번째 케이스는 두 번째 AV1 fixture가 첫 번째와 완전히 동일한 파일임을 md5로 우연히 발견해서 manual_offset 방식으로 재설계) |
| CMP-05 | RD-curve + BD-rate calculation | P0 | [-] | Real CLI implementation as of 2026-08-11 (`bitvue bd-rate --reference r.ivf --anchor a1..a4 --test t1..t4`, `bitvue-cli`): builds RD curves from real measured PSNR/SSIM (not the old QP-heuristic `psnr_from_qp`) + real bitrate (file size / actual decoded duration), computes BD-rate/BD-quality via a from-scratch VCEG-M33 cubic-polynomial-fit implementation (`bitvue_metrics::bd_rate`) -- the pre-migration Tauri `calculate_bd_rate` (deleted `e7194cc`, no test coverage) had a real unit mismatch (exponentiated a quality-domain integral as if it were a rate-domain delta). Verified against real SVT-AV1 encodes (2 presets × 4 CRF levels) end-to-end. Along the way found and fixed a real, separate bug: `IvfHeader::framerate_num`/`framerate_den` are swapped relative to their on-disk meaning, so file-duration-from-framerate was off by `(fps)²` in both this new code and `bitvue-cli info`'s existing bitrate estimate (now fixed in both; the `IvfHeader` field names themselves were left as-is -- renaming them is a larger, separate blast-radius change, see code comments at both fix sites). RDCurvesPanel.tsx itself remains on dead `@tauri-apps/api` `invoke()` calls, unmounted (Phase 7.5-adjacent, not touched) -- this item is CLI-only, no UI |
| CMP-06 | VMAF scoring (pooled + per-frame) | P0 | [ ] | Blocked (2026-08-10): the `vmaf` Cargo feature (`crates/bitvue-metrics`, `dep:libvmaf-rs`) fails to build on this dev machine -- `libvmaf-rs` 0.5.2 (latest, no newer version) transitively depends on `ffmpeg-next ^6.0` → `ffmpeg-sys-next 6.1.0`, which expects FFmpeg's ≤6.x C API (`libavcodec/avfft.h`, removed upstream in FFmpeg 7.0). System has FFmpeg 8.1.2 (Homebrew, already latest) -- not a missing-dependency problem, `brew install ffmpeg` does not fix it. Real fix needs either a system-wide FFmpeg downgrade (risks breaking other tools depending on 8.1.2, declined for now) or an isolated build-only FFmpeg@6/7 via a separate prefix + `PKG_CONFIG_PATH` override (not attempted). libvmaf itself (the CLI/lib, not the Rust binding) IS installed and current (3.2.0) -- the blocker is purely `libvmaf-rs`'s own ffmpeg-next dependency chain. |
| CMP-07 | VMAF sub-scores (ADM2, VIF, motion2) | P2 | [ ] | Nice-to-have beyond VQ Probe baseline |
| CMP-08 | APV codec support | P2 | [ ] | ViCueSoft added this in v7.7/7.8; niche/professional format |
| CMP-09 | AV3/AVM naming reconciliation | P1 | [x] | **Not the same codec, and Bitvue's "AV3" isn't a real spec implementation of anything.** `bitvue-av3-codec`'s `ObuType` (SequenceHeader/TemporalDelimiter/OverheadInfo/FrameHeader/Frame/TileGroup/Metadata/RedundantFrameHeader/TileList/Padding, values 1-9+15 -- an AV1-shaped list with one invented type, "OverheadInfo") doesn't match the real AV2/AVM spec's OBU taxonomy (`OBU_LEADING_TIP`/`OBU_REGULAR_TIP`, `OBU_CLOSED_LOOP_KEY`/`OBU_OPEN_LOOP_KEY`, `OBU_RAS_FRAME`/`OBU_BRIDGE_FRAME`, `OBU_FILM_GRAIN`, `OBU_QUANTIZATION_MATRIX`, `OBU_ATLAS_SEGMENT`, `OBU_LAYER_CONFIGURATION_RECORD`, etc. -- verified 2026-08-20 against AOM's official syntax browser, `av2.aomedia.org/v13-public/syntax_browser.html`). The crate is also unwired (no `bitvue-sidecar`/`bitvue-cli` dependency), self-tested only against synthetically constructed bytes (no real AVM/AV2 sample data anywhere in the repo), and frontend "AV3" references are cosmetic-only (an error-message string, a hardcoded feature-name list in `CodingFlowView.tsx`) with no real parse path behind them -- same "Monster Pack"-style disconnected-scaffolding pattern as `command_chain.rs`/`event_observer.rs`/etc. (axis 8, 2026-08-20). Crate not deleted as part of this check -- flagged for a separate decision. |
| CMP-10 | CABAC range/state visualization (HEVC/AVC) | P1 | [ ] | Both VQ Analyzer and VEGA expose this; Bitvue coverage unverified |

**Explicitly out of scope** (broadcast QC territory, not a bitstream analyzer concern): live TS conformance
(TR 101290, SCTE-35, CableLabs), captions (EIA-608/708, DVB), audio codec/loudness analysis. See spec §1.5.

---

## Layer 7: UX Interaction Contracts (NEW — 2026-07-31, `UX_PARITY_MATRIX.md` §6/§7/§9)

Discovered via the V14-pack UX mining pass — real gaps, not codec/compare features so they don't fit Layer 6.
Referenced from `DEVELOPMENT_PHASES.md` Phase 7.6 the same way Layer 6 is referenced from Phase 7.5.

| ID | Feature | Severity | Status | Notes |
|----|---------|----------|--------|-------|
| CTX-01 | Context-menu system (Player/HexView/StreamView scopes, guard+disabled-reason policy) | P1 | [x] | Contract: `UX_PARITY_MATRIX.md` §6 (3 scopes, 7 items, 3 guards: `always`/`has_selection`/`has_byte_range`). **Verified real 2026-08-11** (found stale during a UX audit -- this row still said `[ ]` after the feature had shipped): `bitvue_engine::export::context_menu` implements the full guard-evaluated catalog for **5** scopes (Player/HexView/StreamView/Timeline/DiagnosticsPanel -- 2 more than the original §6 spec, added `f3e9e81`/`35955fc`/`e545bba`/`7b8bfc9`), backed by real Rust unit tests (`export/export_test.rs::context_menu_tests`, `tests/export.rs`) and sidecar IPC tests (`bitvue-sidecar/src/context_menu.rs`). Frontend: generic `ContextMenu.tsx` (guard-evaluated items rendered with disabled+tooltip state) is wired via real `onContextMenu` handlers + `getContextMenuItems()` calls in all 5 consumer panels (`YuvViewerPanel`/`VideoCanvas` for Player, `HexViewTab`, `StreamTreePanel`, `Timeline`, `DiagnosticsPanel`) -- confirmed by direct grep, not just presence of the component. **Guard/disabled-reason policy re-verified 2026-08-23**: disabled-reason tooltip and guard purity both genuinely hold (`ContextMenu.tsx:62-63`, `context_menu.rs`'s guard fns take `&GuardEvalContext` read-only); disabled items are un-clickable at the DOM level (no bypass). The previously-noted Player-scope defect (guard inputs hardcoded `false`) is **stale as of 2026-08-23**: `YuvViewerPanel/index.tsx`'s `handleCanvasContextMenu` now computes both `hasSelection`/`hasByteRange` from real `selection` state (fixed incidentally during INT-01's click→spatialBlock work, not tracked as its own fix at the time) — all 5 scopes now correctly wire the guards they use. HexView scope gained 2 new items (`Copy.Offset`/`Copy.BitRange`, both `has_byte_range`-guarded, `context_menu.rs`); Player's pre-existing `Copy.Selection` stub is now wired to copy the selected block's position/size (`YuvViewerPanel/index.tsx`) — see INT-01/INT-03 rows. |
| CTX-02 | Display-order vs decode-order separation (Timeline/Player/Diagnostics/Metrics/Compare use display_idx primary, decode_idx internal-only) | P0 | [-] | **Data half fixed 2026-08-23**: `frontend/contexts/FileStateContext.tsx`'s `applyDisplayOrder` now joins the decode-order `frames` array against `get_timeline`'s real PTS-sorted `display_idx` (by `pts`, not `decode_idx` — respects `frame_identity`'s "decode_idx is internal only" contract, no wire/Rust change needed since `get_timeline` already carried real `pts` per entry). Populates `FrameInfo.display_order`/`.coding_order`, which ~8 already-built consumers were silently rendering as "N/A" for (`DetailsPanel`/`StatisticsTab`/`ThumbnailsView`/`VirtualizedThumbnailsView`/`DebugPanel`/`FrameSyntaxTab`/`dataExport` CSV+JSON) — verified end-to-end with a real Electron screenshot (`DetailsPanel` now shows real `Display Order: 0`/`Coding Order: 0`, filmstrip `D:`/`C:` badges populate). Ambiguous PTS (missing/duplicate, ~`PtsQuality::Bad`) correctly left `undefined` rather than guessed; 3 new tests cover the join, the ambiguous-PTS fallback, and `get_timeline` rejecting gracefully. **Deliberately NOT fixed in this pass**: `Timeline.tsx`/`TimelineThumbnails.tsx` still visually render/scrub/key bars by array position (decode order), not by the new `display_order` — physically reordering the bars is a separate, larger, riskier change (arrow-key nav, hit-testing, and every `setFrameSelection` call assume array position === decode-order `frame_index`), see `applyDisplayOrder`'s doc comment for the full reasoning. No PTS/DTS-mismatch band exists anywhere (unchanged). `docs/FRAME_IDENTITY_CONTRACT.md` still doesn't exist as a file — the rule is only stated inline in code doc-comments and `UX_PARITY_MATRIX.md` §9. CLI's `-display_order` flag was already confirmed fine (`main.rs:101-102`, genuinely PTS-sorts) |
| PERF-01 | Perf budget envelope + LOD degrade sequence (UI frame/hit-test/overlay/tooltip/selection-propagation targets, 4-step graceful degrade on breach) | P1 | [-] | **Verified 2026-08-23**: exact budget numbers (16.6/1.5/6.0/0.8/2.0ms) and the 4-step degrade sequence are real code — but only as a hardcoded `PerfBudgets::default()` in `bitvue-engine/src/parity_harness/gates.rs`, fed exclusively by unit tests (no real GUI telemetry pipeline exists). Its cited source `perf_budget_and_instrumentation.json` doesn't exist in the repo. The degrade sequence has zero frontend implementation. `lockcheck.rs`'s separate perf-smoke thresholds are not wired into `parity_check.sh` or any CI workflow. Real virtualization exists independently (`VirtualizedFilmstrip.tsx`) but isn't connected to this budget model |
| EVB-01 | One-click Evidence Bundle export (manifest+env+version+selection_state+order_type+backend_fingerprint+plugin_versions+warnings+screenshots, 4 entrypoints, ABI compat policy) | P0 | [x] | 4/4 entrypoints; manifest/screenshots/diff-contract/ABI-policy all real. Contract: `UX_PARITY_MATRIX.md` §7. Manifest/env/version/selection_state/order_type/backend_fingerprint/warnings all real (Phase 7.6). Screenshots real as of 2026-08-11 (`4d8c904`) -- `bitvue:captureScreenshot` (Electron `capturePage()`) → base64 → sidecar decode → `screenshots/screenshot_NNNN.png`, verified end-to-end via real selftest run. **2026-08-19: 4th entrypoint (CompareWorkspace toolbar "Export Diff Bundle") wired** -- `CompareWorkspace.tsx` calls the same `useExportEvidenceBundle` hook (now accepting an optional `{workspace, mode}` override) as the other 3 entrypoints, so it's the same `Export.EvidenceBundle` command/bridge call/backend path end to end, just with `workspace: "compare"` and `mode: showDiff ? "diff" : "normal"` metadata recorded in the exported `bundle_manifest.json`. No new backend scope was needed: `bitvue_engine::export::context_menu`'s 5-variant `ContextMenuScope` (Player/HexView/StreamView/Timeline/DiagnosticsPanel) only gates right-click *menu item* guards, unrelated to `export_evidence_bundle`'s `workspace`/`mode` params, which are (and always were) free-form manifest metadata strings, not a guarded enum -- confirmed by reading `export::evidence::EvidenceBundleManifest` and `UX_PARITY_MATRIX.md` §7's entrypoint table, which lists all 4 entrypoints under the identical `Export.EvidenceBundle` command. Verified with a real Electron run (`BITVUE_ELECTRON_SCREENSHOT_OPEN_DEPENDENT=1` + `BITVUE_ELECTRON_SCREENSHOT_CLICK_SELECTOR=".export-diff-bundle-button"` against `test_data/av1_test.ivf` opened as both A and B): a real `bitvue_evidence_<timestamp>/` directory was written to disk with `workspace: "compare"`, `mode: "diff"` in its manifest and a genuine non-blank `screenshots/screenshot_0000.png` (2560x1536, matches the actual window) of the compare view. Found and fixed a real pre-existing automation gap along the way: `window.alert(...)` (the hook's success/failure notice, used by all 4 entrypoints) is a synchronous blocking dialog that hangs `BITVUE_ELECTRON_SCREENSHOT_CLICK_SELECTOR` runs forever with no prior workaround -- `runScreenshotAndExit` (`bitvue-desktop/electron/main.ts`) now stubs `window.alert` to a console log before dispatching any clicks, screenshot-mode only. Diff contract real as of 2026-08-11 (`a271d8b`): `parity_harness::compare_evidence_bundles` was operating on a disconnected duplicate manifest type (never produced by any real bundle) with an unused config param -- fixed to reuse `export::evidence::EvidenceBundleManifest` (the type actually written to `bundle_manifest.json`) and to gate every field check on `config.compare_fields`; added `compare_evidence_bundle_dirs` and a first real caller, `bitvue evidence-diff <a> <b> [--strict]` (`bitvue-cli`). **ABI compat policy real as of 2026-08-11** (2nd commit same day): git history showed all 16 manifest fields were added together in one commit (`eb2cee3`, 2026-01-30) with zero schema changes since, so there was no real removal/rename history to encode -- instead made the policy's 3 rules actually true starting now: (1) additions -- `#[serde(default)]` added at the struct level (backed by the existing `Default` impl), verified a manifest JSON with fields stripped/added still parses instead of hard-failing (previously it hard-failed, a real bug); (2) removals -- `CURRENT_BUNDLE_SCHEMA_VERSION` const + `check_bundle_schema_compatibility()` (MAJOR.MINOR, same-MAJOR=compatible) wired into `compare_evidence_bundles` as an unconditional (not `compare_fields`-gated) `BREAKING` check, verified via a hand-edited real bundle (`bundle_version` bumped to `"2.0"`) correctly surfacing through the CLI; (3) renames -- documented policy to use `#[serde(alias = "old_name")]` for ≥2 MINOR versions before the MAJOR bump that drops it (serde's native mechanism, no new machinery needed) |

---

## Layer 8: Data/Viz Completeness Gaps (NEW — 2026-08-23, `UX_PARITY_MATRIX.md` §1)

Discovered via a code-grep verification pass over §1's 8 previously "⚠️ unverified" rows — real, concrete gaps
(mock data, disconnected calculations, dead components), not just missing features. Full evidence/citations live
in `UX_PARITY_MATRIX.md` §1; this table is the tracked-ID summary.

| ID | Feature | Severity | Status | Notes |
|----|---------|----------|--------|-------|
| DIV-01 | Timestamps/duration, declared vs derived frame rate | P2 | [ ] | Not exposed anywhere in GUI; `FileInfo.fps`/`.duration` never populated by any sidecar command, `bitvue-indexer` hardcodes `duration_ms: None`. CLI `info` has a declared-only rate, no "derived" rate computed anywhere |
| DIV-02 | Compliance/syntax/CRC/missing-ref error list | P1 | [x] | **Fixed 2026-08-23**: bypassed the unused `DiagnosticsManager`/`diagnostics.rs` entirely (confirmed dead — the event bus actually flows through a separate, simpler `event::Diagnostic` type). Fixed 3 already-broken joints instead: (1) `command_support.rs`'s `event_to_json` was serializing `DiagnosticAdded` as an opaque Rust Debug string — now real structured JSON; (2) `index_ivf_av1` now collects real per-frame parse-failure diagnostics (`find_frame_obu`/`parse_frame_header_basic` failures, previously silently swallowed) and a new fourcc-mismatch check (IVF `fourcc != "AV01"` was never validated — real correctness gap, now honestly diagnosed instead of silently mis-parsing); (3) frontend was discarding `indexStream`'s returned events entirely — `FileStateContext.tsx` now captures+stores them, `DiagnosticsPanel`'s "mock diagnostics for demonstration" fallback deleted, real data flows via a new `diagnostics` prop. Verified with a real Electron screenshot opening `vp9_test.ivf` (IVF container, non-AV01 fourcc) through the AV1-only pipeline: Diagnostics panel shows real `All (31)` — 1 Error (fourcc) + 30 Warn (one per misparsed frame) — where it previously showed empty/fabricated data. New Rust regression test (`index_stream_reports_a_diagnostic_for_a_non_av01_fourcc_in_an_ivf_container`) + frontend `bridgeDiagnosticToDiagnostic` mapper tests. Full workspace `cargo test --workspace --exclude abseil --lib --tests` + `cargo fmt`/`clippy` clean, 2293 vitest clean |
| DIV-03 | SEI/metadata (HDR, mastering, color info) | P2 | [-] | H.264 SEI MDCV/CLL genuinely parsed but never read outside `bitvue-avc`; `MetadataInspector` engine is test-only; AV1 has no HDR-metadata parsing at all. **Scoped 2026-08-23, not attempted**: this is not a wiring task — `bitvue-avc` isn't even a dependency of `bitvue-indexer`/`bitvue-sidecar` (AVC files aren't indexed by the production pipeline *at all today*, HDR or not), so exposing AVC SEI needs a whole new `index_avc` pipeline stage first (real new work, not "read one more field"). AV1 (the only codec actually wired) has zero HDR-metadata-OBU parsing (`metadata_obu()`, spec §5.9.28-30) — also new parsing work, not wiring. Additionally **no fixture in this repo has real HDR data** (`av1_test.ivf`'s `timing_info_present_flag=0`; `avc_test.h264`'s VUI has `nal_hrd_parameters_present_flag=0` — both confirmed by direct bit-level parsing) — even fully wired, nothing would render against current fixtures. Cheapest honest verification would be a synthetic-NAL integration test extending `bitvue-avc/tests/integration_test.rs`'s `create_minimal_h264_stream` pattern, not a screenshot. UI target is already settled/cheap if the backend ever lands: `SelectionInfoPanel.tsx`'s "Video Properties" section |
| DIV-04 | Complexity proxies (bpp, motion magnitude) | P2 | [-] | bpp real and shown (as `Av1EfficiencyMapRenderer` spatial overlay, not an aggregate chart); MV-magnitude histogram computed engine-side (`player/mini_charts.rs`) but never exposed |
| DIV-05 | Reference depth + reordering indicators | P2 | [-] | Real display/decode order numbers shown in `DetailsPanel`/`StatisticsTab`; no explicit ref-depth number or reorder-mismatch badge; reorder-detection engine is test-only |
| DIV-06 | HRD/CPB fullness + constraints | P1 | [-] | `HRDBufferPanel.tsx` renders a **disconnected client-side simulation** (hardcoded 1MB buffer/30fps), not the stream's real HRD params — those are parsed then discarded in `bitvue-avc/src/sps.rs`. Complete backend `HrdModel` (591 lines) has zero production callers. **Scoped 2026-08-23, not attempted**: only AVC is a genuine "wire the discarded fields" job (HEVC's `hrd_parameters()` isn't parsed *at all*, only skip-commented in VPS + SPS-VUI stops before reaching it — real new bitstream-parsing work; AV1 has no bit_rate/cpb_size concept at all, its `decoder_model_info`/buffer-delay fields are `skip_bits`-discarded not even stored, and would need a parallel AV1-shaped buffer model, not a plug into `HrdModel`). Even the AVC path is blocked twice over: (1) AVC isn't indexed by the production pipeline at all (same blocker as DIV-03 — `index_stream` is IVF/AV1-only), (2) `avc_test.h264` itself has `nal_hrd_parameters_present_flag=0` (confirmed by direct parsing) — a new fixture encoded with explicit VBV/HRD settings would be needed regardless of wiring. One small, independently-real win identified along the way: IVF's `framerate_num`/`framerate_den` (already parsed, `bitvue-av1-codec/src/ivf.rs`) are never copied into `ContainerModel` — `HRDBufferPanel`'s `frameRate={30}` hardcode could be fixed cheaply and separately from the rest of this item; not done here since it doesn't move DIV-06 itself (folds into DIV-01, the timestamps/frame-rate item, still `[ ]`) |
| DIV-07 | Decode errors + concealment markers | P2 | [ ] | No concealment concept anywhere in the repo; `DecodeStatus::Failed` only ever set in test code; frontend has only a generic whole-file error dialog |
| DIV-08 | PSNR/SSIM series+summary (stream-wide, GUI) | P1 | [-] | CLI `--psnr` has a real series+avg/min; GUI has none — historical `RDCurvesPanel.tsx` deleted, its would-be replacement `QualityMetricsPanel.tsx` is dead Tauri code (unreachable, calls a nonexistent command). Only live GUI path is single-frame-pair PSNR/SSIM in Compare workspace |

**Pattern across DIV-03/05/06**: a real, sometimes substantial backend engine exists (`MetadataInspector`,
reorder-detection in `insight_feed.rs`, `HrdModel`) but is constructed only in test files —
built, tested, never wired to a production caller. Same shape as the `command_chain.rs`/`event_observer.rs`
"Monster Pack" disconnected-scaffolding pattern already flagged for CMP-09 above (axis 8, 2026-08-20) — a
recurring project-wide pattern worth checking for elsewhere before assuming any engine-crate module is live.
**DIV-02 turned out to be a different shape** (fixed 2026-08-23, see its own row): `DiagnosticsManager` really
is unused dead code, but the actual live event pipeline (`event::Diagnostic`/`DiagnosticAdded`) was real and
already flowing — just broken at 3 disconnected joints (opaque Debug-string wire serialization, silently-
swallowed per-frame parse failures, and the frontend discarding `indexStream`'s return value). Worth
distinguishing "engine is genuinely dead" from "engine is live but the wire/frontend joints are broken" before
diagnosing a gap as the former.

---

## Layer 9: Per-Panel Interaction Contract Gaps (NEW — 2026-08-23, `UX_PARITY_MATRIX.md` §3/§4/§5)

Discovered via a code-level verification pass over the 3 highest-traffic panel rows (Player/Timeline/Hex) in
§3's mouse-interaction, §4's tooltip-field, and §5's zoom/pan tables. **Result: those tables describe a design
target, not shipped behavior** — most cells per panel are absent or materially different from what's built.
Full cell-by-cell evidence with file:line citations lives in `UX_PARITY_MATRIX.md` §3/§4/§5 directly (each row
now carries ✅/⚠️/❌ markers); this is the tracked-ID summary. Reference/Metrics/Tree panel rows in §3-§5 are
still unverified — not yet covered by this pass.

| ID | Panel | Severity | Status | Notes |
|----|-------|----------|--------|-------|
| INT-01 | Player surface (`VideoCanvas.tsx`) | P1 | [-] | Pan works but ungated by zoom, Alt+drag-scrub absent; wheel-zoom requires Ctrl/Cmd (contract implies plain wheel); context menu is Details/Export-Evidence-Bundle/Copy-Selection (2 of 3 are no-op stubs), not the pixel/block-copy items the contract describes. **Hover pixel/block tooltip fixed 2026-08-23**: new `resolvePixelValueAtPoint` (`pixelValueLookup.ts`, pure/DOM-free, same chroma-subsampling scale convention `VideoCanvas.tsx`'s `applyChannelMode` already uses) + a new `PlayerPixelTooltip` component wired into `VideoCanvas.tsx`'s mousemove handler (reusing the same `getBoundingClientRect()`-based coordinate conversion the click handler already established, factored into a shared `clientToFrameCoords` helper). Shows frame idx, pixel (x,y), real Y/U/V sample values, resolved block position/size (via the existing `resolveSpatialBlockAtPoint`), and the active-overlays list — matches the `UX_PARITY_MATRIX.md` §4 contract except "Pin Sample", which that doc marks optional/v2 and is not implemented. Suppressed while dragging (drag-pan, not hover) and outside frame bounds. "Copy Pixel" is deliberately not implemented — there's no click-triggered mechanism to capture a transient hover value without material extra complexity, and "Copy Selection" (persisted, click-selected block info) remains a separate, not-yet-wired context-menu item. Verified end-to-end with a **real Electron screenshot** using a new `BITVUE_ELECTRON_SCREENSHOT_MOUSEMOVE=<selector>` harness hook (dispatches a real bubbling `mousemove` at the matched element's center, same pattern as the existing `CONTEXT_MENU` hook) — screenshot shows the tooltip rendering genuine computed values ("Frame #0 · (159, 119)", "Y 18 · U 128 · V 128", "Block 64×32 @ (128, 96)") against the real decoded frame, not fixture/mock data. 11 new/updated tests (`pixelValueLookup` unit tests incl. 420/422/444 subsampling, `VideoCanvas` hover-tooltip suite) + full vitest (2346) + typecheck clean + production build clean. **Fit/100%/200% zoom presets fixed 2026-08-23**: real `1`/`2`/`3` keydown cases added to `YuvViewerPanel/index.tsx`'s handler (previously entirely absent — `keyboardShortcuts.ts`'s matching dialog entries were empty no-op stubs, `unreachable from the live key handler`). "Fit" computes a genuine zoom from the canvas container's actual on-screen size (`clientWidth`/`clientHeight`, measured via a new `containerRef` VideoCanvas exposes) against the frame's logical size — not a fixed guess; "100%"/"200%" are direct `setZoom` calls, both reset pan to `{0,0}`. Also added a toolbar "Fit to Window" button (`ZoomControls.tsx`, `codicon-screen-full`) alongside the keyboard binding. Verified end-to-end with a real Electron run + `BITVUE_ELECTRON_SCREENSHOT_KEY=1` (this harness mechanism dispatches a real bubbling `KeyboardEvent`, unlike `CLICK_SELECTOR`'s synthetic-click limitation) — zoom label genuinely changed from 100% to a computed 153% matching the real container/frame ratio, not a hardcoded value. 12 new/updated tests (keydown cases incl. modifier-key exclusion, toolbar button, real fit-zoom math with a stubbed container size) + full vitest (2334) + typecheck clean. **Click→spatialBlock select fixed 2026-08-23**: real click-vs-drag detection in `VideoCanvas.tsx` (5px movement threshold) resolves the clicked coding-unit block via a new `resolveSpatialBlockAtPoint` (prefers real `partition_grid` leaf blocks over the fixed `qp_grid`), dispatches a new `SelectionContext.setSpatialBlockSelection` (optimistic local `temporal: {type:"block"}` + real `select_spatial_block` bridge call, same shape as `setBitRangeSelection`), and `SelectionInfoPanel`'s "Selection" section now renders the real position/size instead of an always-static placeholder. Completes one of the two missing IA_TRISYNC_PANELS_PRESENT legs for Player (spatial-block/QP-heatmap tri-sync); Unit-tree remains unwired. 55 new/updated tests (hit-test utility, VideoCanvas click-vs-drag, SelectionContext, SelectionInfoPanel) + full vitest suite (2320) + typecheck clean; not screenshot-verifiable (the screenshot harness's `CLICK_SELECTOR` mechanism dispatches `mousedown`+`.click()`, not a real `mouseup`, so it can't exercise click-vs-drag detection — verified via component tests instead, real Electron run confirmed no regression). **"Copy Selection" wired 2026-08-23**: the pre-existing (previously no-op) `Copy.Selection` context-menu item now copies the currently *selected* (clicked, persisted) block's position/size as text — deliberately distinct from the hover tooltip's transient pixel value, which has no click-triggered copy mechanism (see this row's hover-tooltip note above). 2 new tests (real selection + no-selection no-op) + full vitest clean |
| INT-02 | Timeline/Bars (`Timeline.tsx`) | P1 | [-] | Click-select is real (updates real `SelectionContext`); no dblclick-zoom-at-cursor, no drag-pan (drag = frame scrub), no LOD/semantic zoom (see fix note — visual zoom only). Real context menu is only Export-Evidence-Bundle + Copy-Selection — no bookmark/CSV-export/jump/marker-toggle commands exist anywhere in the repo. Tooltip shows only frame_index/type/size-KB, no GOP/IDR/PTS/DTS/marker-flags, zero action buttons. **Ctrl/Cmd+wheel zoom + Shift+drag range-select fixed 2026-08-23** (scoped first via a dedicated Explore pass — see that report's Q1/Q3 for the full reasoning): real `scaleX(zoom)` CSS-transform zoom on `.timeline-thumbnails` (0.5-3x clamp), same technique already proven by `Filmstrip/views/ThumbnailsView.tsx`'s wheel-zoom — explicitly **visual only**, not the semantic/LOD zoom that would actually fix `.timeline-thumbnails`' documented unvirtualized-60k-DOM-node problem (`FileStateContext.tsx:230-239`); that remains out of scope, comparable in size to `VirtualizedFilmstrip.tsx`'s own dedicated component. Real click/drag hit-testing needed zero changes (`document.elementFromPoint` already accounts for ancestor CSS transforms). **Found and fixed a real, previously-undetectable bug while wiring this up**: a plain React `onWheel` JSX handler calling `e.preventDefault()` throws "Unable to preventDefault inside passive event listener invocation." in a real browser (React marks wheel listeners passive by default) — invisible to jsdom's `fireEvent.wheel` (doesn't reproduce a real passive listener) and to every prior test, only surfaced via a real Electron screenshot run (new `BITVUE_ELECTRON_SCREENSHOT_CTRL_WHEEL` harness hook + the selftest's console-error check). Fixed with a real non-passive `addEventListener("wheel", ..., {passive:false})` via a forwarded ref instead of the JSX prop. **`ThumbnailsView.tsx`'s `onWheel={handleWheel}` has this exact same latent bug, confirmed by inspection, deliberately left unfixed (out of scope for this pass)** — flagging here so it isn't rediscovered as new. Shift+drag range-select uses the already-existing `TemporalSelection` "range" type (`types/selection.ts`) via the already-existing `setTemporalSelection` (no new context method needed) — **frontend-only**, no backend round trip: `SelectionState::select_range` exists in `bitvue-engine` but has zero production callers (no `Command::SelectRange` variant, no bridge function) — same legitimate shape as `setSyntaxSelection`'s no-bridge-call pattern, not a gap. Drag mechanics mirror INT-03's `HexViewTab.tsx` mousedown/mousemove/mouseup pattern exactly. Committed + live-drag ranges render a real `.in-range` highlight on every bar in `[min,max]`. 15 new/updated tests (wheel-zoom clamping, shift-drag in both directions, plain-drag regression check, range highlight rendering) + full vitest (2360) + typecheck + production build clean; verified end-to-end with a real Electron screenshot (confirms the passive-listener fix — zero console errors, where the pre-fix version failed the selftest's console-error check) — shift-drag itself isn't screenshot-verifiable for the same reason as INT-01/INT-03 (no real multi-step drag dispatch in the harness), verified via the component/integration tests instead (including a real `document.elementFromPoint` stub, not just the percent fallback) |
| INT-03 | Hex/Bit view (`HexViewTab.tsx`) | P1 | [-] | No dblclick/triple-click, no shift+wheel horiz-scroll. Tooltip is a native `title` (hex offset+value only); a separate click-triggered side panel adds dec/ASCII/local-bit-pattern but never shows the computed mapped-syntax-field or a global bit index. Context menu is only Copy-Bytes + Export-Evidence-Bundle — no Copy-Offset/BitRange, no Jump-to-Syntax, no Bookmark. §5's "no zoom" rule is genuinely honored; the "optional font-size pref" note is aspirational (no such setting exists). **Multi-byte drag-select fixed 2026-08-23**: real mousedown→mouseenter→mouseup drag detection (mirrors `VideoCanvas.tsx`'s INT-01 click-vs-drag pattern) replaces the old single-byte-only `onClick`; a plain click (no movement) still selects exactly 1 byte, a real drag extends `[min,max]` and commits one `select_bit_range` round trip on release (a global `window` mouseup listener, so releasing outside any byte still commits). Highlight now spans the whole range; the info panel shows a Range/Length summary for multi-byte selections (single-byte Value/ASCII/Binary breakdown unchanged for 1-byte); `Copy.Bytes` copies the whole range, space-separated. The incoming Syntax→Hex jump also now highlights a multi-byte node's full byte span, not just its first byte (was always 1-byte before, an accuracy fix as a side effect). 8 new/updated tests + full vitest (2325) + typecheck clean; drag-select itself isn't screenshot-verifiable for the same `CLICK_SELECTOR` limitation as INT-01 (dispatches `mousedown`+`.click()`, no real `mouseup`/`mouseenter` sequence) — verified via component tests, real Electron run confirmed no regression. **Copy Offset / Copy Bit Range fixed 2026-08-23**: 2 new context-menu catalog entries (`context_menu.rs`'s `HexView` scope, both `has_byte_range`-guarded like the pre-existing `Copy.Bytes`) wired in `HexViewTab.tsx` — Copy Offset writes `0x<hex>` of the range start; Copy Bit Range writes `<startBit>-<endBit>` derived directly from the local byte-range state (not `selection.bitRange`, which only reflects the last range whose async `select_bit_range` round trip finished and can be stale for a just-made selection). Jump-to-Syntax and Bookmark remain unimplemented (see the dedicated context-menu scoping pass: Jump needs a new cross-panel active-tab-focus channel, Bookmark needs a data-model change since `BookmarksPanel.tsx`'s model is frame-only). 2 new frontend tests + 2 new Rust tests (catalog item count + enabled-state) + full vitest (2360) + full `cargo test --workspace --exclude abseil --lib --tests` clean |
| INT-04 | Metrics/HRD plots (`FrameSizesView.tsx`/`HRDBufferPanel.tsx`) | P1 | [-] | `QualityMetricsPanel.tsx` (the doc's presumed target) is confirmed still dead/unmounted Tauri code and isn't even a chart. The real, mounted charts are `FrameSizesView`/`HRDBufferPanel` (Filmstrip display modes): HRD has a genuine nearest-point hover tooltip, FrameSizesView has a real click→frame-select; everything else (dblclick zoom-reset, pan, wheel zoom, shift-drag zoom-box, context menu, rich tooltip fields, Pin/Copy actions) is absent from both. No zoom/pan exists at all despite §5 claiming semantic downsample↔full-res |
| INT-05 | Reference graph (`ReferenceGraphPanel.tsx`) | P2 | [ ] | **Not actually a node/edge graph** — a static GOP-card browser (plain DOM flexbox, no canvas/SVG) that is itself unmounted dead code (own file comment: not in any `App.tsx` panel list, `invoke("get_frames")` targets a Tauri API absent under Electron). Every §3/§5 claim (tooltip, node-select+pin, pan/move-node, zoom, context menu) is false against this component. §4 has no Reference-graph row at all — a doc coverage gap independent of the component's dead-code status |
| INT-06 | Trees/Tables (syntax tree / stream tree / diagnostics table) | P1 | [-] | One generic doc row hides real per-component divergence: DiagnosticsPanel click-select is real (local state) with a working 2-item context menu (Export Evidence Bundle + Copy Selection, no "jump"); StreamTreePanel's click handler exists in code but is a **no-op in production** (`App.tsx:137-139` mounts it with zero props, `onUnitSelect` always `undefined`), and its 2-item StreamView context menu (Compare Display/Decode Order) is itself unwired to any real action; the syntax tree (`FrameSyntaxTab.tsx`) has no click handler on its row at all, no context menu (no `ContextMenuScope` variant exists for it), and its tooltip field is always `undefined` for real backend data. None of Copy-Offset/Copy-Path/Bookmark exist anywhere. No zoom exists (correctly, per §5); the "optional global UI-scale setting" is fully aspirational — no Settings/Preferences UI exists in the app at all |

**Remaining Tier 3 candidates, scoped 2026-08-23, not attempted** (two dedicated Explore passes — one per
question set — before any implementation, same discipline as DIV-06/DIV-03): several looked like "just wiring"
but turned out to need real new plumbing or a data-model change, so were deliberately left for a future pass
rather than half-implemented:
- **Player: Export frame image** — real but not free: needs (1) exposing `VideoCanvas`'s internal `canvasRef`
  to its parent, (2) a **new** Electron main-process IPC handler (`dialog.showSaveDialog` + `fs.writeFile` —
  confirmed no such handler exists anywhere, only `capturePage()`/`showOpenDialog` do), (3) preload/bridge
  plumbing, (4) a new `Export.FrameImage` catalog item. All patterns to copy from already exist
  (`evidenceExport.ts`); no new Rust decode/parse work needed.
- **Player: Copy pixel info** (the hover-value half, distinct from the now-fixed Copy-Selection/block-info
  half above) — blocked on there being no click-triggered mechanism to capture a transient hover value; not
  attempted, same reasoning as "Pin Sample" being marked optional/v2.
- **Hex: Jump to Syntax node** — the Hex↔Syntax selection sync is already real and bidirectional
  (`FrameSyntaxTab.tsx`'s reverse-direction auto-expand/scroll already ships); the only missing piece is a
  cross-panel "requested active left-panel tab" channel, since `DockableLayout.tsx`'s active tab is local
  `useState` with no external-caller hook. Small but genuinely new plumbing, not zero-cost.
- **Hex: Bookmark** — confirmed a data-model trap: `BookmarksPanel.tsx`'s `Bookmark` type is strictly
  frame-level (`isValidBookmark` actively rejects any other field), and its `addBookmark`/`removeBookmark`/
  localStorage state are private closures inside that one component, not exported anywhere. A byte-offset
  bookmark needs a schema change + state extracted to a shared hook — deferred.
- **Timeline: Add/Remove Bookmark** — unlike Hex, frame-level granularity is a perfect fit for the existing
  model; the only real work is extracting `BookmarksPanel.tsx`'s bookmark state into a shared hook so both it
  and `Timeline.tsx` read the same list (not two divergent copies). Small-to-medium, well-scoped, deferred only
  for session time, not for any deeper blocker.
- **Timeline: Export Range CSV** — genuinely blocked (not just scoped-large): needs a "selected range" concept,
  which didn't exist until this session's Shift+drag range-select landed (see INT-02's fix above). The CSV
  export utility itself (`dataExport.ts`) is already real and complete — once a range exists, this is one line
  (`exportFramesToCSV(frames.slice(rangeStart, rangeEnd + 1))`). Worth revisiting now that INT-02 is done.
- **Timeline: Jump to Hex/Tree/Player** — mostly real already (frame-selection propagation via
  `SelectionContext` already reaches all three); Player needs nothing more, Tree needs a small scroll-to-frame
  addition (`StreamTreePanel.tsx` doesn't auto-scroll to the current frame today), Hex shares the same
  tab-focus-channel gap as "Jump to Syntax node" above.
- **Timeline: Toggle marker kinds** — confirmed zero foundation (no marker concept anywhere in the codebase);
  needs a marker-kind data model + rendering + persistence designed from scratch before any context-menu item
  makes sense. Correctly out of scope for a quick pass.
- **Timeline: dblclick-zoom-at-cursor, aggressive Ctrl-wheel rate** — small, well-bounded follow-ons now that
  INT-02's zoom state exists (scroll-anchoring math for dblclick; a one-line `delta` branch for aggressive
  rate) — not attempted this pass, but no longer blocked on anything.
- **Timeline: real semantic/LOD zoom** — the one thing that would actually fix `.timeline-thumbnails`'
  documented unvirtualized-60k-DOM-node problem; explicitly NOT what INT-02's visual zoom does (see that row's
  note) — a full bucket-aggregation redesign comparable in size to `VirtualizedFilmstrip.tsx`'s own dedicated
  component, deferred.

**Pattern**: across all 6 panels, a real, working core interaction (click-select) is surrounded by a much larger
ring of documented affordances (hover tooltips, drag-range-select, semantic zoom, rich context menus) that were
designed but never built — or, for Reference graph, an entire component that isn't mounted and isn't even the
kind of UI (node/edge graph) the doc describes. Distinct from Layer 8's "engine built but unwired" pattern —
here the frontend interaction code itself mostly doesn't exist yet. Not yet covered by this pass: Trees/Tables'
`StatisticsTab.tsx` sub-case, and the remaining deferred context-menu/zoom items listed just above.

---

## Layer 10: Robustness & Edge-Case Contracts (NEW — 2026-08-23, `UX_PARITY_MATRIX.md` §14)

Discovered via a code-verification pass over §14's 11 interaction/edge-case rules. Two real robustness gaps
(EDGE-01/02) worth prioritizing over the rest — everything else in this layer is the by-now-familiar
"engine built, never wired" shape (Layer 8's pattern) applied to smaller UI affordances.

| ID | Rule | Severity | Status | Notes |
|----|------|----------|--------|-------|
| EDGE-01 | Per-panel error isolation (a render-time throw in one panel shouldn't take down the whole app) | P1 | [x] | **Fixed 2026-08-23**: new `PanelErrorBoundary` (`frontend/components/PanelErrorBoundary.tsx`) wraps all 12 panel entry-point components in `App.tsx` (Stream/Syntax/Selection/Unit-HEX/Stats/Diagnostics/Player/Compare/Filmstrip/Info/Details/YUV-Diff) with a compact, panel-sized fallback (not the app-wide `position:fixed` one) naming the failed panel + a Retry button. Reuses the existing `ErrorBoundary` catch/reset logic, just a different fallback. 4 new tests including a real sibling-isolation check (one panel's `BrokenPanel` throw leaves 2 working sibling panels' content in the DOM); real Electron screenshot confirms pixel-identical non-error rendering (boundaries are invisible until something fails) |
| EDGE-02 | In-flight request concurrency cap | P2 | [x] | **Fixed 2026-08-23**: `request_dispatch::Semaphore` (`MAX_CONCURRENT_REQUESTS = 16`) caps actual concurrent request *execution* — `spawn_request` still spawns a thread immediately per request (reader loop in `main.rs` stays non-blocking, per its own documented design), but that thread now blocks on `semaphore.acquire()` before calling `compute_frames_with_panic_guard`, re-checking `cancel_flag` after acquiring in case it was cancelled while queued. 2 new real multi-threaded tests (peak-concurrency-never-exceeds-cap with 10 threads/3 permits; waiter-blocks-until-release). Full workspace `cargo test --workspace --exclude abseil --lib --tests` clean (incl. the pre-existing `subprocess_smoke.rs` real-subprocess integration tests), `cargo fmt`/`clippy` clean, real Electron run (250-frame stream, many concurrent requests during load) confirmed no regression |
| EDGE-03 | PTS-quality badge (OK/WARN/BAD) in Timeline | P2 | [x] | **Fixed 2026-08-23**: `TimelineBase` gained a real `pts_quality` field, set from `FrameIndexMap.pts_quality()` (already computed on every `get_timeline` call, previously discarded) inside `TimelineExtractor::extract_timeline`'s default impl — one line. Wired through `BridgeTimeline.pts_quality` → `FileStateContext.tsx`'s `applyDisplayOrder` (same `get_timeline` call CTX-02 already made) → `FrameDataContext`'s `streamInfo.ptsQuality` → new `PtsQualityBadge` component, rendered in `TimelineHeader` only for Warn/Bad (Ok/null render nothing, avoiding a permanent no-op pill on the overwhelming common case). Along the way fixed a real doc/tooltip inaccuracy: `PtsQuality::Bad`'s tooltip claimed "non-monotonic" PTS triggers Bad, but the code explicitly only checks duplicates/>50%-missing (decode-order non-monotonicity from B-frames is intentionally not flagged) — corrected the tooltip text to match actual behavior. 2 new Rust tests (clean-stream→Ok, duplicate-PTS→Bad round-tripping through `build_timeline_av1`) + frontend tests (`TimelineHeader` badge rendering, `Timeline` real streamInfo→badge integration, `FileStateContext` Ok/Warn/Bad passthrough + reject-leaves-null). No fixture in this repo has non-Ok PTS (verified: `av1_test.ivf`'s PTS is exactly `0..249` sequential) — Warn/Bad are unit-test-verified only, not screenshot-verified, by necessity. Full workspace `cargo test --workspace --exclude abseil --lib --tests` clean (one confirmed pre-existing unrelated timing flake in `compare_cache_test.rs`, passes serially), `cargo fmt`/`clippy` clean, 2302 vitest clean |
| EDGE-04 | Overlay coverage-% legend + auto-disable <20% | P2 | [ ] | `qp_heatmap.rs`'s `coverage_percent()`/`has_sufficient_coverage()` match the rule almost verbatim but are test-only callers; no sidecar exposure, no frontend legend |
| EDGE-05 | Indexing-in-progress frame-jump gating ("Index building…") | P2 | [ ] | `indexing.rs`'s `IndexReadyGate`/`OpenFastPath` implement this near-verbatim and are unit-tested, but have zero callers outside their own module — production sidecar uses an unrelated simple `indexed: bool` flag with no partial-range gating |
| EDGE-06 | Selection highlight drawn topmost over all overlays | P3 | [ ] | No "selection highlight" concept exists anywhere in the overlay pipeline (2-pass draw order only); a structurally similar `overlay_stack.rs` z-order module has no `Selection` layer type and zero callers |

**Confirmed already solid, no action needed**: overlay missing-data fallback (explanatory box, ~14 renderers,
consistent); compare alignment-confidence diff-gating (real, end-to-end, `alignment.rs`→`compare.rs`→
`CompareWorkspace.tsx`); compare resolution-mismatch diff-disable (real, shares the same path — only its
"experimental resample" escape hatch is unused dead code, not worth tracking).

---

## Regression Suite Status

**2026-07-31: now CI-enforced** — `.github/workflows/ci.yml` job `parity-regression` runs
`scripts/parity_check.sh --local` on every PR touching Rust code. This log no longer needs manual pasting;
check the CI run for the ground truth. Also fixed the same day: `bitvue-avs3`, `bitvue-jpegxs`, `bitvue-vc3`,
`bitvue-codecs-parser` had real `#[test]` coverage but were missing from the CI test matrix since they landed
(Phase 5/6) — their tests had never run in CI until now.

Manual local run: `./scripts/run_regression_suite.sh`

```
Last manual run: 2026-04-20 (stale — see CI from here on)
PASS: 29 (parity_check.sh --local)
FAIL: 0
cargo test -p bitvue-cli --test parity_test: 29/29
```

---

## Remaining Work

All P0/P1 items in Layers 1-5 are complete. **Layer 6 (added 2026-07-31) is mostly unstarted, but CMP-01/02 are
partially built** (compare alignment + side-by-side view already exist in code — see corrected status above,
2026-07-31 `_import_v14` mining pass) — competitor research surfaced dual-stream compare and VMAF as gaps not
previously tracked. See VQA_PARITY_SPEC_V3.md §4.9/§1.5/Phase 7.5 for implementation detail. **Layer 7 (added
2026-07-31) is entirely unstarted** — context-menu system and one-click evidence bundle export, both real UX
gaps with concrete contracts now spec'd in `UX_PARITY_MATRIX.md` §6/§7, promoted to `DEVELOPMENT_PHASES.md`
Phase 7.6 (see "우선순위 재검토" note there for why it's not deferred to Phase 11). **Layer 8 (added 2026-08-23)
is a mix of mock/disconnected UI (DIV-02/06, `[-]`) and fully absent features (DIV-01/07, `[ ]`) — see that
table's "Pattern" note for the recurring test-only-engine-crate root cause behind several of them.** **Layer 9
(added 2026-08-23) found the Player/Timeline/Hex per-panel interaction contracts in `UX_PARITY_MATRIX.md`
§3-§5 are mostly unbuilt design target, not shipped behavior — see that table's "Pattern" note.**

## Fixture Files

Generated with ffmpeg testsrc (synthetic, 30 frames, 320×240):

| File | Codec | Size |
|------|-------|------|
| `test_data/av1_test.ivf` | AV1 IVF | (existing) |
| `test_data/hevc_test.hevc` | HEVC Annex B | 7.5 KB |
| `test_data/avc_test.h264` | AVC Annex B | 7.4 KB |
| `test_data/vp9_test.ivf` | VP9 IVF | 11.4 KB |
