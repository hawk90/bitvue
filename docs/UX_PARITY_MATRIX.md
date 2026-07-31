# Bitvue — UI/UX Parity Matrix
> 2026-07-31. Compacted from `docs/_import_v14/` (gitignored raw import — VQ_Probe/StreamEye web research +
> a prior "V14 monster pack" design spec found in ~/Downloads). Companion to `VQA_PARITY_SPEC_V3.md` (backend/codec
> feature parity), `PARITY_CHECKLIST.md` (implementation tracking), `COMPETITOR_FEATURE_MATRIX.md` (full
> per-product feature matrix), and `DEVELOPMENT_PHASES.md` (Phase 0-12 implementation roadmap). This doc is
> UX/interaction parity only.
> Status column = current Bitvue implementation state, unverified unless checked against code — confirm before relying on it.
> **V14 pack age caveat:** source files are dated 2026-01-09..01-15, ~6-7 months before this doc. It reads as a
> design blueprint (locked layout/interaction/tooltip contracts + its own self-graded completeness checklist),
> not a verification of what Bitvue actually ships today — its `[x]` marks describe the spec being locked, not
> code being built. Treat every row here as a design target to re-validate against current code/UX before acting
> on it, especially anything not independently cross-checked against `PARITY_CHECKLIST.md` or a code grep.

## 0. Competitor UX reference targets

| id | product | vendor | reference type | priority | note |
|---|---|---|---|---|---|
| vq_analyzer | VQ Analyzer | ViCueSoft | user guide + screenshots | 1 | primary nav/mode/panel/tri-sync UX reference |
| elecard_streameye | StreamEye | Elecard | PDF guide + screenshots | 2 | reporting/export/error-viz discipline reference |
| intel_vpa | Video Pro Analyzer | Intel | PDF guide + screenshots | 3 | secondary diagnostics-pattern reference — **not yet web-researched, backlog** |

Rule: match *intent/behavior*, not pixel-for-pixel visual design.

---

## 1. Data/Viz domain superset checklist (VQ Analyzer + StreamEye baseline)

One row = one measurable data item competitors expose. Bitvue must have: a default viz, an export path (or explicit
"why not"), and a jump-to affordance. Status cross-checked against `PARITY_CHECKLIST.md` Layers 1-6 where possible.

| Domain | Item | Bitvue status |
|---|---|---|
| Stream/Container | Tracks, PID/program info, packetization | ✅ (container parsers) |
| Stream/Container | GOP structure, access units, OBU/NAL lists | ✅ (SyntaxPanel) |
| Stream/Container | Timestamps, duration, declared vs derived frame rate | ⚠️ unverified |
| Stream/Container | Compliance/syntax/CRC/missing-ref error list | ⚠️ unverified |
| Bitstream Syntax | Full syntax tree + conditions | ✅ (SyntaxPanel) |
| Bitstream Syntax | Raw offset+bit-range mapping | ✅ (HexViewTab, `get_frame_hex_data`) |
| Bitstream Syntax | SEI/metadata (HDR, mastering, color info) | ⚠️ unverified |
| Frame Temporal | Frame size/type/QP stats, slice/tile counts | ✅ (FrameSizesView) |
| Frame Temporal | Complexity proxies (bits/pixel, motion magnitude) | ⚠️ unverified |
| Frame Temporal | Scene-change markers | ❌ not tracked anywhere in existing docs |
| Frame Temporal | Reference depth + reordering indicators | ⚠️ unverified |
| Decoder-Path | DPB / ref picture sets | ✅ (IA-04 selection info) |
| Decoder-Path | HRD/CPB fullness + constraints | ⚠️ spec'd (§4.1) — build status unverified |
| Decoder-Path | Decode errors + concealment markers | ⚠️ unverified |
| Spatial Overlay | Block partitions (CU/PU/TU) | ✅ (OV-03) |
| Spatial Overlay | MV vectors (L0/L1) | ✅ (OV-02) |
| Spatial Overlay | QP heatmap | ✅ (OV-01) |
| Spatial Overlay | Per-block metric map (PSNR/SSIM/VMAF) | ❌ CMP-06/07 (new, this session) |
| Quality/Compare | PSNR/SSIM series+summary | ⚠️ planned (§4.7), build status unverified |
| Quality/Compare | VMAF series+summary | ❌ CMP-06 (new, this session) |
| Quality/Compare | RD curves / BD-rate | ❌ CMP-05 (new, this session) |
| Quality/Compare | A/B dual-stream compare (sync playback + diff maps) | ❌ CMP-01..04 (new, this session) — see §2 below |

