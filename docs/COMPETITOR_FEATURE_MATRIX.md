# Bitvue — Competitor Feature Matrix

> 2026-07-31. Compacted from in-session web research (5 subagent deep-dives) on VQ Analyzer (ViCueSoft User
> Guide, release notes v5.1–v7.8), VQ Probe (ViCueSoft product page), VEGA Media Analyzer (Interra Systems
> site), StreamEye (Elecard product/PDF), CodecVisa/Pelscope (Codecian site, dated 2017 — legacy/low-confidence).
> Raw per-product notes lived only in the research transcript; this file is the durable record. **Supersedes**
> the prose in `VQA_PARITY_SPEC_V3.md`'s old §1.5 and Appendix C — those sections now just point here.
> See also: `VQA_PARITY_SPEC_V3.md` (codec/backend parity spec), `PARITY_CHECKLIST.md` (implementation
> tracking — source of truth for Bitvue ✅/⚠️/❌ marks below), `UX_PARITY_MATRIX.md` (UI/UX interaction parity),
> `DEVELOPMENT_PHASES.md` (Phase 0-12 implementation roadmap).

Status marks: ✅ implemented (cites a `PARITY_CHECKLIST.md` ID or confirmed via code grep) · ⚠️ unverified /
partial (cross-check doc exists but completion unconfirmed) · ❌ confirmed gap · **—** not a Bitvue-relevant
target (bundled tool, product limitation, or explicitly out of scope).

Products: **VQA**=VQ Analyzer, **VQP**=VQ Probe, **VEGA**=Interra VEGA Media Analyzer, **SE**=Elecard
StreamEye, **CV**=Codecian CodecVisa/Pelscope.

---

## 1. Per-codec overlay / visualization modes

> Canonical Bitvue F-key ↔ mode-name numbering (e.g. HEVC F1=Coding Flow, F2=Predictions...) lives in
> `VQA_PARITY_SPEC_V3.md` §4.4 (trimmed 2026-07-31 to just the numbering — this section is the full detail/
> comparison view). Keep both in sync when adding/renumbering a mode.

### HEVC
| Mode | Source(s) | Bitvue status |
|---|---|---|
| Coding Flow (CTU/CU/PU tree) | VQA | ✅ OV-03 Partition grid |
| Predictions (+Detail popup) | VQA | ✅ OV-06 Prediction mode; detail popup ❌ |
| Transform (+Detail popup) | VQA | ⚠️ OV-04 CBF Luma (partial); TU detail popup ❌ |
| Reconstruction (+Detail) | VQA | ⚠️ unverified — no dedicated checklist ID |
| Loop Filter (+Detail, deblock BS colors) | VQA, VEGA (in-loop-filter viz) | ❌ not tracked |
| SAO (+Detail) | VQA | ❌ not tracked |
| YUV (plain decode) | VQA, all | ✅ base decode view |
| Heat Map (bit-cost) | VQA, SE (bit-size color map), CV (MB-bits heatmap) | ⚠️ TransformRenderer partial (spec Phase 2) |
| MV Heat | VQA | ✅ OV-02 MV Field (plain); magnitude-heat coloring unverified |
| QP Map | VQA, VEGA, SE | ✅ OV-01 |
| PSNR overlay (per-block) | VQA, SE (metrics-in-ROI) | ❌ CMP-06/07 |
| SSIM overlay (per-block) | VQA | ❌ CMP-06/07 |
| PU Type | VQA | ⚠️ PredictionRenderer partial (spec Phase 2) |
| PU Reference Indices | VQA, VEGA (ref-index overlay) | ⚠️ HEVC PU-level unverified (AVC MB-level ✅, see below) |
| Simple Motion | VQA | ⚠️ unverified (spec Phase 2) |
| CABAC range/state visualization | VQA (v6.1+), VEGA | ⚠️ CMP-10 unverified |

