# Bitvue — UI/UX Parity Matrix

> UX/interaction parity design targets + reference data (competitor targets, menu structure, overlay color
> scales, wireframes). Distilled from the V14 design pack (`docs/_import_v14/`, gitignored) and VQ Analyzer /
> StreamEye research. The V14 pack is a design blueprint — its contracts are targets, not proof of what ships.
> V14 pack age: source files dated 2026-01-09..01-15 (found in `~/Downloads`), ~6-7 months before this doc was first
> compacted (2026-07-31). Its own `[x]` marks describe the spec being locked, not code being built.
> **See also:** `docs/specs/features.yaml` (the only status source of truth for every item below) ·
> `VQA_PARITY_SPEC_V3.md` (backend/codec spec) · `PARITY_CHECKLIST.md` · `COMPETITOR_FEATURE_MATRIX.md` ·
> `DEVELOPMENT_PHASES.md`.

## 0. Competitor UX reference targets

| id | product | vendor | reference type | priority | note |
|---|---|---|---|---|---|
| vq_analyzer | VQ Analyzer | ViCueSoft | user guide + screenshots | 1 | primary nav/mode/panel/tri-sync UX reference |
| elecard_streameye | StreamEye | Elecard | PDF guide + screenshots | 2 | reporting/export/error-viz discipline reference |
| intel_vpa | Video Pro Analyzer | Intel | PDF guide + screenshots | 3 | secondary diagnostics-pattern reference — not yet web-researched (backlog) |

Rule: match *intent/behavior*, not pixel-for-pixel visual design.

## 1. Data/Viz domain superset

Every measurable data item a competitor exposes must have in Bitvue: a default viz, an export path (or an
explicit "why not"), and a jump-to affordance. Domains: Stream/Container, Bitstream Syntax, Frame Temporal,
Decoder-Path, Spatial Overlay, Quality/Compare.

> Feature/status items for this section live in `docs/specs/features.yaml` (areas: container, syntax, ui,
> metrics, overlay, decode, compare; legacy ids DIV-01..08, OV-01..03, CMP-01..07, IA-02..05).
> Note: the desktop indexing pipeline is IVF/AV1-only (`INFRA-001`), so stream-data items are AV1-only in the GUI.

## 2. Compare-AB workspace (from V14 `WS_COMPARE_AB.md`)

**User questions:** where do two streams differ (quality/structure/errors)? what's the root cause? can the
delta become a CI regression guard?

**Target layout:** 2-col grid — left: Timeline A (180px) / Timeline B (180px) / Delta lane (120px) with a
shared cursor; right (min 320px): Compare Controls (sync mode Off/Playhead/Full, metric selector, export) +
Regression rules + Violations list; optional bottom Player Compare strip (240px, side-by-side or diff heatmap).
W4 wireframe (§12) is the visual precedent. Acceptance tests WS51-WS53 (delta lane, rule from selection,
violation → evidence).

- **Component tree:** `CompareWorkspace` → `CompareHeader`(sync mode, metric selector, export) + `Grid2Col`(`TimelinesCol`[TimelineA, TimelineB, DeltaTimeline], `ControlsCol`[RegressionRulesPanel, ViolationsList, CreateRuleFromSelectionButton]) + optional `PlayerCompareStrip`.
- **Layers (simultaneous):** (A) dual timeline + delta lane, shared cursor, sync modes Off/Playhead/Full (B) dual metrics + delta + threshold-exceed regions (C) player compare: side-by-side or diff heatmap (D) regression-guard mini panel (active rules + violations).
- **Interaction:** cursor move syncs A/B · click delta-exceed region → jump to Player compare at that frame · right-click → create regression rule from selection.
- **Tooltip (delta target):** A value, B value, delta, rank-among-worst.
- **Export:** compare report (delta summary + worst frames + evidence snapshots) · regression rules JSON.