Full per-product source breakdown (which competitor has which item) lives in `COMPETITOR_FEATURE_MATRIX.md`.

---

## 2. Compare-AB workspace spec (from V14 pack `WS_COMPARE_AB.md`)

Directly implements CMP-01..05 in `PARITY_CHECKLIST.md` Layer 6. Locked layout, not yet built in Bitvue.

**User questions it must answer:** where do two streams differ (quality/structure/errors)? what's the root cause? can the delta become a CI regression guard?

**Grid (2-col):**
- Left: stacked timelines — Timeline A (180px) / Timeline B (180px) / Delta Lane (120px)
- Right: Compare Controls + Violations list (min 320px)
- Optional bottom: Player Compare strip (240px) or opens Player workspace side-by-side

**Component tree:** `CompareWorkspace` → `CompareHeader`(sync mode, metric selector, export) + `Grid2Col`(`TimelinesCol`[TimelineA, TimelineB, DeltaTimeline], `ControlsCol`[RegressionRulesPanel, ViolationsList, CreateRuleFromSelectionButton]) + optional `PlayerCompareStrip`

**Layers (simultaneous):** (A) dual timeline + delta lane, shared cursor, sync modes Off/Playhead/Full (B) dual metrics + delta + threshold-exceed regions (C) player compare: side-by-side or diff heatmap (D) regression-guard mini panel (active rules + violations)

**Interaction:** cursor move syncs A/B · click delta-exceed region → jump to Player compare at that frame · right-click → create regression rule from selection

**Tooltip (delta target):** A value, B value, delta, rank-among-worst

**Export:** compare report (delta summary + worst frames + evidence snapshots) · regression rules JSON

**Acceptance tests:** WS51 dual timeline shows delta lane · WS52 create regression rule from selected range · WS53 violation list jumps to evidence

### 2.1 Compare alignment axes (from `compare_alignment_contracts.json` + `COMPARE_ALIGNMENT_POLICY.md`)

Correctness rule underlying CMP-01..05 — how two streams get mapped frame-to-frame before any diff/delta is computed.

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

**Mismatch handling:** reorder detected → show alignment warning + offer axis switch · drop detected → mark missing frames with reason · config mismatch → block compare + surface detailed reason · resolution mismatch beyond tolerance → disable diff overlays, show side-by-side with scale indicator instead.

**Evidence must record:** chosen axis, mismatch events, order_type. Ties to CMP-01 (sync playback) and CMP-04 (Find First Difference) — both must use this axis-resolution policy, not ad-hoc frame-index matching.

---

## 3. Per-panel interaction contract (compressed from `MOUSE_INTERACTION_SPEC.md`)

| Panel | Hover | Click | Drag | Wheel | Ctrl/Cmd+wheel | Shift+drag | Context menu |
|---|---|---|---|---|---|---|---|
| Timeline/Bars | tooltip (bar/marker) | select frame; dblclick=zoom-in @cursor | pan | semantic zoom @cursor | aggressive zoom | range-select [start..end] | Add/Remove Bookmark, Export Range CSV, Jump Hex/Tree/Player, Toggle marker kinds |
| Metrics/HRD plots | nearest-point tooltip | select nearest frame; dblclick=reset zoom | pan | semantic zoom @cursor | — | zoom-box or range-select (pick one, document) | Export series CSV, Add threshold line, Copy value |
| Reference graph | node/edge tooltip | node select→SelectFrame; Ctrl+click=pin | pan bg / move node | visual zoom | — | — | Center on selection, Show deps/refs only, Export graph JSON |
| Player surface | pixel/block tooltip | click block→spatialBlock select | pan (zoom>Fit); Alt+drag=scrub frames | visual zoom (optional) | — | — | Copy pixel/block info, Export frame image, Export overlay snapshot |
| Hex/Bit view | byte/bit tooltip+mapping | set caret; dblclick=byte, triple=unit span | select range→bitRange | vertical scroll | — | Shift+wheel=horiz scroll | Copy offset/bytes/bitrange, Jump to Syntax node, Bookmark |
| Trees/Tables | row tooltip | select row; Ctrl+click=multi/pin | reorder columns (optional) | scroll | — | — | copy, export, jump |

