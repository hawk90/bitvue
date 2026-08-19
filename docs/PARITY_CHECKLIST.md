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
| CMP-09 | AV3/AVM naming reconciliation | P1 | [ ] | Confirm Bitvue's "AV3" and VQ Analyzer's "AVM" refer to the same codec |
| CMP-10 | CABAC range/state visualization (HEVC/AVC) | P1 | [ ] | Both VQ Analyzer and VEGA expose this; Bitvue coverage unverified |

**Explicitly out of scope** (broadcast QC territory, not a bitstream analyzer concern): live TS conformance
(TR 101290, SCTE-35, CableLabs), captions (EIA-608/708, DVB), audio codec/loudness analysis. See spec §1.5.

---

## Layer 7: UX Interaction Contracts (NEW — 2026-07-31, `UX_PARITY_MATRIX.md` §6/§7/§9)

Discovered via the V14-pack UX mining pass — real gaps, not codec/compare features so they don't fit Layer 6.
Referenced from `DEVELOPMENT_PHASES.md` Phase 7.6 the same way Layer 6 is referenced from Phase 7.5.

| ID | Feature | Severity | Status | Notes |
|----|---------|----------|--------|-------|
| CTX-01 | Context-menu system (Player/HexView/StreamView scopes, guard+disabled-reason policy) | P1 | [x] | Contract: `UX_PARITY_MATRIX.md` §6 (3 scopes, 7 items, 3 guards: `always`/`has_selection`/`has_byte_range`). **Verified real 2026-08-11** (found stale during a UX audit -- this row still said `[ ]` after the feature had shipped): `bitvue_engine::export::context_menu` implements the full guard-evaluated catalog for **5** scopes (Player/HexView/StreamView/Timeline/DiagnosticsPanel -- 2 more than the original §6 spec, added `f3e9e81`/`35955fc`/`e545bba`/`7b8bfc9`), backed by real Rust unit tests (`export/export_test.rs::context_menu_tests`, `tests/export.rs`) and sidecar IPC tests (`bitvue-sidecar/src/context_menu.rs`). Frontend: generic `ContextMenu.tsx` (guard-evaluated items rendered with disabled+tooltip state) is wired via real `onContextMenu` handlers + `getContextMenuItems()` calls in all 5 consumer panels (`YuvViewerPanel`/`VideoCanvas` for Player, `HexViewTab`, `StreamTreePanel`, `Timeline`, `DiagnosticsPanel`) -- confirmed by direct grep, not just presence of the component. |
| EVB-01 | One-click Evidence Bundle export (manifest+env+version+selection_state+order_type+backend_fingerprint+plugin_versions+warnings+screenshots, 4 entrypoints, ABI compat policy) | P0 | [x] | 4/4 entrypoints; manifest/screenshots/diff-contract/ABI-policy all real. Contract: `UX_PARITY_MATRIX.md` §7. Manifest/env/version/selection_state/order_type/backend_fingerprint/warnings all real (Phase 7.6). Screenshots real as of 2026-08-11 (`4d8c904`) -- `bitvue:captureScreenshot` (Electron `capturePage()`) → base64 → sidecar decode → `screenshots/screenshot_NNNN.png`, verified end-to-end via real selftest run. **2026-08-19: 4th entrypoint (CompareWorkspace toolbar "Export Diff Bundle") wired** -- `CompareWorkspace.tsx` calls the same `useExportEvidenceBundle` hook (now accepting an optional `{workspace, mode}` override) as the other 3 entrypoints, so it's the same `Export.EvidenceBundle` command/bridge call/backend path end to end, just with `workspace: "compare"` and `mode: showDiff ? "diff" : "normal"` metadata recorded in the exported `bundle_manifest.json`. No new backend scope was needed: `bitvue_engine::export::context_menu`'s 5-variant `ContextMenuScope` (Player/HexView/StreamView/Timeline/DiagnosticsPanel) only gates right-click *menu item* guards, unrelated to `export_evidence_bundle`'s `workspace`/`mode` params, which are (and always were) free-form manifest metadata strings, not a guarded enum -- confirmed by reading `export::evidence::EvidenceBundleManifest` and `UX_PARITY_MATRIX.md` §7's entrypoint table, which lists all 4 entrypoints under the identical `Export.EvidenceBundle` command. Verified with a real Electron run (`BITVUE_ELECTRON_SCREENSHOT_OPEN_DEPENDENT=1` + `BITVUE_ELECTRON_SCREENSHOT_CLICK_SELECTOR=".export-diff-bundle-button"` against `test_data/av1_test.ivf` opened as both A and B): a real `bitvue_evidence_<timestamp>/` directory was written to disk with `workspace: "compare"`, `mode: "diff"` in its manifest and a genuine non-blank `screenshots/screenshot_0000.png` (2560x1536, matches the actual window) of the compare view. Found and fixed a real pre-existing automation gap along the way: `window.alert(...)` (the hook's success/failure notice, used by all 4 entrypoints) is a synchronous blocking dialog that hangs `BITVUE_ELECTRON_SCREENSHOT_CLICK_SELECTOR` runs forever with no prior workaround -- `runScreenshotAndExit` (`bitvue-desktop/electron/main.ts`) now stubs `window.alert` to a console log before dispatching any clicks, screenshot-mode only. Diff contract real as of 2026-08-11 (`a271d8b`): `parity_harness::compare_evidence_bundles` was operating on a disconnected duplicate manifest type (never produced by any real bundle) with an unused config param -- fixed to reuse `export::evidence::EvidenceBundleManifest` (the type actually written to `bundle_manifest.json`) and to gate every field check on `config.compare_fields`; added `compare_evidence_bundle_dirs` and a first real caller, `bitvue evidence-diff <a> <b> [--strict]` (`bitvue-cli`). **ABI compat policy real as of 2026-08-11** (2nd commit same day): git history showed all 16 manifest fields were added together in one commit (`eb2cee3`, 2026-01-30) with zero schema changes since, so there was no real removal/rename history to encode -- instead made the policy's 3 rules actually true starting now: (1) additions -- `#[serde(default)]` added at the struct level (backed by the existing `Default` impl), verified a manifest JSON with fields stripped/added still parses instead of hard-failing (previously it hard-failed, a real bug); (2) removals -- `CURRENT_BUNDLE_SCHEMA_VERSION` const + `check_bundle_schema_compatibility()` (MAJOR.MINOR, same-MAJOR=compatible) wired into `compare_evidence_bundles` as an unconditional (not `compare_fields`-gated) `BREAKING` check, verified via a hand-edited real bundle (`bundle_version` bumped to `"2.0"`) correctly surfacing through the CLI; (3) renames -- documented policy to use `#[serde(alias = "old_name")]` for ≥2 MINOR versions before the MAJOR bump that drops it (serde's native mechanism, no new machinery needed) |

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
Phase 7.6 (see "우선순위 재검토" note there for why it's not deferred to Phase 11).

## Fixture Files

Generated with ffmpeg testsrc (synthetic, 30 frames, 320×240):

| File | Codec | Size |
|------|-------|------|
| `test_data/av1_test.ivf` | AV1 IVF | (existing) |
| `test_data/hevc_test.hevc` | HEVC Annex B | 7.5 KB |
| `test_data/avc_test.h264` | AVC Annex B | 7.4 KB |
| `test_data/vp9_test.ivf` | VP9 IVF | 11.4 KB |
