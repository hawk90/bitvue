# Overlay extraction API

Overlay = per-block QP, MV, partition, prediction mode, or transform data drawn over the frame. Colour scales
and UI contracts live in `docs/UX_PARITY_MATRIX.md`.

## Shared grid types (`crates/bitvue-engine/src/`)

| Type | File | Key fields |
|---|---|---|
| `QPGrid` | `qp_heatmap.rs` | `grid_w`, `grid_h`, `block_w`, `block_h`, `qp: Vec<i16>`, `qp_min`, `qp_max`, `missing`; methods `get(bx, by)`, `coverage_percent()` |
| `MVGrid` | `mv_overlay.rs:114` | `coded_width`/`coded_height`, `block_w`/`block_h`, `grid_w`/`grid_h`, `mv_l0`, `mv_l1`, `mode: Option<Vec<BlockMode>>` |
| `MotionVector` | `mv_overlay.rs` | `dx_qpel`, `dy_qpel` (**quarter-pel**); `MISSING`, `ZERO`, `to_pixels()` |
| `PartitionGrid` | `partition_grid.rs` | `coded_width`/`coded_height`, `sb_size`, `blocks: Vec<PartitionBlock { x, y, width, height, partition, .. }>` |

Frontend mirror: the renderer receives these grids as JSON with the same snake_case fields; see `QPGrid` and
siblings in `frontend/types/video.ts` (`qp: number[]`, row-major, `-1` = missing).

Edge handling: `grid_w = ceil(coded_width / block_w)` (same for height), so the last column and row can be
partial blocks. When you check an extractor, assert that the grid dimensions match the frame dimensions and
that the vector lengths equal `grid_w * grid_h`. Clip drawing at the frame boundary.

## AV1: `bitvue_av1_codec::overlay_extraction` (`crates/bitvue-av1-codec/src/overlay_extraction/`)

This is the only overlay path wired into the app. Sidecar callers: `frame_analysis.rs`, `coding_flow.rs`,
`codec_extended_info.rs`, `deblocking.rs`, `residual_analysis.rs`.

| Function | Returns |
|---|---|
| `extract_qp_grid(obu_data, frame_index, base_qp: i16)` | `Result<QPGrid, BitvueError>` |
| `extract_qp_grid_from_parsed(&ParsedFrame, frame_index, base_qp)` | `QPGrid` |
| `extract_mv_grid(obu_data, frame_index)` / `extract_mv_grid_from_parsed(&ParsedFrame)` | `MVGrid` |
| `extract_partition_grid(obu_data, frame_index)` / `_from_parsed` | `PartitionGrid` |
| `extract_prediction_mode_grid(obu_data, frame_index)` / `_from_parsed` | `PredictionModeGrid` |
| `extract_transform_grid(obu_data, frame_index)` / `_from_parsed` | `TransformGrid` |
| `extract_energy_grid_from_parsed`, `extract_deblocking_data_from_parsed` | `EnergyGrid`, `DeblockingData` |
| `extract_pixel_info(...)` | `PixelInfo` (hover tooltip) |

### How to call it correctly

This is the pattern in `crates/bitvue-sidecar/src/frame_analysis.rs::get_frame_analysis`:

```rust
let (_hdr, frames) = bitvue_av1_codec::parse_ivf_frames(&data)?;
// Prepend the sequence header bytes to the target frame's OBUs.
let obu_data = [seq_bytes.as_slice(), frames[i].data.as_slice()].concat();
// Thread reference state from frame 0 up to i; ParsedFrame::parse is correct only for frame 0.
let mut ref_state = bitvue_av1_codec::frame_header_full::thread_ref_state_before(&frames, &seq, i)?;
let parsed = ParsedFrame::parse_with_ref_state(&obu_data, &mut ref_state)?;
let base_qp = parsed.frame_type.base_qp.unwrap_or(0) as i16;
let qp = extract_qp_grid_from_parsed(&parsed, i, base_qp)?;
let mv = extract_mv_grid_from_parsed(&parsed)?;
```

- The convenience functions that are not `_from_parsed` call `ParsedFrame::parse`, which uses a fresh
  `RefFrameState`. On later frames that need `skip_mode_params`, the tile data offset is wrong and you get 0
  coding units. Use them only in tests or on frame 0.
- By default, extraction falls back to scaffold data when tile parsing fails.
  `overlay_extraction::set_strict_mode(true)` makes those failures propagate as errors, which is useful in
  tests.
- Coding units are cached in an LRU (`cache.rs`: `get_or_parse_coding_units`, `clear_cu_cache`, `cu_cache_size`).
- Bench: `cargo bench -p bitvue-av1-codec --bench overlay_extraction`.
- Tests: `crates/bitvue-av1-codec/tests/{overlay_extraction_test.rs, overlay_extraction_integration_test.rs, mv_extraction_test.rs}`.

## NAL codecs: take `&[NalUnit]` + `&Sps`

| Crate | Functions | Notes |
|---|---|---|
| `bitvue_avc` | `extract_qp_grid(nals, sps, base_qp)`, `extract_mv_grid(nals, sps)`, `extract_partition_grid(nals, sps)`, `extract_prediction_mode_grid`, `extract_mb_type_grid`, `extract_ref_idx_grid` | 16×16 MB grid. The last three return tuples `(grid_w, grid_h, block_w, block_h, Vec<Option<_>>)` |
| `bitvue_hevc` | `extract_qp_grid(nals, sps, base_qp)`, `extract_mv_grid`, `extract_partition_grid`, `extract_prediction_mode_grid` (tuple) | CTU from SPS |
| `bitvue_vvc` | `extract_qp_grid(nals, sps, base_qp)`, `extract_mv_grid`, `extract_partition_grid` | MTT partitioning. Falls back to a synthetic H/V split pattern when parsing fails |

None of these are called from the sidecar, CLI, or MCP; only the crate tests call them. Wiring one in means
adding a sidecar command; see `request_dispatch.rs`.

## Other codecs

| Crate | Functions | Fidelity |
|---|---|---|
| `bitvue_vp9` | `extract_qp_grid(&FrameHeader)`, `extract_qp_grid_with_data(&FrameHeader, Option<&[u8]>)`, `extract_mv_grid(&FrameHeader)`, `extract_partition_grid(&FrameHeader)`, `decode_segment_ids_from_tiles` | Superblock-level, derived from the frame header. Per-block data only via segment IDs (`_with_data`). The VP9 `MotionVector` (integer pel, per its doc) is copied straight into the quarter-pel `MotionVector`, so check the units before trusting MV magnitudes |
| `bitvue_avs3` | `extract_qp_grid(&Avs3Frame, Option<&SequenceHeader>) -> Option<Avs3QpGrid>`, `extract_esao_map`, `extract_ccsao_map` | Picture-level proxy; real AEC decoding is pending (crate doc) |
| `bitvue_jpegxs` | `extract_precinct_map`, `extract_dequant_map`, `extract_transform_map`, `extract_mct_info`, `extract_nlt_info` (take `&JpegXsFrame`) | Header-derived |
| `bitvue_vc3` | `extract_mb_grid(&Vc3Frame) -> Option<MbGrid>` | Header-derived |
| `bitvue_mpeg2_codec` | none | none |