Global modifier convention: **Shift**=range/multi-select, **Ctrl/Cmd**=additive select/pin, **Alt/Option**=alternate mode (scrub/fine-adjust). Bitvue keyboard shortcuts already documented in `PARITY_CHECKLIST.md` Layer 4 (Keyboard Shortcut Parity) — cross-check modifier consistency against this table when implementing.

---

## 4. Per-panel tooltip field contract (compressed from `TOOLTIP_SPEC.md`)

Global rules: appear ≤150ms (debounced), never cover cursor target, stable while hovering same target, support multi-line + monospace blocks + copy buttons, always show units, show "N/A" explicitly for missing fields, hit-test dense targets by logical ID not pixel proximity.

| Panel/target | Required fields | Required actions |
|---|---|---|
| Timeline bar/marker | frame idx (+GOP/IDR), PTS/DTS + derived time, size (bytes+bits), type, marker flags, decode status+last error | Copy Frame Ref, Copy PTS, Jump Hex/Tree |
| Metrics plot point | frame idx+time, series name, value+unit, delta vs prev (optional) | Pin (v2), Copy "metric=value @ frame" |
| HRD/buffer point | time/frame idx, fullness (bits), CPB size, under/overflow risk state | Copy time+fullness, Jump to nearest frame |
| Tree row | Container/Track/Frame/Unit breadcrumb, offset(hex)+size, unit type, flags, parsed?, diagnostic id if error | Copy Offset, Copy Path, Bookmark |
| Syntax node | field name+type, decoded value, bit range, raw bits preview, condition eval | Copy Field, Copy BitRange, Copy Raw Bits |
| Hex/bit cell | offset(hex), byte(hex+dec), ASCII, bit index (local+global), mapped syntax field (best-effort) | Copy Offset, Copy BitRange, Copy Byte |
| Player surface | frame idx, pixel (x,y), luma/chroma values, block id/partition info, active overlays list | Pin Sample (optional), Copy Pixel |
| Diagnostics row | severity, category, offset+frame/unit refs, full message, root-cause chain (collapsible) | Copy Diagnostic, Copy Offset, Jump |

Nice-to-have (not required v1): pinnable tooltip → small inspector box; architecture shouldn't block it later.

---

## 5. Zoom/pan policy per panel (from `ZOOM_POLICY.md`)

| Panel | Pan | Zoom | Type | Notes |
|---|---|---|---|---|
| Timeline/Bars | Yes | Yes | Semantic (LOD buckets ↔ per-frame) | marker glyph + axis font fixed-density |
| Metrics/HRD | Yes | Yes | Semantic (downsample ↔ full-res) | legend/axis fixed-density |
| Thumbnails strip | scroll | optional | Visual (size-adaptive) | adapts to available height regardless |
| Player surface | Yes (zoom>Fit) | Yes | Visual | Fit/100%/200% minimum; overlays scale with zoom |
| Overlay vectors/grids | (follows player) | (follows player) | Mixed | geometry scales w/ player zoom; line/arrowhead fixed-density |
| Hex/Bit view | scroll | **No** | Fixed-density | never chart-style zoom; optional font-size pref only |
| Syntax tree/diagnostics table | scroll | **No** | Fixed-density | optional global UI-scale setting only |
| Reference graph | Yes | Yes | Visual (canvas) | labels abbreviate when zoomed out |

---

