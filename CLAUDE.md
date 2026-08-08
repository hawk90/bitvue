# Bitvue — CLAUDE.md

Baseline pointer file. Keep this short — details live in the docs it points to, not duplicated here.
Bitvue = open-source video bitstream analyzer (Tauri + Rust + React/TypeScript), building feature parity
with commercial tools (VQ Analyzer, VQ Probe, VEGA, StreamEye).

## Doc map (source of truth — read before starting parity/feature work)

| Doc | Owns |
|---|---|
| `docs/VQA_PARITY_SPEC_V3.md` | Backend/codec feature spec, priorities |
| `docs/PARITY_CHECKLIST.md` | Implementation tracking — **the only ✅/⚠️/❌ source of truth**; also owns parity validation strategy/test sources |
| `docs/COMPETITOR_FEATURE_MATRIX.md` | Per-product feature-by-feature matrix (VQ Analyzer/VQ Probe/VEGA/StreamEye/Codecian) |
| `docs/UX_PARITY_MATRIX.md` | UI/UX interaction parity (mouse/tooltip/zoom contracts, workspace specs, menu structure, overlay color scale) |
| `docs/DEVELOPMENT_PHASES.md` | Phase 0-12 implementation roadmap (Rust sketches, task checklists, time estimates) |
| `archive_docs/` | Retired docs, kept for history — don't treat as current |
| `docs/_import_v14/` | Gitignored raw import (a prior design-blueprint pack, dated 2026-01) — **mining complete as of 2026-07-31** (all ~30 previously-unread files triaged; nothing of value left unmined). Pending deletion (not yet executed — see below), then this row should be removed |

All four active docs cross-link via a "See also" header — follow it before assuming a doc is standalone.
Don't create a 5th parity doc; extend one of the four above.

## Pending cleanup (flagged, not executed — 2026-07-31)

`docs/_import_v14/` mining pass finished (see `DEVELOPMENT_PHASES.md` Architecture/Future-Differentiators/MCP
appendices, `PARITY_CHECKLIST.md` release-gate + Layer 6 CMP-01/02 corrections, `UX_PARITY_MATRIX.md` §13-14 —
all added 2026-07-31). The subagent that did this mining had no shell tool, so it could not run the intended
deletion. Someone with shell access should run (originals remain in `~/Downloads/` as fallback):
```
rm -rf docs/_import_v14/monster_pack docs/_import_v14/index_generator docs/_import_v14/task_schemas docs/_import_v14/parity_harness
```
(`parity_harness/` included — its unread files turned out to be a manifest/schema/prompt-file set for a
competitor-parity-matrix harness already implemented in `crates/bitvue-engine/src/parity_harness/mod.rs`; nothing
left in it was undocumented.) Then delete `docs/_import_v14/` itself and remove its row from the doc-map table
above.

## Known doc drift (unresolved — flag before trusting)

- `README.md` claims "VMAF calculation with BD-rate analysis" as a shipped feature. Per `PARITY_CHECKLIST.md`
  Layer 6 (CMP-05/CMP-06), this is **not implemented** — `libvmaf-sys` exists as an optional Cargo feature but
  is unwired, and BD-rate calc is unverified. Reconcile before next release; don't silently "fix" the README's
  marketing claims without the user's sign-off (public-facing copy).

## Working conventions (established this session, confirmed by user correction)

- **Docs are compressed reference material, not human prose.** Tables, one row per concrete feature. No framing
  sentences a re-reader doesn't need. This was corrected once already — don't regress to narrative summaries.
- **Delegate large doc-compaction/multi-file passes to a subagent**, not the main loop. When delegating, paste
  raw source data directly into the prompt rather than telling the subagent to re-derive it — keeps results
  reproducible and avoids repeat research cost.
- Before trusting any "진행 상황" claim (in a doc or your own memory), check `git log` — this repo's history
  has outpaced stale progress notes more than once.
- Verify before citing: grep for the actual code/file when a doc claims something is implemented — several
  `COMPETITOR_FEATURE_MATRIX.md` rows were only correctly marked ✅ after a code grep confirmed them.

## Key scripts

`scripts/dev.sh` `scripts/setup.sh` `scripts/parity_check.sh` (--local for the regression gate)
`scripts/run_regression_suite.sh` `scripts/clean.sh`

## Agents

Bitvue-specific expert agents (`bitvue-master`, `rust-master`, `tauri-master`, `video-codec-expert`,
`video-formats-expert`, `documentation-writer`, `performance-profiler`, `test-engineer`, `ui-ux-designer`) are
defined **globally** (`~/.claude/agents/`), not in this repo's `.claude/agents/` (which holds generic
general-purpose agent templates: `api-documenter`, `architect-reviewer`, `changelog-generator`, `code-reviewer`,
`context-manager`, `debugger`, `dependency-manager`, `product-strategist`, `research-coordinator`, `rust-pro`,
`security-auditor`, `technical-writer`). Prefer the global Bitvue-tuned ones when a name overlaps in spirit
(e.g. `rust-master` over `rust-pro`, `documentation-writer` over `technical-writer`).

**2026-07-31 cleanup:** deleted local `.claude/agents/{performance-profiler,test-engineer,ui-ux-designer}.md`
— they had the *exact same names* as better, Bitvue-tuned global agents but were generic 200-900 line
boilerplate stubs silently shadowing them (verified by diff: global versions are concise, project-specific;
local ones were generic "specializing in ... across all technology stacks" filler). Also deleted
`video-editor.md` (FFmpeg video editing/color-grading — not this project's domain at all). If a future local
agent file collides in name with a global one, diff them before assuming the local copy is the customization —
here it was the reverse.

**Unresolved, flagged not fixed:** `.claude/commands/generate-tests.md` (Bitvue-tuned) and the global skill
`~/.claude/skills/generate-tests.md` (generic) share the name `generate-tests` — precedence between a
project command and a same-named global skill wasn't verified. If `/generate-tests` ever produces generic
output instead of Bitvue-aware output, this collision is why — investigate before adding more project commands
with names that might collide globally.

## Workflows

`.claude/workflows/parity-gap.js` — reusable Claude Code Workflow (not GH Actions) mirroring the CUDA project's
`whitepaper-gap.js` pattern: research a competitor product → compact findings into the parity docs as dense
tables → fix cross-references. Run with `Workflow({scriptPath: '.claude/workflows/parity-gap.js', args: {product: 'vega'}})`
(product keys: `vq_analyzer`, `vq_probe`, `vega`, `streameye`, `codecian`, or pass `urls` explicitly for a new
source). Only invoke when actually re-scanning for parity drift — it spawns multiple research agents per run.

## Anti-pattern catalog

`docs/anti-patterns/INDEX.md` — 751 items across 36 files, three domain waves (Rust/media engineering,
VQ-Probe quality-analysis domain, UI/UX+Tauri+React). Reference catalog only — every item's `Bitvue 판정`
field is unfilled by design; the actual repo audit is a separate later pass via a not-yet-built
`.claude/workflows/anti-pattern-scan.js`. Read `INDEX.md` first, not the individual files, for the full map
and next-steps (a dedup pass is recommended before adding a Phase 4).
