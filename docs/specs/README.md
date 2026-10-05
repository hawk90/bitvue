# docs/specs — feature registry

`features.yaml` is the **single source of truth for feature status and competitor parity**. The Markdown docs in
`docs/` keep only narrative: goals, decisions, rationale, reference tables. Don't put ✅/❌ status in Markdown.

## Schema (`schema: 1`)

Each entry in `items:` has these fields, in this order (the file is written with PyYAML (`sort_keys=False`, `allow_unicode`), with
indented lists, `width=120`; keep the order when editing by hand):

| Field | Required | Meaning |
|---|---|---|
| `id` | yes | `<AREA>-<NNN>`, e.g. `OVL-012`. **Stable**: never renumber or reuse an id. New items take the next free number in their area. |
| `title` | yes | Short English name. |
| `area` | yes | `codec` `container` `decode` `overlay` `syntax` `ui` `ux` `compare` `metrics` `export` `cli` `mcp` `perf` `infra` `qa` `probe`. Prefix: CODEC CONT DEC OVL SYN UI UX CMP MET EXP CLI MCP PERF INFRA QA PROBE. |
| `priority` | yes | `P0`..`P3`, taken from the source doc. `priority_inferred: true` means no source gave one. |
| `priority_inferred` | no | `true` when no source doc gave a priority (omit otherwise). |
| `status` | yes | `done` · `partial` · `todo` · `dropped` (see below). |
| `phase` | no | String. Roadmap phase from `DEVELOPMENT_PHASES.md` (`"0"`..`"12"`, `"7.5"`, `"7.6"`), or `"product-N"` for a product stage (§확정 순서). Omit when no phase applies. |
| `codecs` | no | List, only for codec-specific items: `av1` `hevc` `avc` `vp9` `vvc` `mpeg2` `avs3` `jpegxs` `vc3` `apv` `avm`. |
| `acceptance` | yes | List of concrete, checkable criteria. |
| `evidence` | yes | List of repo paths proving the status: `path`, `path:line` or `path::symbol` (the symbol must appear in that file). Point at the implementing function/component or a test, not at sample data or docs. Required non-empty for `done`/`partial`; `[]` for todo (a todo may cite code that shows the gap, explained in `notes`). Every path must exist. |
| `competitors` | no | Map `{vqa, vqp, vega, se, cv}` → `'yes'`/`'partial'`/`'no'` (VQ Analyzer, VQ Probe, VEGA, StreamEye, CodecVisa): whether that product has the feature. |
| `legacy_ids` | no | **List** of old doc ids (OV-01, CMP-03, INT-02, PNL-07, L1-AV1-03 …). A merged item lists every old id. Code comments, commits and `docs/history/*` still use these; grep them. |
| `source` | yes | **List** of `"<DOC>.md §<section>"` strings (always with the doc name and `§`), e.g. `PARITY_CHECKLIST.md §Layer 8`. Merged duplicates list every source. Sections may refer to the pre-2026-10-05 doc layout (see the docs' git history); V14 import files are noted in parentheses. |
| `notes` | no | Gaps, caveats, and conflicts resolved ("doc said X; code: Y"). Required for `dropped`. |

## Status meanings

- **done**: implemented and reachable by a user, through the desktop app (sidecar command + mounted UI) or the
  `bitvue` CLI / MCP server, and the acceptance is met.
  - Codec-agnostic features that work end-to-end for AV1 count as done. The desktop pipeline is IVF/AV1-only;
    that limit is tracked once, as `INFRA-001`.
  - Must be reachable on **every desktop platform** (macOS native menu AND the Win/Linux TitleBar). Reachable
    through only one platform's menu is **partial** (decided 2026-10-05); say which platform works in `notes`.
    A shortcut or menu item that dispatches an event nobody listens for is not reachable.
- **partial**: any of the following holds.
  - Some of the acceptance is met.
  - The feature is engine/crate-only: no sidecar command, the UI isn't mounted, or it's dead Tauri `invoke()` code.
  - A codec-specific item works for only some of its codecs.
- **todo**: not started.
- **dropped**: intentionally out of scope, or obsolete. The reason goes in `notes`.

## How to update (agents and humans)

1. Find the item: `grep -n "<keyword>\|<legacy id>" docs/specs/features.yaml`. Edit it in place. Keep the `id` stable.
2. When the status changes, add or adjust `evidence` with real paths (a test name is better than a file). Put
   the reason in `notes`, briefly. Verify against the code, not against another doc.
3. For a new requirement, append it within its area block with the next free number, and fill in `source`.
4. Validate:
   ```bash
   python3 - <<'EOF'
   import yaml, os, re, collections
   d = yaml.safe_load(open('docs/specs/features.yaml')); ids = [i['id'] for i in d['items']]
   assert len(ids) == len(set(ids)), [k for k, v in collections.Counter(ids).items() if v > 1]
   bad = [(i['id'], e) for i in d['items'] for e in i.get('evidence') or []
          if not os.path.exists(re.sub(r':\d+(-\d+)?$', '', str(e).split('::')[0]))]
   print(len(ids), 'items; missing evidence:', bad)
   EOF
   ```
5. Bump `updated:` at the top of the file.

Snapshot on 2026-10-05 (after deep verification): 377 items, with 103 done, 162 partial, 106 todo and 6 dropped (after the 2026-10-05 one-platform rule). Per-phase counts are in
`docs/DEVELOPMENT_PHASES.md`, and per-product counts are in `docs/COMPETITOR_FEATURE_MATRIX.md`.
