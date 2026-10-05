---
name: bitstream
description: Inspect, parse, analyze, debug, and decode video bitstreams and containers in bitvue (AV1, H.264, HEVC, VP9, VVC, MPEG-2, AVS3, JPEG XS, VC-3 in IVF/MP4/MKV/TS/Annex B), and extract QP/MV/partition overlays. Use when looking at a stream file, chasing a parse or decode error, adding container/codec support, or working on overlay extraction code.
allowed-tools: Read, Grep, Glob, Bash
---

# Bitstream inspection and parsing

Use the real `bitvue` CLI (crate `bitvue-cli`, binary `bitvue`) and the codec crates to look at a stream. Most
CLI subcommands are **AV1-only**. Check the support matrix before you trust any output for another codec.

Codec spec background (profiles, tools, intra modes) lives in the `video-codec-expert` agent. Container layout
background (box/EBML hierarchies) lives in `video-formats-expert`. This skill covers what the **code** does.

## Workflow

1. **Identify the container.** `bitvue_formats::detect_container_format(&Path)` checks magic bytes first, then
   the file extension. It returns `ContainerFormat::{MP4, Matroska, AVI, IVF, AnnexB, Unknown}`. MPEG-TS has no
   variant, so a `.ts` file comes back `Unknown`. See `references/containers.md`.
2. **Get a stream summary** (AV1 in IVF or MP4):
   ```bash
   cargo run -q -p bitvue-cli -- info -f test_data/av1_test.ivf
   cargo run -q -p bitvue-cli -- frames -f test_data/av1_test.ivf -n 20 -F json   # text|json|csv
   cargo run -q -p bitvue-cli -- validate -f test_data/av1_test.ivf              # -s: non-zero exit on failure
   ```
   `info` prints format, codec FourCC, size, frame counts, profile, bit depth, an I/P/Other split, and an
   estimated bitrate. `frames` prints index, type, size, PTS, file offset, and key-frame flag per frame.
3. **For any codec, list frames** with `decode`. It is the only subcommand that dispatches per codec:
   ```bash
   cargo run -q -p bitvue-cli -- decode test_data/hevc_test.hevc --stats --frames 10
   cargo run -q -p bitvue-cli -- decode test_data/avc_test.h264 --avc --stats   # Annex B auto-detects as HEVC
   cargo run -q -p bitvue-cli -- decode test_data/vp9_test.ivf --stats          # IVF FourCC VP90 → VP9
   cargo run -q -p bitvue-cli -- decode in.ivf --md5 --regress                  # silent, exit 1 on any parse error
   ```
   Force flags: `--av1 --hevc --avc --vp9 --vvc --mpeg2 --avs3`. Other flags: `--frames N`, `--stats`,
   `--stream-stats`, `--md5`, `-o FILE`, `--y4m`, `--dump`, `--dump-bitdepth`, `--display-order`, `--no-crop`,
   `--fast 0|1|2`, `--film-grain`, `--errors LOG`, `--psnr --reference REF`, `--cpu-max-feature`.
   Only AV1 produces YUV output (through dav1d). See `references/decode.md`.
4. **Look inside a frame.** For syntax trees, use `bitvue_av1_codec::parse_obu_syntax` /
   `parse_bitstream_syntax`, or the sidecar commands `get_frame_syntax` and `get_frame_analysis` (the desktop
   app path). See `references/syntax.md`.
5. **Overlays (QP/MV/partition).** See `references/overlay-api.md` for the `extract_*` functions per codec.
   Only the AV1 ones are wired into the sidecar.
6. **Something broken?** Use `validate` (it calls `parse_all_obus_resilient`), `decode --errors`, the sidecar
   `get_hex_range`, and the fuzz targets. See `references/debugging.md`.

Other subcommands: `analyze` (per-frame `--syntax/--residual/--coding-flow`, AV1 IVF/MP4), `export -f -o
--format json|csv|markdown` (AV1; IVF/MP4/MKV), `batch -d -p -o`, `quality --reference --distorted -f -m
psnr,ssim,vmaf`, `bd-rate`, `evidence-diff A B [-s]`. Run `bitvue <cmd> --help` for the current flags.