## 6. Context menu & guard contract (from `context_menus.json` + `guard_rules.json`)

Previously flagged as a gap with "no equivalent found" — the V14 harness actually defines a concrete starter
contract (3 scopes, 7 items, 3 guards). Not yet implemented in Bitvue; this is the target shape.

| Scope | Item | Command | Guard |
|---|---|---|---|
| Player | Details | `Toggle.DetailMode` | has_selection |
| Player | Export Evidence Bundle | `Export.EvidenceBundle` | always |
| Player | Copy Selection | `Copy.Selection` | has_selection |
| HexView | Copy Bytes | `Copy.Bytes` | has_byte_range |
| HexView | Export Evidence Bundle | `Export.EvidenceBundle` | always |
| StreamView | Compare in Display Order | `Set.OrderType.Display` | always |
| StreamView | Compare in Decode Order | `Set.OrderType.Decode` | always |

**Guards:** `always` (no restriction) · `has_selection` (disabled reason: "No selection.") · `has_byte_range`
(disabled reason: "No byte range selected."). Policy: disabled items must show the reason as a tooltip; guard
evaluation must never mutate state. This is a starter set (marked "expand via screenshot-driven refinement" in
the source) — treat per-panel rows in §3 above as the fuller target once built.

---

## 7. Evidence bundle export contract (from `evidence_bundle_diff_contracts.json` + `export_entrypoints.json`)

Previously flagged as a gap ("no equivalent in Bitvue — closest is `bitvue export --json`"). The harness defines
a concrete one-click bundle contract; still a real gap (not built), but now spec'd instead of vague.

**Bundle must include:** `bundle_manifest.json`, `env.json`, `version.json`, `selection_state.json`,
`order_type.json`, `backend_fingerprint.json`, `plugin_versions.json`, `warnings.json`, `screenshots/*`.

**ABI compatibility policy:** forward-compatible — fields may be added with defaults; fields must not be removed
without a major version bump; renames require an alias mapping for ≥2 minor versions.

**Diff contract:** compare fields = `selection_state`, `order_type`, `backend_fingerprint`, `plugin_versions`,
`warnings`, `render_snapshots` (optional). Ignore fields (excluded from diff): `timestamps`, `machine_hostname`, `paths`.

**Required entrypoints (no detours):**

| Scope | Path | Command |
|---|---|---|
| MainMenu | File > Export > Evidence Bundle | `Export.EvidenceBundle` |
| MainPanel | BottomBar > Export | `Export.EvidenceBundle` |
| ContextMenu | RightClick > Export Evidence Bundle | `Export.EvidenceBundle` |
| CompareWorkspace | Toolbar > Export Diff Bundle | `Export.EvidenceBundle` |

Export must include explicit order type and backend/plugin fingerprints; the ABI rules above apply to all of them.

---

## 8. Additional workspace specs (gaps only, from `WS_*.md`)

Same source family as §2's Compare-AB (`WS_COMPARE_AB.md`). Only new/gap components are listed — anything
already implied ✅ by `PARITY_CHECKLIST.md` Layers 1-6 (e.g. base timeline, base player overlays, base
reference/selection info) is omitted. Full layout/component-tree detail lives in the source files under
`docs/_import_v14/monster_pack/docs/workspaces/` (gitignored, local-only).

| Workspace | New/gap components not yet in Bitvue | Acceptance tests (source IDs) |
|---|---|---|
| `WS_TIMELINE_TEMPORAL` | Multi-lane mixed timeline: QP-avg line, bpp line, slice/tile-count line, diagnostics-density lane, reorder-mismatch band (PTS≠DTS shading), ref-depth band — all simultaneous with base frame-size bars; cursor snap-to-marker; shift-drag range select → global filter on Metrics/Diagnostics panels | WS01-WS05 |
| `WS_METRICS_QUALITY` | Dedicated metrics workspace: Series+Delta (top) / Distribution+Summary (bottom) split; histogram of metric distribution (whole vs selection, click-to-highlight); worst-N-frames list with jump links; A/B delta lane with threshold-exceed highlighting | WS11-WS14 |
| `WS_PLAYER_SPATIAL` | MiniChartsRow below player surface: block-size-distribution chart + MV-magnitude histogram for current frame, updating on frame change | WS21-WS24 |
| `WS_REFERENCE_DPB` | DPB/buffer inspector tab (occupancy strip/table) and Risk Indicators tab (ref depth metric + missing-ref flags), alongside the reference graph already noted in §3 above | WS31-WS32 |
| `WS_DIAGNOSTICS_ERROR` | Dedicated diagnostics workspace: severity distribution chart + category histogram + error-burst-detection lane (rolling window count); click burst → auto-select range on Timeline + filter Diagnostics table (tri-sync) | WS41-WS43 |