### VVC
| Mode | Source | Bitvue status |
|---|---|---|
| Dual Tree | VQA | ❌ vvdec not connected — decode-level gap (spec §3.1) |
| Coding Flow / Predictions (CCLM/MRLP/IBC/GPM) | VQA | ❌ same VVC decode gap |
| Transform (MTS/LFNST) / Reconstruction | VQA | ❌ same gap |
| Inverse Map (LMCS) | VQA | ❌ same gap |
| Loop Filter / SAO / Adaptive Filter (ALF) | VQA | ❌ same gap |
| YUV | VQA | ⚠️ parsing only, no decode |
| QP Map / Heat Map | VQA | ❌ same gap |
| Inter Memory Reads | VQA | ❌ VVC-exclusive, niche |

### AV1
| Mode | Source | Bitvue status |
|---|---|---|
| Coding Flow | VQA | ✅ OV-03 |
| Predictions | VQA | ✅ OV-06 |
| Transform | VQA | ✅ OV-04/OV-05 (CBF + transform type) |
| Reconstruction | VQA | ⚠️ unverified |
| Loop Filter (combined view) | VQA | ⚠️ unverified (Bitvue splits into CDEF/LR below) |
| CDEF Filter | VQA | ✅ OV-07 |
| SuperRes Filter | VQA | ✅ OV-10 |
| Loop Restoration Filter | VQA | ✅ OV-08 |
| Film Grain Pixels | VQA | ✅ OV-09 |
| YUV | VQA | ✅ base view |
| Heat Map | VQA, SE, CV | ⚠️ unverified |
| Efficiency Map | VQA, SE | ❌ not tracked |
| Block Type | VQA, SE | ⚠️ unverified |
| PSNR | VQA, SE | ❌ CMP-06/07 |
| Simple Motion | VQA | ⚠️ unverified |

### VP9
| Mode | Source | Bitvue status |
|---|---|---|
| QP Map | VQA, SE | ✅ OV-01 |
| Coding Flow / Partition grid | VQA | ❌ OV-03 excludes VP9 |
| MV Field | VQA | ❌ OV-02 excludes VP9 |
| Predictions | VQA | ❌ OV-06 excludes VP9 |
| Transform / Reconstruction | VQA | ❌ OV-04/05 exclude VP9 |
| Loop Filter | VQA | ❌ not tracked |
| Heat Map | VQA, SE, CV | ⚠️ unverified |
| Block Type | VQA, SE | ⚠️ unverified |
| Efficiency Map | VQA, SE | ❌ not tracked |
| PSNR | VQA, SE | ❌ CMP-06/07 |

### AVC / H.264
| Mode | Source | Bitvue status |
|---|---|---|
| QP Map | VQA, VEGA | ✅ OV-01 |
| MV Field | VQA | ✅ OV-02 |
| Predictions | VQA | ✅ OV-06 |
| Coding Flow / Partition grid | VQA | ❌ OV-03 excludes AVC |
| Transform / CBF | VQA | ❌ OV-04/05 exclude AVC |
| Loop Filter | VQA | ❌ not tracked |
| MB Type | VQA, VEGA | ✅ `AvcMbTypeRenderer.tsx` (commit db5a308, VQA Phase 2) |
| MB Reference Indices | VQA, VEGA | ✅ `reference-indices` overlay case confirmed in `OverlayRenderer/index.tsx` (commit db5a308) |
| YUV | VQA | ✅ base view |
| Heat Map | VQA, SE, CV | ⚠️ unverified |
| PSNR | VQA, SE | ❌ CMP-06/07 |

### MPEG-2
| Mode | Source | Bitvue status |
|---|---|---|
| Predictions / Transform / YUV | VQA | ⚠️ decode not connected (spec §3.1 "파싱만") |
| QP Map | VQA | ❌ decode gap makes overlay moot |
| Simple Motion | VQA | ❌ same |

### AVS3 / JPEG XS / APV / VC-3 / AVM
| Codec | Modes (source: VQA) | Bitvue status |
|---|---|---|
| AVS3 | Prediction, Transform, Reconstruction, Loop Filter, SAO, ESAO, CCSAO, Adaptive Filter, YUV + Heat/QpY/CoeffsY/PSNR/SSIM/Statistics | ❌ codec unimplemented (spec §3.1) |
| JPEG XS | Precinct, Dequant, Transform, MCT, NLT, YUV | ❌ codec unimplemented |
| APV | decode support added VQA v7.7/7.8, no published named-overlay list (gated) | ❌ codec unimplemented (CMP-08) |
| VC-3 / DNxHD | decode support added VQA v7.5+, macroblock/QSF/ACF params | ❌ codec unimplemented |
| AVM | VQA official name for AOM next-gen experimental codec (v7.5+) | ⚠️ naming reconciliation vs Bitvue "AV3" unresolved (CMP-09) |

