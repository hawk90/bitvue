# Bitvue — CLAUDE.md

Baseline pointer file. Keep this short — details live in the docs it points to, not duplicated here.
Bitvue = open-source video bitstream analyzer (Electron + Rust + React/TypeScript), building feature parity
with commercial tools (VQ Analyzer, VQ Probe, VEGA, StreamEye). Migrated off Tauri 2026-08-08 — see
`docs/DEVELOPMENT_PHASES.md` § "제품 아키텍처 확정" for the sidecar-process architecture and rationale.

## Doc map (source of truth — read before starting parity/feature work)

| Doc | Owns |
|---|---|
| `docs/specs/features.yaml` | **The only feature status / parity source of truth** (id, area, priority, status, acceptance, evidence, competitors, legacy ids). Schema + update rules: `docs/specs/README.md` |
| `docs/VQA_PARITY_SPEC_V3.md` | Backend/codec spec narrative: goals, tech stack, §1.5 priority rationale, F-key numbering |
| `docs/PARITY_CHECKLIST.md` | Parity validation strategy, test sources, verification tiers, regression suite, fixtures |
| `docs/COMPETITOR_FEATURE_MATRIX.md` | Competitor research provenance, out-of-scope rationale, per-product counts |
| `docs/UX_PARITY_MATRIX.md` | UI/UX contracts narrative, menu tree, overlay colour scale, shortcuts ownership |
| `docs/DEVELOPMENT_PHASES.md` | Architecture decisions (sidecar, "제품 아키텍처 확정") + Phase 0-12 goals/rationale |
| `docs/history/` | Dated dev logs + frozen old checklist notes — history, not current state |
| `archive_docs/` | Retired docs, kept for history — don't treat as current |

Status changes go in `features.yaml` only (edit the item, keep its id, add evidence) — never ✅/❌ in Markdown.
Don't create new parity docs.

## Known doc drift

- **Resolved 2026-08-10** (`3dcd0dc`, user sign-off given): `README.md`'s Tauri-era build instructions
  (`npm run tauri:dev`/`tauri:build`, WebView2 prereq, `src-tauri/` architecture tree) reconciled with the
  Electron migration, and the VMAF/BD-rate "shipped feature" claim corrected to match `PARITY_CHECKLIST.md`
  Layer 6 (CMP-05/CMP-06 — now in `features.yaml`: BD-rate unwired, VMAF behind an optional unwired-by-default Cargo feature). If
  README drifts again (new features shipped, build flow changes), re-run the same reconciliation — grep
  actual `Cargo.toml` workspace members / `package.json` scripts / `docs/specs/features.yaml` status rather than
  trusting README's existing prose, and get user sign-off before changing public-facing marketing claims.

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
- **Never run `cargo test -p <crate> --lib` alone as a "did I break anything" check** — it skips every
  `tests/` integration binary in that crate. A real infinite-loop bug in `ObuIterator` sat undetected for an
  entire session because of exactly this (2026-08-11, see `bitvue-sidecar`'s `tests/subprocess_smoke.rs`).
  Use `cargo test --workspace --exclude abseil --lib --tests` (or `scripts/run_regression_suite.sh`, which
  already does this correctly) for any change that isn't obviously scoped to one file's inline unit tests.

## Key scripts

`scripts/dev.sh` `scripts/setup.sh` `scripts/parity_check.sh` (--local for the regression gate)
`scripts/run_regression_suite.sh` `scripts/check_fuzz_targets.sh` (fuzz/ is outside the workspace: targets registered + compiling) `scripts/clean.sh` `scripts/prune.sh` (drop Cargo incremental caches, ~60% of target/) `scripts/package_electron.sh [mac|linux|win]`
(builds+packages the Electron app; local counterpart to `.github/workflows/build-electron-app.yml`)

## Agents

Bitvue-specific expert agents live in this repo's `.claude/agents/` (`bitvue-master`, `rust-master`,
`video-codec-expert`, `video-formats-expert`, `documentation-writer`, `performance-profiler`, `test-engineer`,
`ui-ux-designer`). Skills in `.claude/skills/`: `bitstream` (inspect streams via `bitvue` CLI/MCP, codec &
container reference), `optimize` (benches, profiling, hotspots, performance targets), `test-bitvue` (test
layout/commands, llvm-cov, coverage targets). No project commands. For generic code review / security review
use the built-in `/code-review` and `/security-review`.

**2026-10-05:** moved the Bitvue agents/commands/skills here from `~/.claude/`; deleted the 12 generic
template agents, `commands/generate-tests.md`, and `tauri-master` (Tauri removed 2026-08-08). Same day:
merged the 4 commands + 7 skills into the 3 skills above and fact-checked the agents against the code.

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