---

## 9. UI/UX parity checklist (P0/P1 items, from V14 `parity_matrix.seed.json`)

Scoring model (for future use if formalized): weights `information_architecture=0.25, interaction_intent=0.3, contract_correctness=0.3, evidence_reproducibility=0.15`; severity weights `P0=1.0, P1=0.6, P2=0.3, P3=0.1`.

| id | category | severity | question | Bitvue status |
|---|---|---|---|---|
| IA_TRISYNC_PANELS_PRESENT | IA | P0 | Are Tree⇄Syntax⇄Hex⇄Charts⇄Player all present and reachable without detours, with selection propagating across all of them? | ⚠️ unverified — panels exist (IA-01..05 in PARITY_CHECKLIST) but full N-way tri-sync propagation not explicitly tested |
| CONTRACT_DISPLAY_DECODE_SEPARATION | Contract | P0 | Is display-order vs decode-order separation enforced everywhere (UI logic, evidence exports, compare), no implicit mixing? | ⚠️ unverified — concrete rule (per `FRAME_IDENTITY_CONTRACT.md`): display_idx is primary everywhere (Timeline/Player/Diagnostics/Metrics/Compare), decode_idx is internal-only, PTS/DTS mismatch shown only via a dedicated band. `-display_order` exists in VQ Analyzer CLI; Bitvue CLI parity for this flag unconfirmed (see `COMPETITOR_FEATURE_MATRIX.md` §2) |
| INTERACTION_CONTEXT_MENU_GUARDS | Interaction | P1 | Do right-click context menus appear where expected, with disabled items showing a discoverable reason? | ⚠️ contract now defined in §6 above (3 scopes, 7 items, guard+disabled-reason policy) — not yet implemented in Bitvue |
| EVIDENCE_ONE_CLICK_BUNDLE | Evidence | P0 | One-click export bundling screenshots+evidence+version/env+selection-state+order-type+backend fingerprints? | ⚠️ contract now defined in §7 above (required files, ABI policy, 4 entrypoints) — not yet implemented; closest existing is `bitvue export --json` (single-frame/stream, not a full bundle) |
| PERF_ENVELOPE_LOD_VIRTUALIZATION | Performance | P1 | Does the UI stay responsive on large streams via virtualization+LOD, no stale-async or cache-invalidation bugs? | ⚠️ Phase 10 (perf) claimed done in git log — LOD/virtualization specifics unverified against this bar. Concrete budget targets from `perf_budget_and_instrumentation.json`: UI frame ≤16.6ms, hit-test ≤1.5ms, overlay render ≤6.0ms, tooltip build ≤0.8ms, selection propagation ≤2.0ms; degrade steps on breach: disable labels → aggregate vectors → downsample heatmap → placeholder with reason |

**Layer 7 candidate:** context-menu system (§6) and one-click evidence-bundle export (§7) are both real gaps
with concrete contracts now defined above but no current Bitvue implementation — recommend adding as Layer 7
in `PARITY_CHECKLIST.md` once scoped.

---

## 10. Menu structure reference (moved from `VQA_PARITY_SPEC_V3.md` §2.2, 2026-07-31)

