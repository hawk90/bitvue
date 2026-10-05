# Bitvue — Competitor Feature Matrix

> 2026-07-31; tables moved into `docs/specs/features.yaml` on 2026-10-05. Compacted from in-session web research
> (5 subagent deep-dives) on VQ Analyzer (ViCueSoft User Guide, release notes v5.1–v7.8), VQ Probe (ViCueSoft
> product page), VEGA Media Analyzer (Interra Systems site), StreamEye (Elecard product page/PDF) and
> CodecVisa/Pelscope (Codecian site, dated 2017, legacy/low-confidence). Raw per-product notes lived only in
> the research transcript. This file holds the durable provenance and rationale.
> **See also:** `docs/specs/features.yaml` (**source of truth for every feature row and Bitvue status**),
> `VQA_PARITY_SPEC_V3.md` (§1.5 priority rationale, §4.4 F-key numbering), `PARITY_CHECKLIST.md`,
> `UX_PARITY_MATRIX.md`, `DEVELOPMENT_PHASES.md`.

Products: **VQA** = VQ Analyzer, **VQP** = VQ Probe, **VEGA** = Interra VEGA Media Analyzer, **SE** = Elecard StreamEye,
**CV** = Codecian CodecVisa/Pelscope. In features.yaml each item carries `competitors: {vqa, vqp, vega, se, cv}`.
A product is marked `yes` when the research listed it as a source for that feature.

---

## Where the rows went

> Feature/status items for this doc live in `docs/specs/features.yaml`. Old sections map to these areas:
> §1 per-codec overlay modes → area `overlay` (+ `codec`/`decode` for codec support). §2 CLI → area `cli`/`export`.
> §3 quality metrics → area `metrics`. §4 compare → area `compare`. §5 misc tools → `ui`/`syntax`/`container`/`decode`.
> §6 backlog phasing → `phase:` field. Legacy ids: OV-01..10, CMP-01..10, IA-02/03/05/06, L1-AV1-04/05, INT-01.

Keep F-key numbering in sync with `VQA_PARITY_SPEC_V3.md` §4.4 and `frontend/utils/codecModeRegistry.ts`.

### Per-product summary (features.yaml, 2026-10-05 verification)

Computed from `docs/specs/features.yaml`: for each product key, count items whose `competitors.<key>` is `yes`, grouped by `status`.

| Product | Features sourced | done | partial | todo | dropped |
|---|---|---|---|---|---|
| VQA | 117 | 33 | 59 | 25 | 0 |
| VQP | 26 | 10 | 11 | 2 | 3 |
| VEGA | 64 | 16 | 36 | 9 | 3 |
| SE | 62 | 20 | 32 | 10 | 0 |
| CV | 20 | 4 | 12 | 2 | 2 |

Most `partial` rows are engine-only: the crate parser or overlay extractor exists, but the desktop pipeline
(sidecar/indexer) is IVF/AV1-only (INFRA-001). As of this table: compare views (CMP-001..004) and BD-rate via CLI (MET-001) are `done`; YUVDiff (CMP-011, PSNR-U/V not
shown) is `partial`; RD-curve plotting (MET-008), VMAF (MET-002/003), CABAC visualization (OVL-011) and
APV (CODEC-005) are open. Check the YAML ids for current status.

---

## Architecture note — competitor CLI styles

StreamEye's CLI is XML-config-driven (`SEyeConsole.exe config.xml /in:<path> /out:<path>`), not flag-based. That is
structurally different from VQA's flag style and Bitvue's subcommand style (`bitvue decode|info|frames|analyze|quality|export|batch|validate|bd-rate|evidence-diff`),
so it is not a parity target. VEGA's CLI is Docker-deployed, claims 5x realtime 4K/2K throughput and emits XML
conformance reports (SPS/PPS/VPS/SEI/Slice). Its focus is broadcast conformance, so it is mostly out of scope.
CV has no CLI/automation and no keyboard shortcuts, which makes Bitvue's CLI and shortcut set a relative strength.
CV also has no PSNR/SSIM/VMAF support at all. That is a CodecVisa limitation, not a Bitvue gap.

Alignment correctness rules for A/B compare (alignment axes, mismatch handling) are specified in `UX_PARITY_MATRIX.md` §2.1.
VMAF sub-scores (ADM2/VIF/motion2) are a Bitvue proposal beyond the competitor baseline, not a competitor feature.

## Out of scope (status `dropped` in features.yaml)

Bitvue is a codec bitstream *analyzer*, not a live broadcast/TS *monitoring probe* (`VQA_PARITY_SPEC_V3.md` §1.5).
Dropped:
- Conformance suites: TR101290/CableLabs/ARIB/HbbTV/ATSC3/CMAF, T-STD, VEGA AV1 Stream Compliance Assurance, CV HEVC constraint checks.
- Broadcast/ABR QC: closed captions, blockiness/black-frame/freeze, audio loudness/silence/CALM.
- ABR ladder tooling: Convex Hull, QP variation across ABR renditions.
- Bundled utilities: Any2Hevc, Pelscope.
- Unverified marketing claims: VEGA "AI/ML anomaly detection".

Deliberately left as low-priority `todo` (niche or low-confidence, not out of scope): the cross-codec comparison viewer, the
per-block "interpolation" overlay, CV-sourced bit-numbers-per-CU/binary CU-bit view, H.264 Data Partitions, and MMT.