### Stream-level analysis and comparing encodes

What to read off a stream, and where to get it:
- **Efficiency:** bitrate and frame-type split (`info`), average and per-frame size (`frames -F csv`), I-frame
  size relative to the average, and GOP length / key-frame interval (the key-frame column of `frames`, or MCP
  `get_gop_structure`). Compression ratio vs raw = `w × h × 1.5 × bytes_per_sample × frames / file_size` for
  4:2:0.
- **Parameters:** profile, level, bit depth, and chroma from the sequence header / SPS (see
  `references/syntax.md`). QP spread comes from the QP overlay (`references/overlay-api.md`) or MCP `get_qp_map`.

Comparing two encodes of the same source (AV1 IVF/MP4 unless noted):
```bash
cargo run -q -p bitvue-cli -- info -f a.ivf; cargo run -q -p bitvue-cli -- info -f b.ivf     # bitrate, frame types
cargo run -q -p bitvue-cli -- quality --reference src.ivf --distorted a.ivf -m psnr,ssim       # vmaf only if built in
cargo run -q -p bitvue-cli -- bd-rate --reference src.ivf --anchor a_q28.ivf a_q36.ivf a_q44.ivf a_q52.ivf \
  --test b_q28.ivf b_q36.ivf b_q44.ivf b_q52.ivf --metric psnr                                 # ≥4 points per curve
cargo run -q -p bitvue-cli -- evidence-diff bundle_a/ bundle_b/ -s                           # exported evidence bundles
```
Compare bitrate and quality together. A lower bitrate alone does not mean a better encode, so use BD-rate for
the fair comparison.

> **Known bug:** `bitvue analyze` panics in debug builds. Clap reports a duplicate short flag (`-f` is
> declared for both `file` and `frame`) in `crates/bitvue-cli/src/main.rs`. Use a release build or fix the
> flag before relying on it.

### MCP server

`bitvue-mcp-server` (crate `bitvue-mcp`, `crates/bitvue-mcp/src/main.rs`) provides these tools: `load_file`,
`get_stream_info`, `analyze_frame`, `get_qp_map`, `get_motion_vectors`, `get_gop_structure`,
`find_decoding_issues`, `search_syntax`, `compare_streams`, `list_files`. It is not registered in a repo
`.mcp.json`; add it yourself if you want it.

Two limits:
- `load_file` accepts only `.ivf`/`.av1`, even though its description says MP4/MKV/TS.
- Paths must sit under the server's working directory.

## Support matrix

Status as of the 0.12.0 workspace. Re-check `crates/bitvue-cli/src/commands/decode.rs` and
`crates/bitvue-decode/src/lib.rs` before quoting it.

| Codec | Parser crate | CLI `decode` frame list | Pixel decoder (`bitvue-decode`) | Overlay fns | Desktop app (sidecar) |
|---|---|---|---|---|---|
| AV1 | `bitvue-av1-codec` | yes | `Av1Decoder` (dav1d, always on) | yes, real tile parse | **yes** (IVF only) |
| H.264 | `bitvue-avc` | yes (`--avc`) | `H264Decoder`, feature `ffmpeg` | yes | no |
| HEVC | `bitvue-hevc` | yes | `HevcDecoder`, feature `ffmpeg` | yes | no |
| VP9 | `bitvue-vp9` | yes | `Vp9Decoder`, feature `ffmpeg` | approximate (header-derived) | no |
| VVC | `bitvue-vvc` | "not yet implemented" | `VvcDecoder`, feature `vvdec` | yes | no |
| MPEG-2 | `bitvue-mpeg2-codec` | "not yet implemented" | none | none | no |
| AVS3 | `bitvue-avs3` | yes (`--avs3`) | none | picture-level proxy | no |
| JPEG XS | `bitvue-jpegxs` | code path exists, no CLI flag or auto-detect | none | precinct/dequant/transform maps | no |
| VC-3/DNxHD | `bitvue-vc3` | code path exists, no CLI flag or auto-detect | none | `extract_mb_grid` | no |

