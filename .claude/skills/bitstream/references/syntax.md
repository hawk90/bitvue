# Syntax units and real types

For codec theory (tools, profiles, intra modes), see the `video-codec-expert` agent. This file lists what the
parsers in this repo actually expose.

## AV1: `bitvue_av1_codec` (`crates/bitvue-av1-codec/src/`)

### OBU header and `ObuType` (`obu.rs`)

Header byte layout: `forbidden(1) | obu_type(4) | extension_flag(1) | has_size_field(1) | reserved(1)`. If
`extension_flag` is set, an extension byte follows with `temporal_id(3) | spatial_id(2) | reserved(3)`. When
`has_size_field` is set, the payload size follows as a leb128 value (`decode_uleb128`, `leb128_size`).

| obu_type | `ObuType` |
|---|---|
| 0 | `Reserved0` |
| 1 | `SequenceHeader` |
| 2 | `TemporalDelimiter` |
| 3 | `FrameHeader` |
| 4 | `TileGroup` |
| 5 | `Metadata` |
| 6 | `Frame` (header + tile group) |
| 7 | `RedundantFrameHeader` |
| 8 | `TileList` |
| 9–14 | `Reserved9`…`Reserved14` |
| 15 | `Padding` |

`ObuHeader` fields: `obu_type`, `has_extension`, `has_size`, `temporal_id`, `spatial_id`, `header_size` (1 or 2).

`Obu` fields:
- `header`
- `payload_size`, `total_size`, `offset` (all `u64`, byte offset in the input)
- `payload: Arc<[u8]>`, skipped by serde
- `frame_type: Option<FrameType>` and `frame_header: Option<FrameHeader>`, set only for `Frame`/`FrameHeader` OBUs

Entry points:
- `parse_obu_header(&mut BitReader)`
- `parse_obu(data, offset) -> Result<(Obu, usize)>`
- `parse_all_obus(data)`: strict, stops on the first error
- `parse_all_obus_resilient(data, StreamId)`: see `debugging.md`
- `ObuIterator::new(data)`, with `.next_obu_with_offset()`

The payload size is capped at 100 MB per OBU (`MAX_OBU_PAYLOAD_SIZE`).

### Headers

- `parse_sequence_header(payload) -> SequenceHeader` (`sequence.rs`). Fields include `profile: Av1Profile`,
  `still_picture`, `reduced_still_picture_header`, `operating_points`, `max_frame_width`/`max_frame_height`,
  `use_128x128_superblock`, `enable_order_hint`, `enable_ref_frame_mvs`, and `color_config`.
- `parse_frame_header_basic(payload) -> FrameHeader` (`frame_header.rs`). Fields include `frame_type`,
  `show_frame`, `show_existing_frame`, `error_resilient_mode`, `base_q_idx: Option<u8>`, `delta_q_present`,
  `refresh_frame_flags: Option<u8>`, `ref_frame_idx: Option<[u8; 7]>`, `order_hint`, `width`/`height`,
  `upscaled_width`, and `header_size_bytes`.
- `frame_header_full.rs`: `parse_frame_header_full`, `RefFrameState`, and `thread_ref_state_before`. Use these
  when a frame's correct parse depends on earlier frames (order hints, skip mode). See `overlay-api.md`.
- `FrameType` is `bitvue_engine::FrameType`: `Key`, `Inter`, `BFrame`, `IntraOnly`, `Switch`, `SI`, `SP`,
  `Unknown`.

### Syntax trees (bit-accurate, for the UI syntax panel)

`syntax_parser/`:
- `parse_obu_syntax(data, obu_index, global_offset) -> SyntaxModel`. `global_offset` is a **bit** offset from the start of the file (the indexer passes `byte_offset * 8`).
- `parse_bitstream_syntax(data) -> Vec<SyntaxModel>`
- `parse_sequence_header_syntax`, `parse_frame_header_syntax`
- `TrackedBitReader` records the bit range of each field

The sidecar exposes these trees through `get_frame_syntax`; the CLI exposes them through `analyze --syntax`.

### Tiles and blocks (`tile/`)

- `parse_tile_group`, `parse_superblock`, `parse_partition_tree`, `parse_coding_unit`, `partition_tree_to_grid`
- Types: `BlockSize`, `PartitionType`, `PredictionMode`, `RefFrame`, `MotionVector`, `SuperblockSize`
- The symbol decoder is in `symbol/`: `SymbolDecoder`, `ArithmeticDecoder`, `CdfContext`
- `PartitionType` (0–9): `None`, `Horz`, `Vert`, `Split`, `HorzA`, `HorzB`, `VertA`, `VertB`, `Horz4`, `Vert4`
  (`tile/partition.rs`)