### Global cross-codec
| Feature | Source | Bitvue status |
|---|---|---|
| Dual View (2 bitstreams synced + delta diff) | VQA, SE (Compare) | ❌ CMP-01/02 |
| Debug YUV (PSNR/SSIM vs reference, find-first-diff) | VQA | ⚠️ planned §4.7; find-first-diff ❌ |
| Thumbnails/Bars Filmstrip | VQA, all | ✅ IA-02 |
| HRD/VBV buffer plot | VQA | ⚠️ spec'd §4.1, build unverified |
| Stats export-to-file | VQA | ✅ Layer 5 JSON export / `--stats` (partial) |

---

## 2. CLI / automation

| Flag / tool | Source | Bitvue status |
|---|---|---|
| Force-codec flags (`-hevc`/`-vp9`/`-av1`/`-avc`/`-vvc`/`-mpeg2`/`-avs3`/`-jxs`/`-vc3`/`-apv`/`-avm`/`-yuv`) | VQA | ⚠️ unverified — Phase 9 roadmap unchecked; auto-detect exists (L1-*-05) |
| `-o <file>` (YUV output) | VQA | ✅ `bitvue decode -o` |
| `-y4m` | VQA | ✅ `bitvue decode --y4m` |
| `-n <frame>` (seek) | VQA | ⚠️ unverified |
| `-debug_yuv <file>` / `-dependent <file>` | VQA | ⚠️ unverified (§4.7 planned) |
| `-regress` (headless) | VQA | ⚠️ CLI is headless by nature; explicit flag unconfirmed |
| `-frames <n>` | VQA | ⚠️ Phase 9 unchecked (L1-AV1-04 max-frames exists, flag name unconfirmed) |
| `-md5` | VQA | ✅ `--md5` |
| `-psnr` | VQA | ✅ `--psnr --reference` |
| `-extract <pattern>` / `-dump` / `-dump_mode <stage>` | VQA | ❌ not tracked / Phase 9 unchecked |
| `-nocrop` / `-errors <file>` / `-fast <level>` | VQA | ❌ Phase 9 unchecked |
| `-stats <file>` | VQA | ⚠️ JSON export may partially cover |
| `-syntax_stats <file>` / `-syntax_count` | VQA | ❌ not tracked |
| `-norm_pix` / `-norm_bits` / `-percent` | VQA | ❌ not tracked |
| `-display_order` | VQA | ❌ Phase 9 unchecked — ties to `CONTRACT_DISPLAY_DECODE_SEPARATION` gap in `UX_PARITY_MATRIX.md` |
| `-dump_bitdepth <n>` / `-cpu_max_feature` | VQA | ❌ Phase 9 unchecked |
| `-film_grain` (AV1 pre/post dump) | VQA | ❌ Phase 9 unchecked |
| `-dump_headers` / `-dump_headers_filter` | VQA | ❌ not tracked |
| `-oppoint` (AV1 operating point) / `-ols` (VVC output layer set) | VQA | ❌ not tracked (VVC undecoded anyway) |
| `-track` / `-library_stream` (AVS3) / `-mv-to-csv` | VQA | ❌ not tracked / N/A (AVS3 unimplemented) |
| Batch mode | VQP (CLI batch), VEGA (Docker/batch + XML reports), SE (XML-config-driven) | ✅ `bitvue batch` (Layer 5) — see architecture note below |
| JSON export | Bitvue-native | ✅ `bitvue export --json` |