The CLI never decodes pixels for anything except AV1. `do_yuv_dump` prints a note and skips the YUV dump for
every other codec, whatever features you build with. The `ffmpeg` and `vvdec` features need system libraries,
so avoid `--all-features`.

| Container | Detect | Demux code | Used by |
|---|---|---|---|
| IVF | `DKIF` magic / `.ivf` | `bitvue_av1_codec::{parse_ivf_header, parse_ivf_frames}` | CLI, sidecar/indexer, MCP |
| MP4/MOV | `ftyp` @4 / ext | `bitvue_formats::mp4::{parse_mp4, extract_{av1,avc,hevc}_samples}` | CLI info/frames/analyze/validate/export |
| MKV/WebM | EBML magic / ext | `bitvue_formats::mkv::{parse_mkv, extract_{av1,avc,hevc}_samples}` | CLI export |
| MPEG-TS | none (`Unknown`); `bitvue_formats::ts::is_ts` | `bitvue_formats::ts::{parse_ts, extract_*_samples}` | library only |
| Annex B | start code + plausible NAL type / `.h264 .h265 .hevc .265` | `bitvue_{avc,hevc,vvc}::parse_nal_units` | CLI decode |
| AVI | `RIFF`…`AVI ` | detection only | none |

## Where things live

- CLI: `crates/bitvue-cli/src/main.rs` (clap args) and `src/commands/{info,frames,analyze,validate,export,batch,quality,decode,bd_rate,evidence_diff}.rs`
- Container detection: `crates/bitvue-formats/src/container.rs`. Demuxers: `mp4.rs`, `mkv.rs`, `ts.rs`. Allocation cap: `resource_budget.rs`.
- AV1: `crates/bitvue-av1-codec/src/{ivf,obu,sequence,frame_header,frame_header_full,syntax_parser/,tile/,overlay_extraction/}`
- NAL codecs: `crates/bitvue-{avc,hevc,vvc}/src/{nal,sps,pps,slice,overlay_extraction}.rs`. VP9: `crates/bitvue-vp9/src/{superframe,frame_header,overlay_extraction}.rs`
- Grid types: `crates/bitvue-engine/src/{qp_heatmap.rs (QPGrid), mv_overlay.rs (MVGrid), partition_grid.rs (PartitionGrid)}`. Errors: `error.rs` (`BitvueError`). Diagnostics: `event.rs`.
- Desktop data path: `crates/bitvue-indexer/src/lib.rs` (IVF+AV1 indexing) and `crates/bitvue-sidecar/src/request_dispatch.rs` (command table), `frame_analysis.rs`, `decode_bridge.rs`, `commands/data_plane.rs` (`get_hex_range`).
- Pixel decode: `crates/bitvue-decode/src/{decoder.rs (Av1Decoder), ffmpeg.rs, vvdec.rs, yuv.rs, yuv_loader.rs, strategy/}`
- Fixtures: `test_data/{av1_test.ivf, avc_test.h264, hevc_test.hevc, vp9_test.ivf}`. App samples: `samples/foreman_*` (also `.mp4`, `.mkv`, `.webm`).

## References

- `references/containers.md`: detection rules, IVF layout, MP4/MKV/TS sample extraction and codec IDs
- `references/syntax.md`: OBU types, AV1 header fields, NAL type enums for AVC/HEVC/VVC, VP9/MPEG-2/AVS3 units
- `references/overlay-api.md`: `extract_*` signatures per codec, `ParsedFrame` and ref-state rules, grid types
- `references/decode.md`: `Av1Decoder` API, feature-gated decoders, YUV conversion, thumbnails
- `references/debugging.md`: resilient parsing, error variants, hex inspection, fuzzing, relevant tests