### 2.1 Alignment policy (from `compare_alignment_contracts.json` + `COMPARE_ALIGNMENT_POLICY.md`)

Correctness rule for A/B compare: how two streams are mapped frame-to-frame before any diff/delta is computed.

| Axis | Type | Notes |
|---|---|---|
| display_order | DisplayIdx | Explicit; never infer from decode order |
| decode_order | DecodeIdx | Explicit; never infer from display order |
| pts | Timestamp | PTS-based mapping for display comparisons |
| dts | Timestamp | DTS-based mapping for decode comparisons |
| picture_hash | Hash | Bitstream-derived picture digest where available |
| content_hash | Hash | Decoded YUV digest (backend-specific, must be labeled) |
| sps_pps_config | ConfigKey | Codec config matching guard |
| temporal_unit_id | Id | TU/AU mapping where applicable |

**Alignment order (policy):** 1) PTS-based (primary) → 2) display_idx fallback → 3) nearest-neighbor with gap indicator.

**Mismatch handling:** reorder detected → show alignment warning + offer axis switch · drop detected → mark missing
frames with reason · config mismatch → block compare + surface detailed reason · resolution mismatch beyond
tolerance → disable diff overlays, show side-by-side with scale indicator instead.

**Evidence must record:** chosen axis, mismatch events, order_type. Sync playback and Find First Difference must
both use this axis-resolution policy, not ad-hoc frame-index matching.

> Feature/status items for §2/§2.1 live in `docs/specs/features.yaml` (area: compare; legacy ids CMP-01..04).

## 3-5. Per-panel interaction, tooltip and zoom/pan contracts

Intent (from `MOUSE_INTERACTION_SPEC.md`, `TOOLTIP_SPEC.md`, `ZOOM_POLICY.md`): each panel (Timeline/Bars,
Metrics/HRD plots, Reference graph, Player surface, Hex/Bit view, Trees/Tables) has a defined
hover/click/drag/wheel/modifier/context-menu behavior, a required tooltip field+action set, and a zoom type
(Semantic for timeline/charts, Visual for player/thumbnails, Fixed-density/no-zoom for hex/trees).

- **Global modifier convention:** Shift = range/multi-select · Ctrl/Cmd = additive select/pin · Alt/Option =
  alternate mode (scrub/fine-adjust).