**Architecture note:** StreamEye's CLI is XML-config-driven (`SEyeConsole.exe config.xml /in:<path> /out:<path>`),
not flag-based — structurally different from VQA's and Bitvue's flag/subcommand style. Not a parity target,
noted for awareness only. VEGA CLI is Docker-deployed, claims 5x realtime 4K/2K throughput and emits XML
conformance reports (SPS/PPS/VPS/SEI/Slice) — broadcast-conformance focus, mostly out of scope (§1.5 rationale,
`VQA_PARITY_SPEC_V3.md`).

---

## 3. Quality metrics

| Metric | Source | Bitvue status |
|---|---|---|
| PSNR | VQA, VQP, VEGA, SE | ⚠️ planned §4.7; CLI `--psnr` ✅ but YUVDiff-panel PSNR unverified |
| SSIM | VQA, VQP, SE | ⚠️ planned §4.7, unverified |
| VMAF (pooled + per-frame) | VQP, SE | ❌ CMP-06 |
| VMAF phone | SE | ❌ CMP-07 (nice-to-have) |
| VMAF sub-scores (ADM2/VIF/motion2) | (Bitvue proposal, beyond baseline) | ❌ CMP-07 |
| APSNR / DELTA / MSE / MSAD / VQM / NQI / EPSNR / VIF | SE | ❌ not tracked |
| Metrics-in-ROI | VQP, SE | ❌ not tracked |
| RD-curve plotting | VQP | ⚠️ RDCurvesPanel exists (CMP-05) |
| BD-Rate calculation | VQP | ❌ CMP-05 |
| Convex Hull (ABR ladder optimization) | VQP | ❌ likely out of scope — ABR encode-ladder tuning, not bitstream analysis |
| Scene change detection | VQP | ❌ not tracked (also flagged in `UX_PARITY_MATRIX.md` §1) |
| PSNR/SSIM/VMAF support | CV | **—** confirmed absent even in CodecVisa itself; not a Bitvue gap, note as CV limitation |

---

## 4. Compare / dual-stream features

| Feature | Source | Bitvue status |
|---|---|---|
| Side-by-side sync playback (2 streams) | VQA (Dual View), VQP, VEGA (Comparison Viewer), SE (Compare) | ❌ CMP-01/02 |
| Split view H/V w/ slider | SE (Horizontal/Vertical Split) | ❌ CMP-02 |
| Overlapped / Independent view | VQP | ❌ CMP-02 variant |
| Subtraction (\|A−B\| pixel diff) | SE (Subtraction), VQP (B&W diff) | ❌ CMP-03 |
| Temperature / heatmap diff | SE (Temperature), VQP (Heat Map diff) | ❌ CMP-03 |
| PSNR / PSNR Clip (stream-vs-stream) | SE | ❌ CMP-06 (shared metric infra) |
| Find First Difference (stream vs stream) | VQA (dual/dependent stream), SE | ❌ CMP-04 |
| Cross-codec compare (HEVC/H264/VP9 in one view) | VEGA (Comparison Viewer) | ❌ not tracked, niche |
| QP-variation-across-ABR-renditions | VEGA | ❌ out of scope (ABR/broadcast QC) |
| CABAC range/state visualization | VQA, VEGA | ⚠️ CMP-10 unverified |

Alignment correctness rules for A/B compare are specified in `UX_PARITY_MATRIX.md` §2.1 (alignment axes,
mismatch handling) — directly implements CMP-01..05.

---

## 5. Misc distinctive tools

