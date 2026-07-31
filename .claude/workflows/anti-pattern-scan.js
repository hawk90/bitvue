export const meta = {
  name: 'anti-pattern-scan',
  description: "Audit the Bitvue repo against docs/anti-patterns/*.md, filling in each item's Bitvue 판정 verdict",
  whenToUse:
    'Run after the anti-pattern catalog is stable (see docs/anti-patterns/INDEX.md). args: { files?: string[] } ' +
    '- category IDs like ["PARSE","MEM"] to scope the scan; omit for all files (expensive - 47 files as of the ' +
    'last catalog wave, one agent per file).',
  phases: [{ title: 'Audit', detail: 'per file: grep the relevant crate/dir, judge each item, write verdict back' }],
}

// Rough file -> code-area mapping, steers each audit agent's grep scope. Extend when new catalog files land.
const AREA_HINTS = {
  OWN: 'crates/',
  MEM: 'crates/',
  LAYOUT: 'crates/',
  PARSE: 'crates/bitvue-avc crates/bitvue-hevc crates/bitvue-vp9 crates/bitvue-vvc crates/bitvue-av1-codec crates/bitvue-av3-codec crates/bitvue-mpeg2-codec crates/bitvue-avs3 crates/bitvue-jpegxs crates/bitvue-vc3',
  CODEC: 'crates/bitvue-codecs crates/bitvue-codecs-parser crates/bitvue-formats',
  IO: 'crates/bitvue-formats crates/bitvue-core',
  CONC: 'crates/bitvue-core src-tauri/src',
  CACHE: 'crates/bitvue-core',
  IPC: 'src-tauri/src/commands',
  PIXEL: 'crates/bitvue-decode crates/bitvue-metrics',
  ERR: 'crates/ src-tauri/src',
  PERF: 'crates/bitvue-benchmarks',
  API_TYPE: 'crates/ Cargo.toml',
  FRONTEND: 'frontend/',
  ALIGN: 'crates/bitvue-core/src/compare.rs crates/bitvue-core/src/alignment.rs',
  SPATIAL: 'crates/bitvue-core/src/compare.rs crates/bitvue-core/src/compare_strategy.rs',
  COLOR: 'crates/bitvue-formats crates/bitvue-decode',
  METRIC: 'crates/bitvue-metrics',
  PIPE: 'crates/bitvue-metrics crates/bitvue-core/src/compare.rs crates/bitvue-core/src/compare_cache.rs',
  HEAT: 'crates/bitvue-core/src/diff_heatmap.rs frontend/components/panels/OverlayRenderer',
  STAT: 'crates/bitvue-metrics',
  UIX_IA: 'frontend/components frontend/contexts',
  UIX_SYNC: 'crates/bitvue-core/src/selection.rs frontend/contexts frontend/components',
  UIX_ASYNC: 'frontend/ src-tauri/src/commands',
  UIX_TIMELINE: 'frontend/components/Filmstrip.tsx frontend/components/panels',
  UIX_TREE_HEX: 'frontend/components/panels',
  UIX_VIZ: 'frontend/components/panels/OverlayRenderer',
  UIX_INPUT: 'frontend/',
  UIX_LAYOUT: 'frontend/components',
  UIX_ERR: 'frontend/ src-tauri/src',
  UIX_A11Y: 'frontend/',
  TAURI_CMD: 'src-tauri/src/commands',
  TAURI_EVT: 'src-tauri/src',
  TAURI_WEB: 'src-tauri/',
  FRONT_REACT: 'frontend/components frontend/contexts',
  UX_SCENARIO: 'frontend/ scripts/',
  FFI: 'crates/bitvue-decode (dav1d FFI) crates/bitvue-metrics (libvmaf FFI) crates/vendor/abseil',
  DEC: 'crates/bitvue-decode',
  RPERF: 'crates/',
  SER: 'src-tauri/src/commands crates/bitvue-cli crates/bitvue-mcp',
  PLUGIN: 'crates/bitvue-codecs crates/bitvue-codecs-parser Cargo.toml',
  MCP: 'crates/bitvue-mcp crates/bitvue-core/src/mcp.rs',
  BUILD: 'Cargo.toml crates/*/Cargo.toml .github/workflows/ci.yml src-tauri/Cargo.toml',
  PLAT: 'src-tauri/ frontend/',
  TEST: 'crates/*/tests fuzz/ scripts/ crates/bitvue-cli/tests',
  OBS: 'crates/bitvue-core src-tauri/src',
  SEC: 'src-tauri/src/commands crates/bitvue-formats crates/bitvue-core',
}

const targets = args?.files ?? Object.keys(AREA_HINTS)
if (!args?.files) {
  log(`No files specified - auditing ALL ${targets.length} catalog files (one agent each). This is expensive; pass args: { files: ["PARSE", "MEM"] } to scope it.`)
} else {
  log(`Auditing ${targets.length} file(s): ${targets.join(', ')}`)
}

phase('Audit')
const audited = await pipeline(
  targets,
  fileKey =>
    agent(
      `Bitvue project (video bitstream analyzer, Tauri + Rust + React). Audit docs/anti-patterns/${fileKey}.md ` +
        `against the REAL codebase - this is Phase 2 of a two-phase plan; the catalog itself (Phase 1) is already ` +
        `written and every item currently reads "**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움".\n\n` +
        `Read the file, and for EACH item (every "### {ID}: ..." record) investigate whether the anti-pattern is ` +
        `actually present in the code. Grep/read around ${AREA_HINTS[fileKey] ?? 'crates/ src-tauri/ frontend/'} ` +
        `and anywhere else relevant if that hint is wrong or incomplete. For each item, classify:\n` +
        `- **Confirmed** — found concrete evidence, cite file:line\n` +
        `- **Suspected** — structurally plausible given the code's shape but not directly confirmed; explain why\n` +
        `- **N/A** — pattern doesn't apply to this codebase's architecture, feature doesn't exist yet, or genuinely absent\n\n` +
        `Edit the file in place: replace each item's line ` +
        `"**Bitvue 판정**: 미정 — 2단계(저장소 감사)에서 채움" with ` +
        `"**Bitvue 판정**: {Confirmed|Suspected|N/A} — {one-line evidence/reasoning, file:line if applicable}". ` +
        `Don't guess - if you can't find enough signal either way after a genuine look, mark Suspected and say so ` +
        `rather than picking Confirmed or N/A to seem decisive.\n\n` +
        `Report back SHORT: counts of Confirmed/Suspected/N/A for this file, and the 2-3 most severe Confirmed ` +
        `findings if any (one line each). Do not paste the full file back.`,
      { phase: 'Audit', label: `audit:${fileKey}` }
    )
)

const summaries = audited.filter(Boolean)
log(`Audited ${summaries.length}/${targets.length} files.`)

return { filesAudited: targets, summaries }
