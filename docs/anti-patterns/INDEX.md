# Bitvue — Rust Video-Analysis Anti-Pattern Catalog (INDEX)

> Reference catalog, not yet an audit. Every item's `Bitvue 판정` (verdict) field reads "미정" — this is
> Phase 1 of the plan (build the catalog); Phase 2 is a separate later pass that scans the actual repo and
> fills in Confirmed/Suspected/N/A per item with file:line evidence, via `.claude/workflows/anti-pattern-scan.js`
> (built 2026-07-31, not yet run — see "Running the audit" below).
> See also: `CLAUDE.md` (repo doc map), `docs/DEVELOPMENT_PHASES.md` §Phase 7.5 (the Analyzer/VQ-Probe
> architecture-separation rule this catalog's structure mirrors).

**Total: 970 items across 47 files, four domain waves.** This exceeds the user's own working estimate of
700-900 raw candidates compressing to a 350-500 operational catalog. A targeted dedup pass (3 file-pairs
checked, not a full sweep) found genuinely little true duplication — see "Dedup pass results" below — so the
excess is mostly real breadth, not redundancy, but a fuller consolidation read-through is still recommended
before treating every item as equally load-bearing.

## Dedup pass results (2026-07-31, 3 targeted pairs, not a full sweep)

Flag-don't-merge passes — nothing deleted or restructured, only one-line cross-reference notes added before
each flagged item's `**Bitvue 판정**` line.

| Pair checked | 중복 (true duplicate) | 관련 (related, distinct angle) |
|---|---|---|
| `CACHE.md` vs `PIPE.md` | 0 | 5 |
| `IPC.md` vs `TAURI_CMD.md` | 3 | 1 |
| `UIX_VIZ.md` vs `PIXEL.md`/`HEAT.md` | 0 | 8 |

Only 3 true duplicates found across ~90 items compared (IPC-015↔TAURI-CMD-004, IPC-005↔TAURI-CMD-005,
IPC-017↔TAURI-CMD-008 — all lock-contention/state-snapshot/stale-response patterns that happened to get
written from both a "payload design" and a "command design" angle). The other 16 flagged pairs turned out to
be the same *failure pattern* applied to architecturally distinct subsystems (e.g. UIX_VIZ vs HEAT: several
items are "auto-normalize/outlier/sentinel-value" bugs recurring in both a QP heatmap and a quality-score
heatmap — different code, same class of mistake), which is exactly what you'd expect given the Analyzer/
VQ-Probe architectural separation this catalog mirrors. Not yet checked: any pair outside these 3 — full
coverage would need ~C(47,2) comparisons, clearly not worth doing exhaustively; spot-check further pairs on
demand if a specific overlap is suspected.

## Wave 1 — Rust & Media Engineering (14 files, 354 items)

Code-level Rust anti-patterns, mostly Bitvue's own bitstream-analyzer domain but broadly reusable for any
Rust media-processing codebase.

| File | Category | Items |
|---|---|---|
| `OWN.md` | 소유권·수명·복사 (Ownership, Lifetimes, Copying) | 23 |
| `MEM.md` | 메모리와 Allocation | 35 |
| `LAYOUT.md` | 데이터 레이아웃과 캐시 지역성 | 30 |
| `PARSE.md` | 파서와 Bitstream 안전성 — largest file, malformed/adversarial input handling | 40 |
| `CODEC.md` | 컨테이너와 코덱 경계 (container/codec architecture) | 25 |
| `IO.md` | 파일 I/O와 Zero-copy | 20 |
| `CONC.md` | Async와 병렬 처리 (Tokio/Rayon nesting, cancellation) | 30 |
| `CACHE.md` | 캐시와 인덱싱 | 25 |
| `IPC.md` | Tauri IPC와 직렬화 (payload size/format) | 20 |
| `PIXEL.md` | Decode·Pixel·Image Pipeline | 20 |
| `ERR.md` | Error·panic·복구 (Rust-side error/panic design) | 20 |
| `PERF.md` | 벤치마크·프로파일링·CI 안티패턴 (measurement pitfalls, not bugs directly) | 24 |
| `API_TYPE.md` | API·타입 설계 (workspace facade/re-export, newtype, dispatch) | 24 |
| `FRONTEND.md` | 프런트엔드 렌더링 연계 (rendering mechanics/layer separation) | 18 |

## Wave 2 — VQ-Probe Quality-Analysis Domain (7 files, 140 items)

Architecturally separate from Wave 1 by design (see `DEVELOPMENT_PHASES.md` Phase 7.5's anti-pattern warning
against a `UniversalFrameAnalysis` god-struct). Covers Bitvue's planned dual-stream compare/VMAF feature set.

| File | Category | Items |
|---|---|---|
| `ALIGN.md` | 프레임 정렬 (two-stream temporal alignment) | 18 |
| `SPATIAL.md` | 해상도와 공간 정렬 | 18 |
| `COLOR.md` | 색공간과 Bit Depth (HDR/SDR comparison correctness) | 20 |
| `METRIC.md` | 품질 지표 구현 — PSNR (10) / SSIM (10) / VMAF (10) | 30 |
| `PIPE.md` | 프레임 재사용과 계산 공유 (avoid N× redundant decode across metrics) | 18 |
| `HEAT.md` | Heatmap과 공간 통계 (quality-score heatmap computation) | 18 |
| `STAT.md` | 집계와 통계 (aggregation/statistical reporting integrity) | 18 |

## Wave 3 — UI/UX + Tauri + React (15 files, 257 items)

Not reachable by static code review alone — see each file's 탐지 method (Code/Interaction/Visual/
Performance/User test/Domain review). `UIX_SYNC.md` is the highest-severity file in this wave: tri-sync
(Timeline⇄Player⇄Syntax⇄Hex selection consistency) is Bitvue's defining feature.

