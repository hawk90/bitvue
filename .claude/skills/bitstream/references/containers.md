# Containers

Code: `crates/bitvue-formats/src/` (MP4, MKV, TS, detection) and `crates/bitvue-av1-codec/src/ivf.rs` (IVF).
`bitvue-formats` does not parse IVF. For box and EBML hierarchy background, see the `video-formats-expert` agent.

## Detection: `bitvue_formats::container`

`detect_container_format(path) -> io::Result<ContainerFormat>` runs `detect_from_magic_bytes` on the first
32 bytes. If that returns `Unknown`, it falls back to `detect_from_extension`.

| Variant | Magic | Extensions |
|---|---|---|
| `MP4` | `ftyp` at offset 4 | mp4 m4v m4a mov |
| `Matroska` | EBML `1A 45 DF A3` | mkv webm |
| `AVI` | `RIFF` + `AVI ` at offset 8 | avi |
| `IVF` | `DKIF` | ivf |
| `AnnexB` | 3- or 4-byte start code followed by a plausible H.264 NAL type or HEVC parameter-set type | h264 h265 hevc 265 |
| `Unknown` | anything else, including MPEG-TS | |

- Annex B detection does not tell H.264 from HEVC. CLI `decode` assumes HEVC, so pass `--avc` for H.264.
- For TS, use `bitvue_formats::ts::is_ts(&data)`: it checks the 0x47 sync byte at 188-byte packet spacing.
- `bitvue_av1_codec` also has byte-level checks: `is_ivf`, `is_av1_ivf`, `is_mp4`, `is_mkv`, `is_webm`,
  `is_mov`, `is_ts`. `parse_av1(data)` sniffs the container and returns `Av1Info`.

## IVF: `bitvue_av1_codec::ivf`

The file header is 32 bytes, little-endian:

| Bytes | Field (`IvfHeader`) |
|---|---|
| 0..4 | `signature` = `DKIF` |
| 4..6 | `version` (0) |
| 6..8 | `header_size` (32) |
| 8..12 | `fourcc` (`AV01`, `VP90`, ...) |
| 12..14 / 14..16 | `width` / `height` |
| 16..20 | `framerate_den` (timebase denominator, i.e. the frame rate for a 1/fps timebase) |
| 20..24 | `framerate_num` |
| 24..28 | `frame_count` |

Each frame has a 12-byte header (`size: u32`, `timestamp: u64`) followed by the payload. The resulting
`IvfFrame` has `size`, `timestamp`, `data: Vec<u8>`, and `temporal_id`.

API:
- `parse_ivf_header(&[u8]) -> Result<IvfHeader, BitvueError>`
- `parse_ivf_frames(&[u8]) -> Result<(IvfHeader, Vec<IvfFrame>), BitvueError>`
- `extract_obu_data(&[u8])`: concatenates the frame payloads
- `is_ivf`, `is_av1_ivf`

Writing IVF: `bitvue_formats::IvfWriter`.

IVF is the only container that the desktop app (`bitvue-indexer`), the sidecar, and the MCP server accept.
CLI `decode` reads the IVF FourCC: `AV01` maps to AV1, `VP90` or `VP9 ` maps to VP9, and anything else
defaults to AV1.

## MP4 / MOV: `bitvue_formats::mp4`

- `parse_mp4(&[u8]) -> Result<Mp4Info, BitvueError>`. `Mp4Info` has `brand`, `compatible_brands`, `codec`
  (sample-entry FourCC), `timescale`, `sample_count`, and more.
- `extract_av1_samples` accepts `av01`. `extract_avc_samples` accepts `avc1`/`avc3`. `extract_hevc_samples`
  accepts `hvc1`/`hev1`. All three return `Vec<Cow<[u8]>>`, which borrows from the input when possible.
- Samples are the raw sample bytes, so AVC/HEVC come out as length-prefixed NAL units (avcC/hvcC framing), not Annex B. Convert them
  before you feed them to the Annex B `parse_nal_units`.
- `hvc1` means parameter sets live only in `hvcC`. `hev1` means they may also appear in-band. (The doc
  comment in `mp4.rs` has these two swapped.)
- Allocation is capped through `ResourceBudget` (`resource_budget.rs`).
- Wrapper: `bitvue_av1_codec::extract_obu_data_from_mp4`.

## Matroska / WebM: `bitvue_formats::mkv`

- `parse_mkv(&[u8]) -> Result<MkvInfo, BitvueError>`. `MkvInfo` has `codec_id`, `video_track_number`,
  `sample_count`, `samples`, and `timestamps`.
- Codec IDs:
  - `extract_av1_samples` takes `V_AV1`.
  - `extract_avc_samples` takes `V_MPEG4/ISO/AVC`.
  - `extract_hevc_samples` takes `V_MPEGH/ISO/HEVC`.
- Wrapper: `bitvue_av1_codec::extract_obu_data_from_mkv`.

## MPEG-TS: `bitvue_formats::ts`

- `parse_ts(&[u8]) -> Result<TsInfo>`. `TsInfo` has `video_pid`, `sample_count`, `samples`, and
  `timestamps`. It walks PAT, then PMT, then reassembles PES.
- AV1: `parse_ts`/`extract_av1_samples` pick the first PMT stream with `stream_type` 0x06 (private data). They
  do **not** check the AV1 registration descriptor, so any 0x06 stream matches.
- H.264 uses `stream_type` 0x1B (`extract_avc_samples`, `extract_avc_video_samples`). HEVC uses 0x24
  (`extract_hevc_samples`, `extract_hevc_video_samples`). These split PES on Annex B start codes and return
  NAL payloads without the start codes. The `*_video_samples` variants return `VideoSample` with timing.
- Wrapper: `bitvue_av1_codec::extract_obu_data_from_ts`. No CLI command or sidecar command reads TS today.

## Tests

- `crates/bitvue-formats/tests/{container_formats_test.rs, container_edge_cases_test.rs, mp4_av1_integration_test.rs}`
- Inline tests at the bottom of `mp4.rs` and `container.rs`