| Tool | Source | Bitvue status |
|---|---|---|
| Buffer Analyzer (CPB/T-STD conformance) | VEGA | ⚠️ HRD buffer graph spec'd §4.1; conformance-checking layer not tracked |
| Trace Viewer (syntax↔hex linked inspector) | VEGA | ⚠️ IA-03+IA-05 exist; bidirectional linking is Phase 8 in-progress (spec §4.6) |
| Comparison Viewer (cross-codec bitrate/QP/buffer/MV/blockiness) | VEGA | ❌ not tracked, overlaps §4 CMP items above |
| Error Log Viewer (XML/PDF export) | VEGA | ❌ Status panel exists (IA-06) but no structured export |
| Quad Tree view (block-split structure) | VEGA | ✅ overlaps OV-03 Partition grid (AV1/HEVC only) |
| Per-block overlay (coded bits/prediction/MV/QP/interpolation/ref-index) | VEGA | ⚠️ mostly covered piecemeal (OV-01/02/06 + AVC MB Type/RefIdx); "interpolation" sub-mode not tracked |
| Analytical graphs (bitrate/frame-dist/compression-ratio/QP/DPB-occupancy) | VEGA | ⚠️ FrameSizesView covers bitrate/QP; DPB-occupancy graph not tracked |
| In-loop-filter + intra-prediction process visualization | VEGA | ⚠️ overlaps HEVC Loop Filter/SAO gaps above |
| Pixel-value-at-every-decode-stage display (pre-deblock/predicted/residual/final) | VEGA, CV | ❌ not tracked |
| Detailed residue view (HEVC/H264) | VEGA | ⚠️ Transform/CBF renderer partial overlap |
| Closed-caption visualization | VEGA | ❌ out of scope (§1.5 broadcast-QC exclusion) |
| HEVC SCC extension / RExt (4:2:2/4:4:4) | VEGA, VQA | ⚠️ listed in spec §3.3, decode-level support unverified |
| Conformance checks (TR101290/CableLabs/ARIB/HbbTV/ATSC3/CMAF) | VEGA | ❌ explicitly out of scope (§1.5 rationale) |
| ABR quality checks (blockiness/black-frame/freeze/loudness/silence/CALM) | VEGA | ❌ out of scope (broadcast/ABR QC + audio loudness) |
| AV1 Analyzer: Access Unit View / Graph View (compression-ratio, OBU-size) | VEGA | ⚠️ AU view overlaps IA-03; standalone compression-ratio/OBU-size graph not tracked |
| AV1 Analyzer: Frame Buffer / MV / Coefficient / Filtering View | VEGA | ⚠️ mostly overlaps OV-01..10 AV1 overlays (already ✅); standalone "view" framing not tracked |
| AV1 Analyzer: Stream Compliance Assurance | VEGA | ❌ out of scope (compliance/QC framing) |
| Hex viewer (offset/bytes/ASCII) | SE, VEGA (Unit Info) | ✅ IA-05 HexViewTab |
| Header text/syntax view | SE, VEGA | ✅ IA-03 SyntaxPanel |
| Bit distribution visualization | SE | ❌ not tracked |
| GOP thumbnail nav | SE | ✅ IA-02 Filmstrip |
| Color-gamut switch (BT.601/709/2020) | SE (YUV Viewer) | ⚠️ described as target UI in `UX_PARITY_MATRIX.md` §10 Options menu (moved from spec §2.2); build status unverified |
| Endianness selection, 20+ raw pixel formats | SE (YUV Viewer) | ❌ not tracked |
| Container breadth (MPEG-1 System, MXF, HEIC, DASH-MPD, etc.) | SE | ⚠️ spec §3.2 — MXF/AVI/MMT explicitly ❌; HEIC/DASH-MPD not tracked at all |
| Bit-numbers-per-CU/MB, binary view of CU/MB bits, per-block-type bit histogram | CV | ❌ not tracked (legacy/2017 source, low confidence) |
| Any2Hevc transcoder / Pelscope (bundled utilities) | CV | **—** bundled tool, not an analyzer feature; N/A for Bitvue |
| H.264 Data Partitions support | CV | ❌ not tracked, niche |
| Conformance check (HEVC syntax/constraints) | CV | ❌ not tracked |
| No CLI/automation, no keyboard shortcuts (CV limitation) | CV | **—** highlights Bitvue's CLI (Layer 5 ✅) and full shortcut set (Layer 4 ✅) as relative strengths |

---

## 6. New backlog items surfaced (not yet in `PARITY_CHECKLIST.md`)

Beyond the existing CMP-01..10 (Layer 6): ROI-based metrics, bit-distribution visualization, decode-stage
pixel display (pre-deblock/predicted/residual/final), structured error-log export (XML/PDF or JSON
equivalent), YUV-viewer color-gamut/endianness/raw-pixel-format options, and MXF/HEIC/DASH-MPD container
support. Recommend scoping into a future checklist layer once prioritized — not added speculatively here.