| File | Category | Items |
|---|---|---|
| `UIX_IA.md` | 정보 구조와 탐색 | 15 |
| `UIX_SYNC.md` | 선택과 동기화 — **highest severity, Bitvue's core UX contract** | 15 |
| `UIX_ASYNC.md` | 비동기 상태와 피드백 | 15 |
| `UIX_TIMELINE.md` | Timeline 안티패턴 | 22 |
| `UIX_TREE_HEX.md` | Syntax Tree와 Hex View | 20 |
| `UIX_VIZ.md` | 전문 시각화의 의미 왜곡 (QP/MV overlays + VQ-Probe charts that mislead interpretation) | 25 |
| `UIX_INPUT.md` | 조작과 키보드 UX | 18 |
| `UIX_LAYOUT.md` | 레이아웃과 패널 | 15 |
| `UIX_ERR.md` | 오류 메시지와 진단 UX (user-facing counterpart to Wave 1's `ERR.md`) | 18 |
| `UIX_A11Y.md` | 접근성 | 12 |
| `TAURI_CMD.md` | Command 중심 설계 (granularity/orchestration, not payload format) | 10 |
| `TAURI_EVT.md` | Event 남용 | 10 |
| `TAURI_WEB.md` | WebView와 Native 경계 | 10 |
| `FRONT_REACT.md` | React 상태·렌더링 (state-management architecture, reconciled against `FRONTEND.md`) | 18 |
| `UX_SCENARIO.md` | 자동화 검사(10) + 계측 지점(11) + 승인 시나리오(13) — the verification closer for this wave | 34 |

## Wave 4 — Systems & Infrastructure (11 files, 219 items)

SIMD and GPU/CUDA deliberately excluded — no SIMD-intrinsic or GPU code exists in the repo yet to ground
concrete examples in; revisit if/when that changes.

| File | Category | Items |
|---|---|---|
| `FFI.md` | Rust ↔ C/C++ FFI (dav1d/libvmaf boundary — UB/crash risk, not just perf) | 22 |
| `DEC.md` | Decode·FFmpeg 통합 (decoder state-machine/timestamp semantics) | 20 |
| `RPERF.md` | Rust 성능 추상화 (dyn dispatch, iterator chains, Drop timing) | 20 |
| `SER.md` | Serialization·Schema·대용량 데이터 모델 (export/CLI/MCP, broader than Wave 1's `IPC.md`) | 22 |
| `PLUGIN.md` | 코덱·플러그인 확장 구조 | 20 |
| `MCP.md` | MCP·AI 연동 (real code exists: `crates/bitvue-mcp`, `bitvue-core/src/mcp.rs`) | 20 |
| `BUILD.md` | 빌드·Feature·Workspace 관리 | 20 |
| `PLAT.md` | 크로스플랫폼 데스크톱 (OS/filesystem, distinct from Wave 3's `TAURI_WEB.md`) | 20 |
| `TEST.md` | 테스트 전략 안티패턴 | 20 |
| `OBS.md` | 관측성·진단·프로파일링 | 20 |
| `SEC.md` | Security & Untrusted Input (the closing category — untrusted file handling) | 15 |

## ID prefix reference

`OWN` `MEM` `LAYOUT` `PARSE` `CODEC` `IO` `CONC` `CACHE` `IPC` `PIXEL` `ERR` `PERF` `API` `FE` (Wave 1) ·
`ALIGN` `SPATIAL` `COLOR` `METRIC-PSNR/SSIM/VMAF` `PIPE` `HEAT` `STAT` (Wave 2) · `UIX-IA/SYNC/ASYNC/TIME/
TREE/HEX/VIZ/INPUT/LAYOUT/ERR/A11Y` `TAURI-CMD/EVT/WEB` `FRONT-STATE/RENDER` `UX-SCENARIO` (Wave 3) ·
`FFI` `DEC` `RPERF` `SER` `PLUGIN` `MCP` `BUILD` `PLAT` `TEST` `OBS` `SEC` (Wave 4)

## Every item's record shape

Wave 1/2/4 (code-level): ID/이름, 분류/심각도/탐지, 나쁜 예 (code), 문제, 발생 조건, 권장 (code), 탐지 방법,
예외, **Bitvue 판정: 미정**.
Wave 3 (UX-level, code snippets optional): ID/이름, 분류/심각도/탐지 (Code/Interaction/Visual/Performance/
User test/Domain review), 사용자 목표, 증상, 원인, 구현 냄새, 영향, 권장, 탐지, **Bitvue 판정: 미정**.

## Running the audit

`.claude/workflows/anti-pattern-scan.js` runs Phase 2 (the actual repo audit): one agent per catalog file,
grep/read the relevant code area, classify each item Confirmed/Suspected/N/A with evidence, write the verdict
back into the file's `Bitvue 판정` line. `Workflow({scriptPath: '.claude/workflows/anti-pattern-scan.js', args: {files: ['PARSE','MEM']}})`
to scope it, or omit `args` to audit all 47 files (expensive — one agent per file, run deliberately, not by
default). Not yet run as of this catalog's completion (2026-07-31).