- Inter modes in `PredictionMode` (`tile/coding_unit.rs`): single-reference `NewMv`, `NearestMv`, `NearMv`,
  `GlobalMv`; compound `NearestNearestMv`, `NearNearMv`, `NearestNewMv`, `NewNearestMv`, `NearNewMv`,
  `NewNearMv`, `GlobalGlobalMv`, `NewNewMv`. AV1 has no `ZEROMV`; that is the VP9 name, and AV1 uses `GlobalMv`.

> Intra-mode fact check: AV1 has 13 luma intra modes: DC, 8 directional, SMOOTH, SMOOTH_V, SMOOTH_H, PAETH.
> The 8 directional modes each take an angle delta of −3..+3, giving 56 directional angles. Chroma adds CFL.
> The old docs said "56 angular + 4 planar". That count is wrong.

## H.264: `bitvue_avc` (`nal.rs`, `sps.rs`, `pps.rs`, `slice.rs`, `sei.rs`)

- `parse_nal_units(annex_b) -> Vec<NalUnit>`. `NalUnit` has `header`, `offset: usize`, `size: usize`,
  `payload` (emulation-prevention bytes removed), and `raw_payload`.
- `NalUnitHeader` has `forbidden_zero_bit`, `nal_ref_idc`, and `nal_unit_type`.
- `NalUnitType` values:

  | Value | `NalUnitType` |
  |---|---|
  | 1 | `NonIdrSlice` |
  | 2–4 | `SliceDataA`/`SliceDataB`/`SliceDataC` |
  | 5 | `IdrSlice` |
  | 6 | `Sei` |
  | 7 | `Sps` |
  | 8 | `Pps` |
  | 9 | `Aud` |
  | 10 | `EndOfSequence` |
  | 11 | `EndOfStream` |
  | 12 | `FillerData` |
  | 13 | `SpsExtension` |
  | 14 | `PrefixNal` |
  | 15 | `SubsetSps` |
  | 16 | `Dps` |
  | 19 | `AuxSlice` |
  | 20 | `SliceExtension` |
  | 21 | `SliceExtensionDepth` |

  Helpers: `is_vcl()`, `is_parameter_set()`.
- Other parsers: `parse_sps`, `parse_pps`, `parse_slice_header`, `parse_sei`.
- Fields worth checking first:
  - `Sps`: `profile_idc: ProfileIdc` (66 Baseline, 77 Main, 88 Extended, 100 High, 110 High10, 122 High422,
    244 High444, 44 CAVLC444), `level_idc`, `chroma_format_idc`, `max_num_ref_frames`, `frame_mbs_only_flag`
  - `Pps`: `entropy_coding_mode_flag` (CABAC), `transform_8x8_mode_flag`, `num_slice_groups_minus1` (> 0 means
    FMO, which only Baseline/Extended allow)
  - `SliceHeader`: `first_mb_in_slice`, `slice_type`, `pic_parameter_set_id`, `frame_num`, `slice_qp_delta`
- Whole-stream parse: `parse_avc(data) -> AvcStream` (`sps_map`, `pps_map`, `slices` with POC, `sei_messages`)
  and `parse_avc_quick`.
- Frames: `extract_annex_b_frames`, `extract_frame_at_index`.

## HEVC: `bitvue_hevc`

- `NalUnitHeader` has `nal_unit_type`, `nuh_layer_id`, `nuh_temporal_id_plus1`, and `temporal_id()`.
  `NalUnit.offset` and `NalUnit.size` are `u64`.
- `NalUnitType` uses the spec numbering:

  | Value | `NalUnitType` |
  |---|---|
  | 0–9 | `TrailN`, `TrailR`, `TsaN`, `TsaR`, `StsaN`, `StsaR`, `RadlN`, `RadlR`, `RaslN`, `RaslR` |
  | 16–18 | `BlaWLp`, `BlaWRadl`, `BlaNLp` |
  | 19–20 | `IdrWRadl`, `IdrNLp` |
  | 21 | `CraNut` |
  | 32 | `VpsNut` |
  | 33 | `SpsNut` |
  | 34 | `PpsNut` |
  | 35 | `AudNut` |
  | 36 | `EosNut` |
  | 37 | `EobNut` |
  | 38 | `FdNut` |
  | 39 | `PrefixSeiNut` |
  | 40 | `SuffixSeiNut` |

  Helpers: `is_vcl`, `is_irap`, `is_idr`, `is_bla`, `is_cra`, `is_rasl`, `is_radl`, `is_reference`.
