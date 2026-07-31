# Bitvue — Rust Video-Analysis Anti-Pattern Catalog (INDEX)

> Reference catalog, not yet an audit. Every item's `Bitvue 판정` (verdict) field reads "미정" — this is
> Phase 1 of the plan (build the catalog); Phase 2 is a separate later pass that scans the actual repo and
> fills in Confirmed/Suspected/N/A per item with file:line evidence, via `.claude/workflows/anti-pattern-scan.js`
> (not yet built — blocked on this catalog being finished, see `docs/anti-patterns/` in `CLAUDE.md`).
> See also: `CLAUDE.md` (repo doc map), `docs/DEVELOPMENT_PHASES.md` §Phase 7.5 (the Analyzer/VQ-Probe
> architecture-separation rule this catalog's structure mirrors).

**Total: 751 items across 36 files, three domain waves.** Compare against the user's own working estimate
of 700-900 raw candidates compressing to a 350-500 operational catalog — this collection is already at the
high end of that range without a Phase 4 (systems/infra: FFI, FFmpeg/decoder integration, SIMD, GPU/CUDA,
serialization, plugin architecture, MCP/AI integration, build/workspace, cross-platform, testing,
observability, security). **Recommend pausing here and doing a dedup/consolidation pass before adding more**,
rather than growing further — see "Next steps" below.

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

## ID prefix reference

`OWN` `MEM` `LAYOUT` `PARSE` `CODEC` `IO` `CONC` `CACHE` `IPC` `PIXEL` `ERR` `PERF` `API` `FE` (Wave 1) ·
`ALIGN` `SPATIAL` `COLOR` `METRIC-PSNR/SSIM/VMAF` `PIPE` `HEAT` `STAT` (Wave 2) · `UIX-IA/SYNC/ASYNC/TIME/
TREE/HEX/VIZ/INPUT/LAYOUT/ERR/A11Y` `TAURI-CMD/EVT/WEB` `FRONT-STATE/RENDER` `UX-SCENARIO` (Wave 3)

## Every item's record shape

Wave 1/2 (code-level): ID/이름, 분류/심각도/탐지, 나쁜 예 (code), 문제, 발생 조건, 권장 (code), 탐지 방법,
예외, **Bitvue 판정: 미정**.
Wave 3 (UX-level, code snippets optional): ID/이름, 분류/심각도/탐지 (Code/Interaction/Visual/Performance/
User test/Domain review), 사용자 목표, 증상, 원인, 구현 냄새, 영향, 권장, 탐지, **Bitvue 판정: 미정**.

## Next steps

1. **Dedup/consolidation pass before Phase 4** — 751 raw items likely has real overlap (e.g. `CACHE.md`
   vs `PIPE.md` on frame-lifetime management, `IPC.md` vs `TAURI_CMD.md` on command design, `UIX_VIZ.md`
   vs `PIXEL.md`/`HEAT.md` on overlay rendering). Not yet done — flagging rather than doing it here, since
   it needs a full read-through, not a mechanical merge.
2. **Phase 4 (systems/infra) is deferred, scope-check first**: FFI, FFmpeg/decoder integration, Rust perf
   abstractions, serialization/schema, plugin architecture, MCP/AI integration, build/workspace, cross-platform
   desktop, testing/fuzzing, observability, security. SIMD and GPU/CUDA specifically excluded — no
   SIMD-intrinsic or GPU code exists in the repo yet to ground concrete examples in.
3. **`.claude/workflows/anti-pattern-scan.js`** — the actual repo audit, blocked on steps 1-2 landing (or a
   decision to skip them and audit what exists now).
