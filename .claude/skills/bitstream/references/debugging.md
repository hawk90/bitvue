# Debugging bitstream parsing

## Triage order

1. `bitvue validate -f FILE` (AV1 in IVF/MP4). It runs `parse_all_obus_resilient` on every frame and reports
   per-frame failures, for example `ERROR: Frame 0: 11 OBU(s) failed to parse`. Add `-s` to get a non-zero exit
   on failure. For Annex B it only checks that the file is readable and prints a warning.
2. `bitvue decode FILE [--codec flag] --frames N`. It shows how far frame extraction gets for any supported
   codec. `--regress` exits 1 on any parse error. `--errors LOG` writes the parse-error **count** only, not
   per-error details.
3. Find the byte offset (from the `frames`/`decode` "Offset" column or `Diagnostic.offset_bytes`) and inspect
   the raw bytes:
   - App/sidecar: send `get_hex_range` with `{"stream": "A", "offset": <u64>, "len": <usize>}`. The reply is a
     `Control` metadata frame (`offset`, actual `len`) followed by a `Data` frame of raw bytes that shares the
     same correlation id. Client wrapper: `SidecarClient.getHexRange` in
     `bitvue-desktop/src/sidecarClient.ts`. Implementation: `crates/bitvue-sidecar/src/commands/data_plane.rs`.
   - Shell: `xxd -s <offset> -l 64 FILE`.
4. Reproduce in a test against the smallest slice of bytes that fails (see Tests below).

## Error types

`bitvue_engine::BitvueError` (`crates/bitvue-engine/src/error.rs`):

| Variant | Typical cause |
|---|---|
| `Io(std::io::Error)`, `IoError { .. }` | file access |
| `Parse { offset, message }` | malformed syntax at a byte offset |
| `InvalidObuType(u8)` | AV1 reserved or invalid OBU type |
| `UnexpectedEof(u64)` | truncated input |
| `UnsupportedCodec(String)` | codec or container path not implemented |
| `Decode(String)` | decoder failure; also used for oversized-grid guards in the overlays |
| `InsufficientData { needed, available }` | buffer shorter than a fixed header (IVF needs 32 bytes) |
| `InvalidData(String)` | semantically invalid values |

Each codec crate also has its own error enum: `AvcError`, `HevcError`, `VvcError`, `Vp9Error`, `Mpeg2Error`,
`Avs3Error`, `JpegXsError`, `Vc3Error` (each in `src/error.rs`). Decoders use
`bitvue_decode::decoder::DecodeError`: `Init`, `Decode`, `NoFrame`, `UnsupportedFormat`. `H264Decoder::new()`
and the other FFmpeg decoders only exist with `--features ffmpeg`.

## Resilient parsing (AV1)

`bitvue_av1_codec::parse_all_obus_resilient(data, StreamId) -> (Vec<Obu>, Vec<Diagnostic>)`

- On each error it pushes a `bitvue_engine::event::Diagnostic` and **advances one byte** to resync.
- `Diagnostic` fields: `id`, `severity`, `stream_id`, `message`, `category`, `offset_bytes`, `frame_index`
  (approximate), `count`, `impact_score`.
- `Severity` values: `Info`, `Warn`, `Error`, `Fatal`. `Category` values include `Container`, `Bitstream`,
  `Decode`, `IO`.
- Mapping: `Parse` → Error (impact 85), `InvalidObuType` → Error (90), `UnexpectedEof` → Fatal (100).
- It stops with a Fatal "Too many parse errors" once there are ≥ 10 diagnostics **and** more diagnostics than
  OBUs parsed. Because of that limit, a single corrupt region usually yields 10 errors plus 1 fatal.
- The strict path is `parse_all_obus` / `ObuIterator`, which stops at the first error.

Other resilience points:
- `ParsedFrame` falls back to scaffold grids unless you call `overlay_extraction::set_strict_mode(true)`.
- `parse_vp9` logs and skips frames it cannot parse (`abseil::vlog!`).
- `parse_avc`/`parse_hevc` silently drop SPS/PPS/slices that fail to parse. If counts look low, check this
  first.
- Allocation guards: `MAX_OBU_PAYLOAD_SIZE` (100 MB) in `obu.rs`, `ResourceBudget` in `bitvue-formats`, and
  `MAX_GRID_DIMENSION`/`MAX_GRID_BLOCKS` in the overlay extractors.

## Common pitfalls