- `SliceHeader` key fields: `slice_pic_parameter_set_id`, `dependent_slice_segment_flag`, `slice_type`,
  `slice_pic_order_cnt_lsb`, `slice_sao_luma_flag`, `slice_qp_delta`.
- Whole-stream parse: `parse_hevc(data) -> HevcStream` (`vps_map`, `sps_map`, `pps_map`, `slices`), plus
  `irap_frames()`, `idr_frames()`, and `parse_hevc_quick`.

## VVC: `bitvue_vvc`

- The header shape matches HEVC.
- `NalUnitType` values:

  | Value | `NalUnitType` |
  |---|---|
  | 0 | `TrailNut` |
  | 1 | `StapNut` (the spec calls this STSA_NUT; the variant name is a typo) |
  | 2 | `RadlNut` |
  | 3 | `RaslNut` |
  | 7–8 | `IdrWRadl`, `IdrNLp` |
  | 9 | `CraNut` |
  | 10 | `GdrNut` |
  | 13 | `OpiNut` |
  | 14 | `DciNut` |
  | 15 | `VpsNut` |
  | 16 | `SpsNut` |
  | 17 | `PpsNut` |
  | 18–19 | `PrefixApsNut`, `SuffixApsNut` |
  | 20 | `PhNut` |
  | 21 | `AudNut` |
  | 24–25 | `PrefixSeiNut`, `SuffixSeiNut` |

- Other parsers: `parse_sps` (returns `AlfConfig`, `LmcsConfig`, `DualTreeConfig`), `parse_pps`, `parse_vvc`,
  `parse_vvc_quick`.

## VP9: `bitvue_vp9`

- Superframes (`superframe.rs`): `has_superframe_index`, `parse_superframe_index -> SuperframeIndex`
  (`frame_count`, `frame_sizes`, `frame_offsets`), and `extract_frames(data) -> Vec<&[u8]>`. The index sits at
  the **end** of the chunk.
- `frame_header::parse_frame_header(data) -> FrameHeader`. Fields include `frame_type`, `show_frame`,
  `intra_only`, `width`/`height`, `render_width`/`render_height`, `refresh_frame_flags`,
  `ref_frame_idx: [u8; 3]`, and `interpolation_filter`.
- Whole-stream parse: `parse_vp9`, `parse_vp9_quick`. Frame listing uses `Vp9FrameType::{Key, Inter, ...}`.

## MPEG-2: `bitvue_mpeg2_codec`

- `find_start_codes` returns `StartCodeType` values:

  | Code | `StartCodeType` |
  |---|---|
  | 0x00 | `Picture` |
  | 0x01–0xAF | `Slice(n)` |
  | 0xB2 | `UserData` |
  | 0xB3 | `SequenceHeader` |
  | 0xB5 | `Extension` |
  | 0xB7 | `SequenceEnd` |
  | 0xB8 | `GroupOfPictures` |

- Whole-stream parse: `parse_mpeg2` and `parse_mpeg2_quick`. No CLI or sidecar code calls them yet.

## AVS3 (IEEE 1857.10, not "AV3"): `bitvue_avs3`

- `scan_nal_units(data) -> Vec<NalUnit>`, where each unit carries an `Sci` code:

  | Code | `Sci` |
  |---|---|
  | 0x00–0xAF | `Slice(n)` |
  | 0xB0 | `SequenceHeader` |
  | 0xB1 | `SequenceEnd` |
  | 0xB2 | `UserData` |
  | 0xB3 | `IFrame` |
  | 0xB5 | `Extension` |
  | 0xB6 | `PBFrame` |

- Other parsers: `parse_sequence_header`, `parse_i_picture_header`, `parse_pb_picture_header`,
  `extract_avs3_frames(data, limit)`.

## JPEG XS (ISO 21122) and VC-3/DNxHD

- JPEG XS (`bitvue_jpegxs`):
  - `scan_markers` finds the marker segments in `marker::markers`: `SOC` 0xFF10, `EOC` 0xFF11, `CAP`, `PIH`
    0xFF51, `CDT`, `WGT`, `SLH`, `NLT`, `CWD`, and others.
  - Other functions: `parse_pih`, `extract_jpegxs_frames`.
- VC-3/DNxHD (`bitvue_vc3`): `scan_segments`, `parse_frame_header(data, offset)` (keyed on `DNXHD_MAGIC`,
  returns `CompId`), and `extract_vc3_frames`.