- **Global tooltip rules:** ≤150ms debounced, never cover the target, stable on same target, multi-line +
  monospace + copy buttons, units always shown, explicit "N/A", hit-test dense targets by logical ID.
  Pinnable tooltip → inspector box is v2 (don't block it architecturally).
- **Keyboard shortcut ownership:** renderer-side `frontend/hooks/useKeyboardNavigation.ts` owns all
  shortcuts; the macOS native menu (`bitvue-desktop/electron/nativeMenu.ts`) deliberately sets no
  accelerators to avoid double-firing. Cross-check modifier consistency with the convention above.

> Feature/status items for §3-§5 live in `docs/specs/features.yaml` (area: ux; legacy ids INT-01..06).

## 6. Context menus & guards (from `context_menus.json` + `guard_rules.json`)

V14 starter contract (3 scopes; marked "expand via screenshot-driven refinement" in the source — the §3 per-panel
targets are the fuller goal). Bitvue added Timeline and DiagnosticsPanel scopes and the HexView Copy Offset /
Copy Bit Range items. Catalog lives in `crates/bitvue-engine/src/export/context_menu.rs`.

| Scope | Item | Command | Guard |
|---|---|---|---|
| Player | Details | `Toggle.DetailMode` | has_selection |
| Player | Export Evidence Bundle | `Export.EvidenceBundle` | always |
| Player | Copy Selection | `Copy.Selection` | has_selection |
| HexView | Copy Bytes | `Copy.Bytes` | has_byte_range |
| HexView | Copy Offset | `Copy.Offset` | has_byte_range |
| HexView | Copy Bit Range | `Copy.BitRange` | has_byte_range |
| HexView | Export Evidence Bundle | `Export.EvidenceBundle` | always |
| StreamView | Compare in Display Order | `Set.OrderType.Display` | always |
| StreamView | Compare in Decode Order | `Set.OrderType.Decode` | always |

Guards: `always` (no restriction) · `has_selection` (disabled reason "No selection.") · `has_byte_range`
(disabled reason "No byte range selected."). Policy: disabled items show the reason as a tooltip; guard
evaluation never mutates state.

> Feature/status items live in `docs/specs/features.yaml` (area: ux; legacy ids CTX-01).

## 7. Evidence bundle contract (from `evidence_bundle_diff_contracts.json` + `export_entrypoints.json`)

- **Bundle files:** `bundle_manifest.json`, `env.json`, `version.json`, `selection_state.json`,
  `order_type.json`, `backend_fingerprint.json`, `plugin_versions.json`, `warnings.json`, `screenshots/*`.
- **ABI policy:** forward-compatible — add fields with defaults; no removal without a major bump; renames need
  an alias for ≥2 minor versions.
- **Diff contract:** compare `selection_state`, `order_type`, `backend_fingerprint`, `plugin_versions`,
  `warnings`, `render_snapshots` (optional); ignore `timestamps`, `machine_hostname`, `paths`.
- **Entrypoints (all → `Export.EvidenceBundle`):** Export menu › Evidence Bundle · bottom-bar Export ·
  right-click Export Evidence Bundle · Compare toolbar Export Diff Bundle.

> Feature/status items live in `docs/specs/features.yaml` (area: export; legacy ids EVB-01).

## 8. Additional workspace specs (from `WS_*.md`, gaps only)

Same source family as §2. Only new/gap components are listed; base timeline, base player overlays and base
reference/selection info are omitted. Full layouts in `docs/_import_v14/monster_pack/docs/workspaces/` (local-only).

| Workspace | Gap components | Acceptance tests |
|---|---|---|
| `WS_TIMELINE_TEMPORAL` | Multi-lane mixed timeline: QP-avg line, bpp line, slice/tile-count line, diagnostics-density lane, reorder-mismatch band (PTS≠DTS shading), ref-depth band — all simultaneous with base frame-size bars; cursor snap-to-marker; shift-drag range select → global filter on Metrics/Diagnostics panels | WS01-WS05 |
| `WS_METRICS_QUALITY` | Dedicated metrics workspace: Series+Delta (top) / Distribution+Summary (bottom) split; histogram of metric distribution (whole vs selection, click-to-highlight); worst-N-frames list with jump links; A/B delta lane with threshold-exceed highlighting | WS11-WS14 |
| `WS_PLAYER_SPATIAL` | MiniChartsRow below player surface: block-size-distribution chart + MV-magnitude histogram for current frame, updating on frame change | WS21-WS24 |
| `WS_REFERENCE_DPB` | DPB/buffer inspector tab (occupancy strip/table) and Risk Indicators tab (ref depth metric + missing-ref flags), alongside the reference graph | WS31-WS32 |
| `WS_DIAGNOSTICS_ERROR` | Dedicated diagnostics workspace: severity distribution chart + category histogram + error-burst-detection lane (rolling window count); click burst → auto-select range on Timeline + filter Diagnostics table (tri-sync) | WS41-WS43 |

> Feature/status items live in `docs/specs/features.yaml` (area: ui).

## 9. UI/UX parity checklist (V14 `parity_matrix.seed.json`)

Scoring model (if formalized): weights `information_architecture=0.25, interaction_intent=0.3,
contract_correctness=0.3, evidence_reproducibility=0.15`; severity weights `P0=1.0, P1=0.6, P2=0.3, P3=0.1`.

> Items (IA_TRISYNC_PANELS_PRESENT, CONTRACT_DISPLAY_DECODE_SEPARATION, INTERACTION_CONTEXT_MENU_GUARDS,
> EVIDENCE_ONE_CLICK_BUNDLE, PERF_ENVELOPE_LOD_VIRTUALIZATION) live in `docs/specs/features.yaml` as legacy ids.

Decision (2026-07-31): the context-menu system (§6) and one-click evidence bundle (§7) were promoted into
`DEVELOPMENT_PHASES.md` Phase 7.6, right after Phase 7.5, instead of being deferred to Phase 11.
Perf envelope reference numbers (from V14, hardcoded in `bitvue-engine/src/parity_harness/gates.rs`
`PerfBudgets::default()`): UI frame ≤16.6ms, hit-test ≤1.5ms, overlay render ≤6.0ms, tooltip build ≤0.8ms,
selection propagation ≤2.0ms. Degrade sequence: disable labels → aggregate vectors → downsample heatmap → placeholder.

## 10. Menu structure reference (VQ Analyzer target tree)

Design reference; F-key numbering under Mode is codec-dependent (canonical per-codec mapping:
`VQA_PARITY_SPEC_V3.md` §4.4). Bitvue's actual menus: `bitvue-desktop/electron/nativeMenu.ts` (macOS) and
`frontend/components/TitleBar.tsx` (Win/Linux, items without an action render disabled).
Wiring status per menu: `docs/specs/features.yaml` (area: ui, "… menu" items).

#### File 메뉴
```
File
├── Open Bitstream...          (Ctrl+O)
├── Open Recent ▶
│   └── [최근 파일 목록]
├── Close                      (Ctrl+W)
├── ─────────────────────
├── Extract Frames...          → 프레임 YUV/PNG 추출 다이얼로그
├── Extract NAL/OBU Units...   → 원시 비트스트림 유닛 저장
├── ─────────────────────
└── Exit                       (Alt+F4 / Cmd+Q)
```

#### Mode 메뉴 (코덱별 F키 매핑 — `VQA_PARITY_SPEC_V3.md` §4.4 참조)
```
Mode
├── F1: [코덱 의존 모드 1]
├── ...
├── F12: [코덱 의존 모드 12]
├── ─────────────────────
├── Info Overlays ▶
│   ├── QP Map            (Toggle)
│   ├── Heat Map          (Toggle)
│   ├── MV Heat Map       (Toggle, HEVC)
│   ├── PSNR Overlay      (Toggle)
│   ├── SSIM Overlay      (Toggle)
│   ├── Block Type        (Toggle, AV1/VP9)
│   ├── PU Type           (Toggle, HEVC)
│   ├── MB Type           (Toggle, AVC)
│   ├── Reference Indices (Toggle)
│   └── Efficiency Map    (Toggle, AV1/VP9)
└── Simple Motion         (Toggle)
```

#### View 메뉴
```
View
├── Stream View / Syntax Info / Selection Info / Unit Info (HEX) / Status Panel   (Toggle each)
├── ─────────────────────
├── Stream View Mode ▶  Thumbnails | Buffer/Frame Sizes | B-Pyramid | Metrics (PSNR/SSIM) | Active/DPB References
├── ─────────────────────
├── Y / U / V Component Only   (Y / U / V key) · YUV Combined (reset)
├── ─────────────────────
└── Zoom In (+) · Zoom Out (-) · Fit to Window (0) · Full Screen (F)
```

#### YUVDiff 메뉴
```
YUVDiff
├── Open Debug YUV... / Close Debug YUV
├── Display Mode ▶  Decoded | Debug YUV | Difference (|decoded - debug|) | Amplified Difference
├── Bit Depth ▶     Match Stream | 8 | 10 | 12 | 16-bit (max)
├── Picture Offset...      → 프레임 오프셋 다이얼로그
├── Crop Values...         → 크롭 설정 다이얼로그
├── Auto-reload            (Toggle)
├── Calculate PSNR         → 실시간 PSNR/SSIM 계산
└── Show First Difference  → 첫 불일치 프레임으로 이동
```

#### Options 메뉴
```
Options
├── Color Conversion ▶  ITU-R BT.601 | ITU-R BT.709 (기본값) | ITU-R BT.2020
├── Loop Playback          (Toggle)
├── CPU Optimizations ▶ Auto-detect (default) | SSE2 only | SSSE3 | SSE4.1 | AVX2
├── HEVC Extensions ▶   RExt | SCC | SHVC
├── Digest Calculation ▶ As in Bitstream | Always Calculate | Skip
├── VVC Options ▶       Dynamic Selection Info | Detail Popup Windows
└── JPEG XS Options ▶   Reference CFA Pattern...
```

#### Help 메뉴
```
Help
├── User Guide (F1 — 별도 창)
├── Keyboard Shortcuts...
├── About VQ Analyzer...
└── Activation... / Deactivate License / License Server...   (commercial-only; N/A for Bitvue)
```

## 11. Overlay color-scale reference (moved from `VQA_PARITY_SPEC_V3.md` 부록 A)

### Jet Colormap (QP Map, Heat Map 기본)
```
value: 0.0  → #0000FF (파랑)
value: 0.25 → #00FFFF (시안)
value: 0.5  → #00FF00 (초록)
value: 0.75 → #FFFF00 (노랑)
value: 1.0  → #FF0000 (빨강)
```

### MV 크기 히트맵
```
크기 0 (정지) → 검정 #000000
크기 소 → 파랑 #0000FF
크기 중 → 초록 #00FF00
크기 대 → 빨강 #FF0000
```

### HEVC 루프 필터 경계 강도
```
BS=0 → 표시 안 함
BS=1 → 파랑 (약한 필터)
BS=2 → 빨강 (강한 필터)
```

### SAO 타입
```
Edge Filter → 파랑 #4488FF
Band Filter → 빨강 #FF4444
None → 회색 #888888
```

### AV1 Loop Restoration
```
WIENER → 파랑 #4488FF
SGRPROJ (Self-guided) → 초록 #44BB44
NONE → 회색 #888888
```

## 12. Wireframe reference (screens W0-W5, from `UI_WIREFRAMES.md`)

W0 Single Stream (default 4-region layout: Toolbar/Tree/Timeline+Player+Charts/Inspectors+StatusBar), W1 Stream
Inspect Focus, W2 Timeline+Thumbnails, W3 Player+Overlay Settings, W4 Dual View (side-by-side A/B with
SyncController — precedent for §2), W5 Debug YUV+Maps. Full ASCII diagrams in
`docs/_import_v14/monster_pack/docs/UI_WIREFRAMES.md` (gitignored, local-only); pull back in for a redesign pass.

## 13. Onboarding & explainability (from `ux/ONBOARDING_FIRST_5_MIN.md` + `ux/EXPLAINABILITY_HINTS.md`)

**First-5-minutes flow** (candidate Help-menu walkthrough). Steps 3-5 (Worst Frames list, Session Evidence
export, Regression Guard) depend on the Insight Feed / Regression Guard differentiators in `DEVELOPMENT_PHASES.md`'s
Future Differentiators appendix:

| Step | Workspace | Action |
|---|---|---|
| 1. Find suspect region | Timeline | Enable frame-size + markers (+QP/bpp); drag-select anomalous range |
| 2. Inspect spatial cause | Player | Jump to frame (Enter/dbl-click); toggle QP / MV / Partition; hover values, click block |
| 3. Confirm | Metrics | Whole-stream vs selection histogram; Worst Frames list |
| 4. Diagnose & export | Diagnostics | Click error burst → auto-select range → export Session Evidence |
| 5. Compare (optional) | Compare A/B | Load Stream B; timelines + delta; Regression Guard rule suggestion |

**Explainability micro-copy** (legend/tooltip hint text, copy candidates):

| Overlay | Hint text |
|---|---|
| QP Heatmap | "Auto scale: min/max from current frame" / "Fixed scale: 0..63" / "Heatmap resolution: Quarter/Half/Full affects speed" |
| MV Overlay | "Sampling active (zoomed out)" / "Vectors shown in px (qpel/4)" / "L0/L1 toggle affects reference list" |
| Partition/Grid | "Scaffold grid shown when partition data unavailable" / "Grid decimated when zoomed out" |
| Diff Heatmap | "Abs diff: \|A-B\|" / "Signed diff: A-B" / "Metric diff: per-block delta" |
| Timeline | "Markers never dropped; clustered when dense" / "Global cursor synchronized across workspaces" |

> Feature/status items (walkthrough, hint copy) live in `docs/specs/features.yaml` (area: ux).

## 14. Interaction & edge-case rules (from `UI_INTERACTION_RULEBOOK.md` + `EDGE_CASES_AND_DEGRADE_BEHAVIOR.md`)

Selection precedence Block > Point > Range > Marker (implemented in `crates/bitvue-engine/src/selection.rs`).
Rules beyond what §3-§5 cover (target behaviour; status per rule is in features.yaml):

| Rule | Detail |
|---|---|
| Graph viewport zoom bounds | `[fit*0.5 .. 16x]`; world bounds clamped to `layout_bbox * 1.5`; Fit/Home always available |
| Node/edge density cap | Reference-graph-style views switch to summary mode + banner when node/edge count exceeds cap |
| Overlay draw order | Selection highlight always drawn last (topmost), above all overlay layers |
| PTS quality degrade | VFR/missing/duplicate PTS → primary timeline axis falls back to `display_idx`; badge shows PTS quality OK/WARN/BAD |
| Indexing-in-progress | Frame jumps allowed only within already-indexed range; out-of-range shows "Index building…" (no queueing) |
| Overlay partial coverage | Legend shows coverage %; auto-disable overlay with reason if coverage < 20% |
| Overlay missing data | Transparent render + legend note (never a blank/broken overlay) |
| Compare alignment confidence | High/Med/Low; Low confidence → diff view disabled by default, manual offset still allowed (code: disables on Low **and** >30% gaps) |
| Compare resolution mismatch | Diff disabled (code: >5% mismatch → incompatible); optional explicit resample marked experimental |
| Async backpressure | Late results discarded if `request_id` mismatches current; non-current jobs cancelled during scrub; in-flight cap strictly enforced (`DEVELOPMENT_PHASES.md` Architecture appendix) |
| Non-negotiable | Never freeze UI; never show a blank panel — every failure state has a fallback + explanation |

> Feature/status items live in `docs/specs/features.yaml` (areas: ux, compare, infra; legacy ids EDGE-01..06).

## 15. Source note

§10/§11 moved from `VQA_PARITY_SPEC_V3.md` §2.2 and 부록 A (2026-07-31). Everything else distilled from
`docs/_import_v14/` (gitignored; pruned to `parity_harness/` after mining; original zips in `~/Downloads/`):
COMPETITOR_PARITY_MATRIX, WS_*.md, MOUSE_INTERACTION_SPEC, TOOLTIP_SPEC, ZOOM_POLICY,
VISUALIZATION_COMPLETENESS_CHECKLIST, UI_WIREFRAMES, competitor_targets.json, parity_matrix.seed.json,
parity_harness/*.json, COMPARE_ALIGNMENT_POLICY, ux/*, ux_rules/*, edge_cases/*. Checked but not distilled:
VIZ_DATA_CATALOG, DATA_VISUALIZATION_SPEC, VISUALIZATION_MIX_MATRIX (covered by §1),
semantic_probe/render_snapshot contracts (test plumbing). `critical_contracts/*.md` are implemented as
`bitvue-engine` modules (e.g. `selection.rs`, `coordinate_transform.rs`) and tracked in `DEVELOPMENT_PHASES.md`'s
Architecture appendix, as are perf specs, product policies and the four differentiator specs.