| Symptom | Likely cause |
|---|---|
| Wrong overlay or 0 coding units on frames > 0 | Used `ParsedFrame::parse` instead of `parse_with_ref_state`; see `overlay-api.md` |
| H.264 Annex B parsed as HEVC garbage | CLI auto-detect maps every Annex B file to HEVC; pass `--avc` |
| `info` on VP9 IVF shows `Other=N` frame types | `info` runs AV1 OBU parsing on every IVF; use `decode` for VP9 |
| Syntax bit ranges shifted | `parse_obu_syntax`'s `global_offset` is in **bits**, not bytes |
| EPB confusion in NAL parsing | `NalUnit.payload` has emulation-prevention bytes removed; `raw_payload` keeps them (`remove_emulation_prevention_bytes`) |
| `bitvue analyze` panics | clap duplicate `-f` short flag (debug builds) |

## Symptom checklists

**Truncated input (`UnexpectedEof`, `InsufficientData`, too few frames)**
- `parse_ivf_frames` stops **silently** at a truncated last frame and ignores the header's `frame_count`.
  Compare `frames.len()` with `IvfHeader.frame_count`; if fewer frames come back, the file is truncated (for
  example, an incomplete download or a capture that is still being written).
- MP4: check whether `stsz`/`stco`/`co64` point past EOF. The sample tables can be intact while `mdat` is cut.

**Invalid AV1 OBU type / forbidden bit set**
- `obu_type = (byte >> 3) & 0x0F`, and bit 7 is `obu_forbidden_bit`. Dump the byte and decode it by hand.
- The usual cause is a misaligned offset, not a corrupt stream. Look for a wrong leb128 size, a missed extension
  byte (`extension_flag`), an IVF 12-byte frame header that was not skipped, or input in Annex B
  (length-delimited) format instead of the low-overhead OBU format.

**Tile or frame data fails to parse**
- Key frame or inter frame? Inter frames depend on reference state (see `ParsedFrame::parse_with_ref_state` in
  `overlay-api.md`).
- Are the referenced slots filled? Are QP and MV values in range? Do frame dimensions match the sequence header?

## Manual conformance checklist

`validate` only checks that OBUs parse. When you suspect a semantic problem, also check:
- OBU headers: valid type, forbidden bit 0, size fields consistent with the bytes that are actually present
- Frame size ≤ the sequence header's `max_frame_width`/`max_frame_height`, and non-zero
- A dimension change between frames is legal only with a new sequence header or a frame size override. Flag it.
- `ref_frame_idx` entries (3-bit slot numbers) and `show_existing_frame` must point to slots that an earlier
  frame filled through `refresh_frame_flags`. They are slot indices, not frame numbers.
- `base_q_idx` is in 0–255, the loop filter / CDEF parameters are in range, and the tile count matches the tile
  info
- AVC/HEVC: an SPS/PPS (and VPS for HEVC) arrives before the first slice that references it. MP4 NAL
  length prefixes add up to the sample size.

Log with context (byte offset, frame index). Use `tracing::debug!` for per-unit detail and `warn!`/`error!` for
recovered or fatal failures, as the overlay extractors do.

## Fuzzing (`fuzz/`)

- Targets in `fuzz/fuzz_targets/`: `obu_parser.rs`, `ivf_parser.rs`, `leb128.rs`, `av1_decode.rs`,
  `fuzz_target_1.rs`.
- **`fuzz/Cargo.toml` is stale.** It depends on `path = "../crates/bitvue-av1"`, which does not exist; it
  should be `bitvue-av1-codec`. It also registers only `obu_parser`, `ivf_parser`, and `leb128`. Fix the
  manifest before running `cd fuzz && cargo fuzz run obu_parser` (needs `cargo install cargo-fuzz`, nightly).

## Tests to extend

| Area | Tests |
|---|---|
| Resilient parser | `crates/bitvue-av1-codec/tests/resilient_parser_test.rs` (invalid OBU type, EOF, forbidden bit, multiple errors, error limit, offset tracking) |
| AV1 parsing | `crates/bitvue-av1-codec/tests/{obu_tests.rs, ivf_prop_tests.rs, leb128_prop_tests.rs, bitreader_prop_tests.rs}` (proptest) |
| Containers | `crates/bitvue-formats/tests/container_edge_cases_test.rs` |
| Engine | `crates/bitvue-engine/tests/critical_edge_cases_test.rs` |
| Sidecar | `crates/bitvue-sidecar/tests/subprocess_smoke.rs` (end to end over stdio) |

Run them with `cargo test --workspace --exclude abseil --lib --tests`. Never run `--lib` alone, because that
skips the `tests/` binaries.