VQ Analyzer's target menu tree (ASCII, per top-level menu). This is UI/UX design reference material, a
natural companion to the wireframes in §12 below — moved here rather than living in the backend/codec spec.
F-key numbering under Mode ▶ is codec-dependent; canonical per-codec F-key assignment lives in
`VQA_PARITY_SPEC_V3.md` §4.4.

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
├── F2: [코덱 의존 모드 2]
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
├── Stream View            (Toggle)
├── Syntax Info            (Toggle)
├── Selection Info         (Toggle)
├── Unit Info / HEX View   (Toggle)
├── Status Panel           (Toggle)
├── ─────────────────────
├── Stream View Mode ▶
│   ├── Thumbnails
│   ├── Buffer/Frame Sizes
│   ├── B-Pyramid (Hierarchy)
│   ├── Metrics (PSNR/SSIM)
│   └── Active/DPB References
├── ─────────────────────
├── Y Component Only       (Y key)
├── U Component Only       (U key)
├── V Component Only       (V key)
├── YUV Combined           (reset)
├── ─────────────────────
├── Zoom In                (+)
├── Zoom Out               (-)
├── Fit to Window          (0)
└── Full Screen            (F)
```

#### YUVDiff 메뉴
```
YUVDiff
├── Open Debug YUV...
├── Close Debug YUV
├── ─────────────────────
├── Display Mode ▶
│   ├── Decoded (bitstream only)
│   ├── Debug YUV
│   ├── Difference (|decoded - debug|)
│   └── Amplified Difference
├── ─────────────────────
├── Bit Depth ▶
│   ├── Match Stream
│   ├── 8-bit
│   ├── 10-bit
│   ├── 12-bit
│   └── 16-bit (max)
├── ─────────────────────
├── Picture Offset...      → 프레임 오프셋 다이얼로그
├── Crop Values...         → 크롭 설정 다이얼로그
├── Auto-reload            (Toggle)
├── ─────────────────────
├── Calculate PSNR         → 실시간 PSNR/SSIM 계산
└── Show First Difference  → 첫 불일치 프레임으로 이동
```

#### Options 메뉴
```
Options
├── Color Conversion ▶
│   ├── ITU-R BT.601
│   ├── ITU-R BT.709      (기본값)
│   └── ITU-R BT.2020
├── ─────────────────────
├── Loop Playback          (Toggle)
├── ─────────────────────
├── CPU Optimizations ▶
│   ├── Auto-detect (default)
│   ├── SSE2 only
│   ├── SSSE3
│   ├── SSE4.1
│   └── AVX2
├── ─────────────────────
├── HEVC Extensions ▶
│   ├── RExt (Range Extensions)
│   ├── SCC (Screen Content Coding)
│   └── SHVC (Scalable)
├── ─────────────────────
├── Digest Calculation ▶
│   ├── As in Bitstream
│   ├── Always Calculate
│   └── Skip
├── ─────────────────────
├── VVC Options ▶
│   ├── Dynamic Selection Info
│   └── Detail Popup Windows
└── JPEG XS Options ▶
    └── Reference CFA Pattern...
