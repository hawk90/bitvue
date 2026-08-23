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
| Stream/Container | Timestamps, duration, declared vs derived frame rate | ❌ not exposed — `FileInfo.fps`/`.duration` (`frontend/types/video.ts`) never populated by any sidecar command; `bitvue-indexer/src/lib.rs:153` hardcodes `duration_ms: None`; `ContainerModel` has no frame-rate field at all. Only `bitvue-cli info` computes a *declared*-only rate (`commands/info.rs:56-59`), println-only, no GUI path, no second "derived" rate anywhere (verified 2026-08-23) |
| Stream/Container | Compliance/syntax/CRC/missing-ref error list | ⚠️ mock only — `DiagnosticsPanel.tsx` mounts with zero props (`App.tsx:152-154`), falls back to client-side-fabricated "mock diagnostics for demonstration" (heuristic on `frame.size`/`ref_frames`); real `DiagnosticsManager` (`bitvue-engine/src/diagnostics.rs`, 856 lines) has zero production callers, constructed only in test files (verified 2026-08-23) |
| Bitstream Syntax | Full syntax tree + conditions | ✅ (SyntaxPanel) |
| Bitstream Syntax | Raw offset+bit-range mapping | ✅ (HexViewTab, `get_frame_hex_data`) |
| Bitstream Syntax | SEI/metadata (HDR, mastering, color info) | ⚠️ parsed, unwired — H.264 SEI MDCV/CLL genuinely parsed (`bitvue-avc/src/sei.rs:221`) into `AvcStreamInfo.sei_messages`, but never read outside that crate; richer `bitvue-engine/src/metadata.rs` (752 lines, `MetadataInspector`) is test-only, zero frontend references; AV1 has no HDR-metadata-OBU parsing at all (verified 2026-08-23) |
| Frame Temporal | Frame size/type/QP stats, slice/tile counts | ✅ (FrameSizesView) |
| Frame Temporal | Complexity proxies (bits/pixel, motion magnitude) | ⚠️ partial — bpp is real and shown as a spatial overlay (`Av1EfficiencyMapRenderer.tsx` ← `energy_extractor.rs`, not an aggregate chart); MV-magnitude histogram (`bitvue-engine/src/player/mini_charts.rs`) computed engine-side, never exposed to UI or CLI (verified 2026-08-23) |
| Frame Temporal | Scene-change markers | ❌ now tracked: `DEVELOPMENT_PHASES.md` Phase 8 Stats 탭 bullet (added 2026-07-31) |
| Frame Temporal | Reference depth + reordering indicators | ⚠️ partial — real display/decode order numbers shown (`DetailsPanel.tsx`/`StatisticsTab.tsx`, from `frame_identity/mod.rs`), but no explicit depth number or reorder-mismatch badge; the reorder-detection engine (`diagnostics_bands.rs`, `insight_feed.rs`) is test-only, unreachable from production (verified 2026-08-23) |
| Decoder-Path | DPB / ref picture sets | ✅ (IA-04 selection info) |
| Decoder-Path | HRD/CPB fullness + constraints | ❌ disconnected mock — `HRDBufferPanel.tsx` renders but simulates against a hardcoded 1MB buffer/30fps, ignoring the real stream; actual declared HRD params are parsed then discarded (`bitvue-avc/src/sps.rs:676-693`, assigned to `_`); complete backend `HrdModel` (`bitvue-engine/src/hrd.rs`, 591 lines) has zero production callers (verified 2026-08-23) |
| Decoder-Path | Decode errors + concealment markers | ❌ absent — no concealment concept anywhere in the repo; `DecodeStatus::Failed` (`player/hud.rs`) only ever set in test code; frontend has only a generic whole-file error dialog, no per-frame marker (verified 2026-08-23) |
| Spatial Overlay | Block partitions (CU/PU/TU) | ✅ (OV-03) |
| Spatial Overlay | MV vectors (L0/L1) | ✅ (OV-02) |
| Spatial Overlay | QP heatmap | ✅ (OV-01) |
| Spatial Overlay | Per-block metric map (PSNR/SSIM/VMAF) | ❌ CMP-06/07 (new, this session) |
| Quality/Compare | PSNR/SSIM series+summary | ⚠️ single-pair only — CLI `--psnr` has a real series+avg/min (`bitvue-cli/decode.rs`); GUI has no stream-wide series/summary panel — historical `RDCurvesPanel.tsx` deleted (`111bb95`), its would-be replacement `QualityMetricsPanel.tsx` is confirmed dead Tauri code (unreachable, calls a nonexistent command); only live path is Compare-workspace single-frame-pair PSNR/SSIM (`YuvDiffPanel.tsx`) (verified 2026-08-23) |
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
| Timeline/Bars | ⚠️ bar-only, real (`Timeline.tsx:180-191`→`TimelineTooltip.tsx`); no marker concept exists at all | ⚠️ click-select real (updates real `SelectionContext`, `Timeline.tsx:240-243`); dblclick-zoom-at-cursor absent — no `onDoubleClick` anywhere | ❌ absent — drag scrubs/selects frames, doesn't pan; only native scrollbar overflow | ✅ **fixed 2026-08-23** (INT-02) — real `scaleX(zoom)` CSS-transform zoom (0.5-3x), Ctrl/Cmd-gated, same technique as `Filmstrip/views/ThumbnailsView.tsx`'s existing wheel-zoom; **visual only**, not the semantic/LOD zoom `.timeline-thumbnails`' unvirtualized-DOM problem would need (deliberately out of scope, flagged in PARITY_CHECKLIST.md) | ✅ same fix as Wheel col — Ctrl/Cmd is the gate | ✅ **fixed 2026-08-23** (INT-02) — real Shift+mousedown→mousemove→mouseup drag commits a `[min,max]` frame range via `setTemporalSelection`'s already-existing "range" type (frontend-only, no backend round trip — see PARITY_CHECKLIST.md note); renders a real `.in-range` highlight | ❌ real menu is only `Export Evidence Bundle` + `Copy Selection` (`context_menu.rs:79-92`) — none of Bookmark/Export-CSV/Jump/marker-toggle exist as commands anywhere (verified 2026-08-23) |
| Metrics/HRD plots | ⚠️ real for HRD only (`HRDBufferPanel.tsx:285-310`); FrameSizesView has only a native per-bar `title`, not a nearest-point implementation. `QualityMetricsPanel.tsx` (dead code, unmounted) is not a plot at all | ⚠️ real exact-bar click for FrameSizesView (`FrameSizesView.tsx:154`→real `SelectionContext` update); HRD canvas has no `onClick` at all; no dblclick-reset-zoom anywhere (no zoom exists to reset) | ❌ absent in both | ❌ absent — no `onWheel` in either | ✅ vacuously true (nothing to gate) | ❌ absent | ❌ absent — no context menu at all; only related UI is `FrameSizesLegend.tsx`, a metric-visibility toggle panel, not Export/threshold/copy (verified 2026-08-23) |
| Reference graph | ❌ N/A — component is a static GOP-card browser, not a node/edge graph at all (no canvas/SVG, `ReferenceGraphPanel.tsx`); the only `title` tooltip is on an unrelated 6×6px overview strip | ❌ click sets local `useState` only, no `SelectionContext` import at all (0 propagation); no `ctrlKey` check anywhere — "pin" doesn't exist | ❌ absent — static CSS flexbox layout, no drag/position state | ❌ absent — no `onWheel`, no zoom state | — | — | ❌ absent — no `onContextMenu` anywhere; only controls are Reload + GOP prev/next buttons |
| Player surface | ✅ **fixed 2026-08-23** (INT-01) — real hover handler (`VideoCanvas.tsx`'s `handleContainerMouseMove`) computes pixel Y/U/V (`resolvePixelValueAtPoint`) and resolved block (`resolveSpatialBlockAtPoint`), rendered by a new `PlayerPixelTooltip`; suppressed while dragging; verified with a real Electron screenshot showing genuine computed values | ✅ **fixed 2026-08-23** (INT-01) — real click-vs-drag detection resolves the clicked coding-unit block (`resolveSpatialBlockAtPoint`, prefers `partition_grid` over `qp_grid`) and dispatches it through `selectSpatialBlock`, no longer dead code; `SelectionInfoPanel` now renders the real selected block's position/size | ⚠️ pan real but unconditional (no "zoom>Fit" gate, no "Fit" level exists at all); Alt+drag scrub absent — no `e.altKey` check anywhere | ⚠️ real, but requires Ctrl/Cmd — `useCanvasInteraction.ts:122-133` called with `requireModifierKey:true` (`YuvViewerPanel/index.tsx:103-108`), so plain wheel is inert | (zoom actually lives here, not the Wheel col — see left) | — | ⚠️ real menu is Details/Export Evidence Bundle/Copy Selection (`export/context_menu.rs:36-55`, matches §6 not this row); **Copy Selection fixed 2026-08-23** (INT-01) — copies the selected block's position/size; Details (`Toggle.DetailMode`) remains a no-op stub |
| Hex/Bit view | ⚠️ native `title` only (`HexViewTab.tsx:303`) — hex offset+value, no ASCII/bit-index/mapped-field | ⚠️ single click sets 1-byte caret+bitRange; no dblclick handler at all (single click already does what "dblclick=byte" describes); triple-click=unit-span is fully absent (verified 2026-08-23) | ✅ **fixed 2026-08-23** (INT-03) — real mousedown→mouseenter→mouseup drag detection (mirrors Player's INT-01 pattern) selects a genuine `[min,max]` multi-byte range, committing one `select_bit_range` round trip on release; a plain click (no movement) still selects exactly 1 byte | ✅ native browser overflow scroll, no custom handler | — | ❌ absent — no wheel handler at all; layout has no horizontal overflow to scroll (16 bytes/line always fits) | ⚠️ real menu is `Copy Bytes`/`Copy Offset`/`Copy Bit Range` + `Export Evidence Bundle` (`export/context_menu.rs:56-68`) — no Jump to Syntax, no Bookmark (scoped, see PARITY_CHECKLIST.md's dedicated context-menu pass: Jump needs a new cross-panel tab-focus channel, Bookmark needs a data-model change). `Copy Bytes` copies the whole selected range (was 1-byte-only); **Copy Offset/Copy Bit Range added 2026-08-23** (verified 2026-08-23) |
| Trees/Tables | ❌ empty for real data — syntax tree's `description` is always `undefined` for real backend data (`FrameSyntaxTab.tsx:47-60`); StreamTreePanel's title is a redundant copy of the visible label; DiagnosticsPanel has none at all | ⚠️ diverges sharply by component: DiagnosticsPanel select is real (local state); syntax tree row has **no click handler at all**; StreamTreePanel's click handler exists but is a no-op in production — `App.tsx:137-139` mounts it with zero props, so `onUnitSelect` is always `undefined`. Ctrl+click multi/pin: 0 grep hits anywhere | ✅ correctly absent (doc marks optional) | ✅ scroll (native/manual-virtualization, consistent across all 3) | — | — | ⚠️ no unified "copy/export/jump" — StreamView scope (2 items: Compare Display/Decode Order, both **no-op stubs**) and DiagnosticsPanel scope (2 items: Export Evidence Bundle + Copy Selection, both real, no "jump") are the only 2 of these components with any context menu at all; syntax tree/StatisticsTab have none (no `ContextMenuScope` variant exists for them) (verified 2026-08-23, see PARITY_CHECKLIST.md INT-06 for per-component table) |

Global modifier convention: **Shift**=range/multi-select, **Ctrl/Cmd**=additive select/pin, **Alt/Option**=alternate mode (scrub/fine-adjust). Bitvue keyboard shortcuts already documented in `PARITY_CHECKLIST.md` Layer 4 (Keyboard Shortcut Parity) — cross-check modifier consistency against this table when implementing.

---

## 4. Per-panel tooltip field contract (compressed from `TOOLTIP_SPEC.md`)

Global rules: appear ≤150ms (debounced), never cover cursor target, stable while hovering same target, support multi-line + monospace blocks + copy buttons, always show units, show "N/A" explicitly for missing fields, hit-test dense targets by logical ID not pixel proximity.

| Panel/target | Required fields | Required actions |
|---|---|---|
| Timeline bar/marker | ⚠️ real tooltip (`TimelineTooltip.tsx:15-31`) shows only `frame_index`/`frame_type`/size-in-KB — no GOP/IDR grouping, no PTS/DTS, size not in bytes+bits, no marker flags; decode-status/last-error cleanly omitted (field doesn't exist on `FrameInfo`, consistent with DIV-07). Sibling `FilmstripTooltip.tsx` (different view, not Timeline/Bars) is richer (PTS/POC/temporal-id/refs) but still has 0 actions (verified 2026-08-23) | ❌ none of the 3 actions exist — plain `<div>`s, no buttons |
| Metrics plot point | ⚠️ FrameSizesView native `title` = frame idx+size+QP only; HRD tooltip div = frame idx+occupancy+over/underflow — neither has timestamp, explicit series name, or delta-vs-prev (verified 2026-08-23) | ❌ neither action exists (native title has none; HRD tooltip is a static read-only div) |
| HRD/buffer point | time/frame idx, fullness (bits), CPB size, under/overflow risk state | Copy time+fullness, Jump to nearest frame |
| Tree row | ❌ aspirational — real tooltip is `title={node.description}`, always `undefined` for real backend data; none of breadcrumb/offset+size/type/flags/parsed/diagnostic-id are rendered on hover (offset does appear elsewhere, as a separate hex-jump-icon tooltip showing only offset) (verified 2026-08-23) | ❌ none of the 3 actions exist anywhere — the only "Bookmark"/"Copy Offset"/"Copy Path" hits repo-wide are the unrelated standalone `BookmarksPanel.tsx` |
| Syntax node | field name+type, decoded value, bit range, raw bits preview, condition eval | Copy Field, Copy BitRange, Copy Raw Bits |
| Hex/bit cell | ⚠️ split across two UI pieces, neither complete — hover `title` has hex offset+value only; click-triggered `.hex-byte-info` panel adds dec/ASCII/8-bit-local-binary but no *global* bit index and no mapped-syntax-field display (the reverse lookup is computed backend-side but not rendered here) (verified 2026-08-23) | ❌ none of the 3 named actions exist — only a generic "Copy Bytes" context-menu command |
| Player surface | ✅ **fixed 2026-08-23** (INT-01) — `PlayerPixelTooltip.tsx` shows frame idx, pixel (x,y), real Y/U/V values, resolved block position/size, active-overlays list; `StatusBar.tsx` still separately shows the static (non-hover) frame idx/zoom%/mode | ⚠️ Pin Sample explicitly optional/v2 (not implemented); Copy Pixel not implemented — no click-triggered mechanism exists to capture a transient hover value (see PARITY_CHECKLIST.md INT-01 note); "Copy Selection" (persisted block, not hover) remains a separate unwired context-menu item |
| Diagnostics row | severity, category, offset+frame/unit refs, full message, root-cause chain (collapsible) | Copy Diagnostic, Copy Offset, Jump |

Nice-to-have (not required v1): pinnable tooltip → small inspector box; architecture shouldn't block it later.

---

## 5. Zoom/pan policy per panel (from `ZOOM_POLICY.md`)

| Panel | Pan | Zoom | Type | Notes |
|---|---|---|---|---|
| Timeline/Bars | ❌ No — drag is frame-scrub, not pan; only native scrollbar overflow | ❌ No — zero zoom code in `Timeline.tsx` (no wheel/dblclick/scale state) | ❌ N/A — no LOD-bucket aggregation anywhere, frames render 1:1; sibling `ThumbnailsView.tsx`'s zoom (different component) is a plain CSS `scaleX`, Visual not Semantic | ❌ N/A — no marker glyphs, no axis exists in `Timeline.tsx` (`TimelineHeader.tsx` is just a title+counter) (verified 2026-08-23) |
| Metrics/HRD | ❌ No | ❌ No | ❌ N/A — no zoom exists, both always render full range at fixed 1:1 scale | ⚠️ font size is constant (10px) but HRD's Y-axis *label count* actually adapts to panel height (`HRDBufferPanel.tsx:241-250`) — moot without zoom to hold density fixed against (verified 2026-08-23) |
| Thumbnails strip | scroll | optional | Visual (size-adaptive) | adapts to available height regardless |
| Player surface | ⚠️ Yes, but ungated (no "Fit" level exists to compare against) | ✅ Yes | ✅ Visual (CSS `transform: scale()`, `VideoCanvas.tsx:140-146`) | ✅ **Fit/100%/200% presets fixed 2026-08-23** — real `1`/`2`/`3` keydown cases in `YuvViewerPanel/index.tsx` (+ a toolbar "Fit to Window" button); "Fit" computes a genuine zoom from the container's real on-screen size vs the frame's logical size (not hardcoded), verified via a real Electron run (`BITVUE_ELECTRON_SCREENSHOT_KEY=1`, zoom label changed 100%→153% matching the real container). ✅ overlays do scale in sync |
| Overlay vectors/grids | (follows player) | (follows player) | Mixed | geometry scales w/ player zoom; line/arrowhead fixed-density |
| Hex/Bit view | ✅ scroll | ✅ **No** — genuinely honored, no scale transform anywhere | ✅ Fixed-density (`BYTES_PER_LINE=16` constant) | ⚠️ never chart-style zoom confirmed true; "optional font-size pref" is aspirational — no settings field/CSS var exists, all sizes hardcoded in `UnitHexPanel.css` (verified 2026-08-23) |
| Syntax tree/diagnostics table | ✅ scroll | ✅ **No** — confirmed, no zoom code anywhere in these 3 components | ✅ Fixed-density | ❌ "optional global UI-scale setting" is entirely aspirational — no Settings/Preferences component exists in `frontend/` at all, no Electron `setZoomFactor` menu item either (verified 2026-08-23) |
| Reference graph | ❌ No | ❌ No | ❌ N/A — no canvas/SVG exists, plain DOM flexbox | ❌ N/A — no zoom, so no zoom-dependent label logic; **component is unmounted dead code** (`ReferenceGraphPanel.tsx:46-53` own comment: not in any `App.tsx` panel list, and its `invoke("get_frames")` targets a Tauri API absent under Electron — would throw immediately if mounted) (verified 2026-08-23) |

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
| HexView | Copy Offset | `Copy.Offset` | has_byte_range |
| HexView | Copy Bit Range | `Copy.BitRange` | has_byte_range |
| HexView | Export Evidence Bundle | `Export.EvidenceBundle` | always |
| StreamView | Compare in Display Order | `Set.OrderType.Display` | always |
| StreamView | Compare in Decode Order | `Set.OrderType.Decode` | always |

**Copy Offset/Copy Bit Range added 2026-08-23** (`context_menu.rs`'s `HexView` scope) — see
`PARITY_CHECKLIST.md` INT-03's fix note.

**Guards:** `always` (no restriction) · `has_selection` (disabled reason: "No selection.") · `has_byte_range`
(disabled reason: "No byte range selected."). Policy: disabled items must show the reason as a tooltip; guard
evaluation must never mutate state. This is a starter set (marked "expand via screenshot-driven refinement" in
the source) — treat per-panel rows in §3 above as the fuller target once built.

**Verified 2026-08-23 — policy genuinely honored:** ✅ disabled-reason really renders as a native `title` tooltip
(`ContextMenu.tsx:62-63`, `title={item.disabled_reason}`); ✅ guard evaluation is provably pure — `context_menu.rs`'s
`evaluate_context_menu_guard`/`build_context_menu` take `&GuardEvalContext` by shared reference, never mutate;
✅ disabled items are genuinely un-clickable (native HTML `disabled` blocks the click at the DOM level, no bypass
surface found). The previously-noted Player-scope defect (guard inputs hardcoded `false`) is **stale as of
2026-08-23**: `YuvViewerPanel/index.tsx`'s `handleCanvasContextMenu` now computes both `hasSelection`/
`hasByteRange` from real `selection` state (fixed incidentally during INT-01's click→spatialBlock work).
Timeline/DiagnosticsPanel/HexView/Player each now correctly compute the guard value(s) their scope's items
actually use; StreamTreePanel hardcodes both but is currently inert (its 2 items use only the `always` guard).

---

## 7. Evidence bundle export contract (from `evidence_bundle_diff_contracts.json` + `export_entrypoints.json`)

Previously flagged as a gap ("no equivalent in Bitvue — closest is `bitvue export --json`"). The harness defines
a concrete one-click bundle contract; real and complete as of 2026-08-19 (`PARITY_CHECKLIST.md` EVB-01 `[x]`,
all 4 entrypoints below wired to the same `Export.EvidenceBundle` command).

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
| IA_TRISYNC_PANELS_PRESENT | IA | P0 | Are Tree⇄Syntax⇄Hex⇄Charts⇄Player all present and reachable without detours, with selection propagating across all of them? | ⚠️ partial — **Syntax⇄Hex verified bidirectional 2026-08-21** (real `select_bit_range`/`find_nearest_node` round trip, not the old one-directional `SyntaxHexLinkContext` stopgap it replaced; see `SelectionContext.tsx`/`FrameSyntaxTab.tsx`/`HexViewTab.tsx`, real Electron screenshot confirmed the hex→syntax auto-expand). Unit(OBU)-tree, spatial-block/QP-heatmap, Ref Graph, and Metrics selection still not wired — `SelectionState` only has fields for 5 of 7 documented multi-sync views (see `DEVELOPMENT_PHASES.md`'s Architecture appendix). **Re-verified 2026-08-23**: `ReferenceGraphPanel.tsx`/`QualityMetricsPanel.tsx` still have no `SelectionContext` usage at all — Ref Graph/Metrics legs unchanged. **spatial-block leg fixed same day** (INT-01): `selectSpatialBlock` now has a real production call site (`VideoCanvas.tsx`'s click-vs-drag detection → `SelectionContext.setSpatialBlockSelection`, new); `SelectionState.temporal`'s existing `{type:"block"}` variant (it already had the field, just nothing ever populated it) is now real. Unit(OBU)-tree, Ref Graph, and Metrics selection remain the 3 unwired legs of 7 |
| CONTRACT_DISPLAY_DECODE_SEPARATION | Contract | P0 | Is display-order vs decode-order separation enforced everywhere (UI logic, evidence exports, compare), no implicit mixing? | ⚠️ **data half fixed 2026-08-23** (PARITY_CHECKLIST.md CTX-02): `FileStateContext.tsx`'s new `applyDisplayOrder` joins `frames` against `get_timeline`'s real PTS-sorted `display_idx` by `pts`, populating `display_order`/`coding_order` for ~8 consumers that were silently showing "N/A" — verified with a real Electron screenshot (DetailsPanel now shows real `Display Order`/`Coding Order`). **Still not enforced at the primary visual surface**: `Timeline.tsx`/`TimelineThumbnails.tsx` still key/render/scrub bars by array position (decode order) — physically reordering them is a separate, deliberately-deferred follow-up (see the fix's doc comment for why: arrow-nav/hit-testing/`setFrameSelection` all assume array position === decode `frame_index`). No PTS/DTS-mismatch band exists anywhere. `docs/FRAME_IDENTITY_CONTRACT.md` still doesn't exist as a standalone file. CLI **does** have `-display_order` parity — `bitvue-cli/src/main.rs:101-102` + `commands/decode.rs:118-125` (PTS-sorts + re-indexes when set) |
| INTERACTION_CONTEXT_MENU_GUARDS | Interaction | P1 | Do right-click context menus appear where expected, with disabled items showing a discoverable reason? | ⚠️ contract now defined in §6 above (3 scopes, 7 items, guard+disabled-reason policy) — not yet implemented in Bitvue |
| EVIDENCE_ONE_CLICK_BUNDLE | Evidence | P0 | One-click export bundling screenshots+evidence+version/env+selection-state+order-type+backend fingerprints? | ⚠️ contract now defined in §7 above (required files, ABI policy, 4 entrypoints) — not yet implemented; closest existing is `bitvue export --json` (single-frame/stream, not a full bundle) |
| PERF_ENVELOPE_LOD_VIRTUALIZATION | Performance | P1 | Does the UI stay responsive on large streams via virtualization+LOD, no stale-async or cache-invalidation bugs? | ⚠️ **verified 2026-08-23 — budgets exist but disconnected from reality**: the exact numbers (UI frame ≤16.6ms, hit-test ≤1.5ms, overlay render ≤6.0ms, tooltip build ≤0.8ms, selection propagation ≤2.0ms) live only in `bitvue-engine/src/parity_harness/gates.rs:146-165` as a hardcoded `PerfBudgets::default()` — the `perf_budget_and_instrumentation.json` source file it cites doesn't exist, `PerfTelemetryEvent`/`record_telemetry` are only ever constructed in unit tests (no real GUI telemetry pipeline), and the 4-step degrade sequence (disable labels → aggregate vectors → downsample heatmap → placeholder) is defined in the same test-only Rust module but has zero frontend implementation (0 grep hits for any of the 4 steps in `frontend/`). `lockcheck.rs`'s `run_perf_smoke` has a *different*, unrelated set of thresholds and — confirmed by grep — is wired into neither `scripts/parity_check.sh` nor any CI workflow. Real virtualization does exist independently: `VirtualizedFilmstrip.tsx:20-46` (genuine viewport windowing) — but it's unrelated to this budget/degrade machinery |

**Now tracked as Layer 7** (2026-07-31): context-menu system (§6) → `PARITY_CHECKLIST.md` CTX-01, one-click
evidence-bundle export (§7) → EVB-01. Both promoted into `DEVELOPMENT_PHASES.md` Phase 7.6 (inserted right after
Phase 7.5 rather than deferred to Phase 11 — see that doc's "우선순위 재검토" note for rationale).

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
precedence in `crates/bitvue-engine/src/selection.rs` already matches the rulebook's Block>Point>Range>Marker order.

| Rule | Detail | Verified 2026-08-23 |
|---|---|---|
| Graph viewport zoom bounds | `[fit*0.5 .. 16x]`; world bounds clamped to `layout_bbox * 1.5`; Fit/Home always available | ❌ N/A — `ReferenceGraphPanel.tsx` has no canvas/zoom/pan of any kind and is itself unmounted dead code (see §3 Reference-graph row) |
| Node/edge density cap | Reference-graph-style views switch to summary mode + banner when node/edge count exceeds cap | ❌ absent — only a flat "+N" count truncation exists, not summary-mode+banner |
| Overlay draw order | Selection highlight always drawn last (topmost), above all overlay layers | ❌ absent — no "selection highlight" concept exists anywhere in the overlay pipeline; `OverlayRenderer/index.tsx` implements exactly 2 passes (mode overlay, info overlays), no 3rd topmost-selection pass. A structurally similar `overlay_stack.rs` z-order module exists but has no `Selection` layer type and zero callers |
| PTS quality degrade | VFR/missing/duplicate PTS → primary timeline axis falls back to `display_idx`; badge shows PTS quality OK/WARN/BAD | ✅ **badge fixed 2026-08-23** (EDGE-03) — `TimelineBase.pts_quality` now real (set from `FrameIndexMap.pts_quality()`), flows to a new `PtsQualityBadge` in `TimelineHeader` (Warn/Bad only). No fixture in this repo has non-Ok PTS, so Warn/Bad are unit-test-verified, not screenshot-verified. "Primary axis falls back to display_idx" half is unrelated/separate — display_idx itself doesn't change based on PTS quality, only the confidence badge does |
| Indexing-in-progress | Frame jumps allowed only within already-indexed range; out-of-range shows "Index building…" (no queueing) | ⚠️ well-built but disconnected — `indexing.rs`'s `IndexReadyGate`/`OpenFastPath` implement this almost verbatim (tested), but have zero callers outside their own module; production sidecar uses an unrelated simple `indexed: bool` flag with no partial-range gating, and `GoToFrameDialog.tsx` only validates `1..totalFrames` |
| Overlay partial coverage | Legend shows coverage %; auto-disable overlay with reason if coverage < 20% | ⚠️ same pattern — `qp_heatmap.rs`'s `coverage_percent()`/`has_sufficient_coverage()` (`<20%` threshold, doc comment matches this rule verbatim) are called only from tests; no sidecar command exposes it, 0 frontend references, no legend shows a coverage % anywhere |
| Overlay missing data | Transparent render + legend note (never a blank/broken overlay) | ✅ genuinely implemented, consistently, across ~14 renderer files (CDEF/FilmGrain/LoopRestoration/SuperRes/QP/MV/Prediction/Transform/CodingFlow/VVC/AVS3/JPEG-XS) — each shows an explanatory message box on missing data. One wording nuance: boxes are semi-opaque black, not literally "transparent" |
| Compare alignment confidence | High/Med/Low; Low confidence → diff view disabled by default, manual offset still allowed | ✅ real end-to-end: `alignment.rs`'s `AlignmentConfidence` → `compare.rs`'s `check_diff_eligibility` → sidecar `diff_enabled`/`disable_reason` wire fields → `CompareWorkspace.tsx` gates the Show-Diff checkbox + shows a warning banner; manual offset controls stay available regardless. One threshold nuance: actual disable condition is Low **and** >30% gaps, not Low alone |
| Compare resolution mismatch | Diff disabled; optional explicit resample marked experimental | ⚠️ disable-on-mismatch half is real (shares the Rule-8 path, `diff_heatmap.rs`'s `ResolutionCheckResult`, `>5%` mismatch → incompatible); the "experimental resample" half (`DiffHeatmapOverlay.experimental_resample`/`toggle_resample()`) is unused dead code — zero callers outside its own file/tests, no frontend control |
| Async backpressure | Late results discarded if `request_id` mismatches current; non-current jobs cancelled during scrub; in-flight cap strictly enforced (see `DEVELOPMENT_PHASES.md` Architecture appendix "Async Backpressure" row) | ⚠️ cancellation is real and well-tested (`main.rs`'s per-request `AtomicBool` cancel-flag registry, `getDecodedFrameYuvCancellable`'s scrub-cleanup pattern in `YuvViewerPanel/index.tsx` correctly discards/cancels superseded requests) — but **"in-flight cap strictly enforced" is false**: every request spawns an unbounded `std::thread` (`main.rs:13-20`), no concurrency limit/cap constant exists anywhere in sidecar or frontend |
| Non-negotiable | Never freeze UI; never show a blank panel — every failure state has a fallback + explanation | ⚠️ strong at async/data-fetch granularity (spot-checked `YuvViewerPanel`/`BitrateGraphPanel`: real error state + Retry button, never blank) but weaker at render-crash granularity — `ErrorBoundary.tsx` is a real, working React error boundary, but mounted only **once around the entire app** (plus once for lazy dialogs), not per-panel — a render-time throw inside any single panel takes down the whole app to one generic fallback screen, not isolated to that panel |

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
Rust modules in `crates/bitvue-engine/src/` (module doc-comments cite the exact spec filenames: `selection.rs` ↔
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
