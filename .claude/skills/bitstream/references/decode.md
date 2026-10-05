# Decoding (pixels)

Crate: `bitvue-decode` (`crates/bitvue-decode/src/`). Parsing (headers, syntax, overlays) needs no decoder.
Decoding is only for YUV/RGB output, thumbnails, and metrics.

## Decoders

| Type | Codec | Backend | Available |
|---|---|---|---|
| `Av1Decoder` (`decoder.rs`) | AV1 | dav1d (`dav1d` crate) | always |
| `H264Decoder`, `HevcDecoder`, `Vp9Decoder`, `FfmpegDecoder` (`ffmpeg.rs`) | H.264, HEVC, VP9 | `ffmpeg-next` | `--features ffmpeg` (needs system FFmpeg) |
| `VvcDecoder` (`vvdec.rs`) | VVC | libvvdec | `--features vvdec` (macOS: `brew install vvdec`) |
| none | MPEG-2, AVS3, JPEG XS, VC-3 | none | parse-only |

All of them implement `traits::Decoder`: `codec_type`, `capabilities`, `send_data(data, Option<i64>)`,
`get_frame`, `decode_all`. `CodecType` has the variants `AV1`, `H264`, `H265`, `H266`, `VP9`.
`DecoderFactory::create(CodecType)` builds a decoder from the registry in `traits.rs`, and
`DecoderFactory::available_codecs()` lists what that build can create.

Build with a feature: `cargo build -p bitvue-decode --features ffmpeg`. Do not use `--all-features`
workspace-wide, because it enables both ffmpeg and vvdec.

## `Av1Decoder` API

- `Av1Decoder::new()`. `new_with_apply_grain(false)` gives pre-film-grain output; CLI `--film-grain` uses it.
- `send_data(&[u8], ts)` / `send_data_owned`, `get_frame()`, `decode_all(data)` (IVF or raw OBUs),
  `decode_from_file(path)`.
- **End of stream:** call `drain_decoder_frames(&mut out)`. Do **not** call `flush()` first. `flush()` wraps
  `dav1d_flush` (meant for seeking) and drops the frames still buffered in the multi-threaded pipeline.
- `DecodedFrame` fields: `width`, `height`, `bit_depth`, `y_plane`/`u_plane`/`v_plane` (`Arc<[u8]>`;
  `u_plane`/`v_plane` are `Option`), the `*_stride` fields, `timestamp`, `frame_type`, `qp_avg`.
  `validate_frame(&DecodedFrame)` sanity-checks a frame.
- `decoder::detect_format(&[u8]) -> VideoFormat` (`Ivf`, `Mp4`, `Mkv`, ...) sniffs the input; it is separate
  from `bitvue_formats::detect_container_format`.

## YUV / RGB

- `yuv::yuv_to_rgb(&DecodedFrame) -> Vec<u8>`, `rgb_to_image(rgb, w, h)`.
- The SIMD strategy is picked at runtime: `current_strategy()`, `available_strategies()`, `set_strategy()`.
  `StrategyType` in `strategy/registry.rs` has `Scalar`, `Avx2`, `Neon`, and `Metal` (future). The code is
  in `strategy/{scalar,avx2,neon,metal}.rs`.
- Raw YUV files: `YuvLoader::open(path, Option<YuvFileParams>)`. `YuvFileParams` has `width`, `height`,
  `chroma_subsampling`, `bit_depth`, and `frame_rate`. The sidecar debug-YUV commands use it
  (`load_debug_yuv`, `get_debug_yuv_frame`, ...).

- **Colour matrix:** `yuv_to_rgb` (scalar and SIMD strategies) always uses **BT.601 full-range** coefficients
  in fixed point /128: R = Y + 1.402·V′, G = Y − 0.344·U′ − 0.714·V′, B = Y + 1.772·U′, where U′ and V′ are
  the samples minus 128. It ignores the stream's `color_config` (matrix coefficients, limited vs full range).
  BT.709 or limited-range content therefore renders slightly off. Check this before you report a colour bug.
  High-bit-depth samples are shifted down to 8-bit first.

## Thumbnails

- An inter frame cannot be decoded on its own. `get_thumbnails` (`decode_bridge.rs`) decodes **from the
  start of the stream** once per batch and captures the requested indices along the way. Batch the requests
  (`THUMBNAIL_BATCH_SIZE`, `frontend/constants/ui.ts`) instead of requesting one at a time.
- Scaling keeps the aspect ratio: `ThumbnailCache::generate_thumbnail(frame, target_width)` in
  `bitvue-engine/src/filmstrip.rs`. The default width is 120 px.

## Corrupt frames

On a decode error, log the frame index, keep going with the next frame where possible, and mark the frame as
failed in the UI (`CachedFrame.error`) instead of aborting the stream.

## Where decoding is used

| Caller | What |
|---|---|
| CLI `decode -o/--dump/--y4m/--md5/--psnr` (`crates/bitvue-cli/src/commands/decode.rs`) | AV1 only via `Av1Decoder`; any other codec prints "YUV dump is only available for AV1" |
| Sidecar `get_decoded_frame_yuv`, `get_thumbnails` (`crates/bitvue-sidecar/src/decode_bridge.rs`) | AV1 in IVF; thumbnails come back as base64 PNG `data:` URLs |
| `bitvue-metrics` / CLI `quality`, `bd-rate` | PSNR/SSIM (VMAF optional) on decoded frames |

## Tests

`crates/bitvue-decode/tests/`:
- `decoder_test.rs`, `decode_system_test.rs`, `edge_cases_test.rs`
- `traits_test.rs`, `yuv_test.rs`, `yuv_loader_test.rs`
- `ffmpeg_test.rs` and `vvdec_test.rs` (cover the feature-gated decoders)

Use more than one resolution (odd sizes, 4:2:0 chroma rounding, 10-bit) when you test conversion or scaling.