```

#### Help 메뉴
```
Help
├── User Guide (F1 — 별도 창)
├── Keyboard Shortcuts...
├── ─────────────────────
├── About VQ Analyzer...
├── ─────────────────────
├── Activation...
├── Deactivate License
└── License Server...
```

---

## 11. Overlay color-scale reference (moved from `VQA_PARITY_SPEC_V3.md` 부록 A, 2026-07-31)

Color-scale contracts for the spatial overlay renderers (QP Map, Heat Map, MV Heat, HEVC loop-filter boundary
strength, SAO type, AV1 Loop Restoration). Moved here as UI/UX design reference, companion to §1's Spatial
Overlay rows and the per-panel tooltip/interaction contracts in §3-§5.

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

---

## 12. Wireframe reference (screens W0-W5, from `UI_WIREFRAMES.md`)

ASCII wireframes for: W0 Single Stream (default 4-region layout: Toolbar/Tree/Timeline+Player+Charts/Inspectors+StatusBar), W1 Stream Inspect Focus, W2 Timeline+Thumbnails, W3 Player+Overlay Settings, W4 Dual View (side-by-side A/B with SyncController), W5 Debug YUV+Maps. Full ASCII diagrams preserved in `docs/_import_v14/monster_pack/docs/UI_WIREFRAMES.md` (gitignored, local-only) — pull back in in full if a redesign pass is scheduled; not reproduced here to keep this doc dense. W4 layout is the direct visual precedent for §2's Compare-AB workspace.

---

## 13. Onboarding & explainability (from `ux/ONBOARDING_FIRST_5_MIN.md` + `ux/EXPLAINABILITY_HINTS.md`, mined 2026-07-31)

**First-5-minutes flow** (candidate Help-menu walkthrough — Worst-Frames-list and Regression-Guard steps depend on
the Insight Feed / Regression Guard differentiators in `DEVELOPMENT_PHASES.md`'s Future Differentiators appendix,
not yet built):

| Step | Workspace | Action |
|---|---|---|
| 1. Find suspect region | Timeline | Enable frame-size + markers (+QP/bpp) overlays; drag-select anomalous range |
| 2. Inspect spatial cause | Player | Jump to selected frame (Enter/dbl-click); toggle QP Heatmap / MV / Partition; hover for values, click to select block |
| 3. Confirm | Metrics | Compare whole-stream vs selection histogram; jump via Worst Frames list *(not yet built)* |
| 4. Diagnose & export | Diagnostics | Click error burst → auto-select range → export Session Evidence *(not yet built)* |
| 5. Compare (optional) | Compare A/B | Load Stream B; timelines + delta; Regression Guard rule suggestion *(not yet built)* |

**Explainability micro-copy** (overlay legend/tooltip hint text, direct copy candidates):

| Overlay | Hint text |
|---|---|
| QP Heatmap | "Auto scale: min/max from current frame" / "Fixed scale: 0..63" / "Heatmap resolution: Quarter/Half/Full affects speed" |
| MV Overlay | "Sampling active (zoomed out)" / "Vectors shown in px (qpel/4)" / "L0/L1 toggle affects reference list" |
| Partition/Grid | "Scaffold grid shown when partition data unavailable" / "Grid decimated when zoomed out" |
| Diff Heatmap | "Abs diff: \|A-B\|" / "Signed diff: A-B" / "Metric diff: per-block delta" (ties to CMP-03, `PARITY_CHECKLIST.md` Layer 6) |
| Timeline | "Markers never dropped; clustered when dense" / "Global cursor synchronized across workspaces" |

---

## 14. Interaction & edge-case rules (from `ux_rules/UI_INTERACTION_RULEBOOK.md` + `edge_cases/EDGE_CASES_AND_DEGRADE_BEHAVIOR.md`, mined 2026-07-31)

New items beyond what §3-§5 (Mouse/Tooltip/Zoom, already mined) cover — cross-checked, `SelectionState`/`TemporalSelection`
precedence in `crates/bitvue-core/src/selection.rs` already matches the rulebook's Block>Point>Range>Marker order.

| Rule | Detail |
|---|---|
| Graph viewport zoom bounds | `[fit*0.5 .. 16x]`; world bounds clamped to `layout_bbox * 1.5`; Fit/Home always available |
| Node/edge density cap | Reference-graph-style views switch to summary mode + banner when node/edge count exceeds cap |
| Overlay draw order | Selection highlight always drawn last (topmost), above all overlay layers |
| PTS quality degrade | VFR/missing/duplicate PTS → primary timeline axis falls back to `display_idx`; badge shows PTS quality OK/WARN/BAD |
| Indexing-in-progress | Frame jumps allowed only within already-indexed range; out-of-range shows "Index building…" (no queueing) |
| Overlay partial coverage | Legend shows coverage %; auto-disable overlay with reason if coverage < 20% |
| Overlay missing data | Transparent render + legend note (never a blank/broken overlay) |
| Compare alignment confidence | High/Med/Low; Low confidence → diff view disabled by default, manual offset still allowed |
| Compare resolution mismatch | Diff disabled; optional explicit resample marked experimental |
| Async backpressure | Late results discarded if `request_id` mismatches current; non-current jobs cancelled during scrub; in-flight cap strictly enforced (see `DEVELOPMENT_PHASES.md` Architecture appendix "Async Backpressure" row) |
| Non-negotiable | Never freeze UI; never show a blank panel — every failure state has a fallback + explanation |

---

## 15. Source note

§10 (menu structure) and §11 (overlay color-scale reference) are not from the V14 pack — they were moved from
`VQA_PARITY_SPEC_V3.md` §2.2 and 부록 A respectively (2026-07-31 doc-family split), which also shifted the old
§10/§11 (wireframe reference / this source note) down to §12/§13 (and, in the 2026-07-31 mining pass below,
§13/§14 became new content, pushing this source note to §15).

Raw pack lives at `docs/_import_v14/` (gitignored — not committed). Distilled into this doc so far: §0-§5 (from
COMPETITOR_PARITY_MATRIX.md, WS_COMPARE_AB.md, MOUSE_INTERACTION_SPEC.md, TOOLTIP_SPEC.md, ZOOM_POLICY.md,
VISUALIZATION_COMPLETENESS_CHECKLIST.md, UI_WIREFRAMES.md, competitor_targets.json, parity_matrix.seed.json),
§6-§8 (from `parity_harness/context_menus.json`, `guard_rules.json`, `evidence_bundle_diff_contracts.json`,
`export_entrypoints.json`, `compare_alignment_contracts.json`, `COMPARE_ALIGNMENT_POLICY.md`, and the 5
`workspaces/WS_*.md` files), §13-§14 (2026-07-31, second mining pass: `ux/ONBOARDING_FIRST_5_MIN.md`,
`ux/EXPLAINABILITY_HINTS.md`, `ux_rules/UI_INTERACTION_RULEBOOK.md`, `edge_cases/EDGE_CASES_AND_DEGRADE_BEHAVIOR.md`).
Checked but not distilled (no genuinely new content beyond what's above): `VIZ_DATA_CATALOG.md`,
`DATA_VISUALIZATION_SPEC.md`, `VISUALIZATION_MIX_MATRIX.md` (data items already covered in §1),
`semantic_probe_contracts.json` / `render_snapshot_contracts.json` (test-harness plumbing, not feature-facing).

**2026-07-31 correction**: the `critical_contracts/*.md` files this note previously called "generic engineering
rules, not concrete enough to fold in" turned out to be the opposite — they're implemented nearly verbatim as
Rust modules in `crates/bitvue-core/src/` (module doc-comments cite the exact spec filenames: `selection.rs` ↔
`SELECTION_PRECEDENCE_RULES.md`, `coordinate_transform.rs` ↔ `COORDINATE_SYSTEM_CONTRACT.md`, etc.). They were
UI/UX-irrelevant enough to leave out of *this* doc (engineering/backend contracts, not interaction/visual parity)
but are now compactly tracked in `DEVELOPMENT_PHASES.md`'s "Architecture & Correctness Reference" appendix —
see there rather than expecting them here. Also moved there: performance specs (fast-path/quality-path, cache
levels, degradation rules, LOD budgets — enriched into `DEVELOPMENT_PHASES.md` Phase 10), product policies
(error handling folded into `PARITY_CHECKLIST.md`'s release-gate checklist; `VERSIONING_POLICY.md` skipped as
generic), and the four "differentiator" specs (insight feed/session evidence/compliance/regression-guard — see
`DEVELOPMENT_PHASES.md`'s Future Differentiators appendix). Per-viz implementation specs (QP heatmap, MV vectors,
partition grid — already-shipped features, skimmed only; diff heatmap — folded into `DEVELOPMENT_PHASES.md`
Phase 7.5 since it's implementation detail, not UX parity) are also `DEVELOPMENT_PHASES.md`'s territory, not
this doc's. Nothing left unmined as of 2026-07-31 — the raw import at `docs/_import_v14/` was pruned down to
`parity_harness/` only after this pass (see repo `CLAUDE.md`); the original zips remain in `~/Downloads/` as the
ultimate fallback.
