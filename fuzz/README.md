# Fuzzing

Fuzz targets for the AV1 parsing path. Requires nightly + cargo-fuzz (`cargo install cargo-fuzz`).
This crate is excluded from the main workspace (`exclude = ["fuzz"]`); run everything from `fuzz/`.

```bash
cd fuzz
cargo +nightly fuzz run frame_analysis -- -max_total_time=60
```

## Targets

| Target | Entry points | Catches |
|---|---|---|
| `leb128` | `decode_uleb128`/`encode_uleb128` | roundtrip errors |
| `ivf_parser` | `parse_ivf_header`, `parse_ivf_frames`, `is_ivf` | container parse panics |
| `obu_parser` | `parse_obu` | single-OBU header/size panics |
| `obu_stream` | `ObuIterator`, `parse_all_obus`, `parse_all_obus_resilient` | panics, non-terminating iteration |
| `syntax_parser` | `parse_bitstream_syntax`, `parse_obu_syntax` | syntax-tree builder panics |
| `frame_analysis` | `ParsedFrame::parse` + QP/MV/partition/prediction/transform extractors | sequence/frame header, symbol decoder, coding-unit parser panics (deepest path) |
| `stream_analysis` | `StreamAnalyzer` over a whole IVF: frame headers with reference-slot state, tile decode with saved CDFs / motion fields, checkpoint rewind | cross-frame state panics (slots, projected motion field, film grain, scaled references), multi-tile and superres paths |
| `protocol_frames` | `bitvue_protocol::read_frame`/`write_frame` over a byte stream, then the JSON shapes (`Request`, `Response`, params) of every accepted payload | allocation sized by the declared length, frame round-trip mismatches, JSON shape panics |
| `tile_group_split` | `tile::layout::split_tile_group` with a `TileLayout` (columns, rows, log2 bit counts, `TileSizeBytes`) chosen by the first 4 input bytes | panics in the tile-range / size-field arithmetic; also asserts consecutive tile numbers, in-frame tiles, ordered non-overlapping in-payload ranges |
| `av1_decode` | `Av1Decoder` (dav1d) | decoder wrapper robustness |

## CI

`fuzz/` is not a workspace member, so `cargo build/test --workspace` never compiles it. The `Fuzz targets
build` job (and `scripts/run_regression_suite.sh`) runs `scripts/check_fuzz_targets.sh`: every
`fuzz_targets/*.rs` must be registered as a `[[bin]]` in `Cargo.toml` and every target must compile. Adding
a target means adding its `[[bin]]` entry, or that check fails. This does not run the fuzzers.

## Scheduled run

`.github/workflows/fuzz.yml` fuzzes every target for 120 s each, Mondays 03:00 UTC
(or on demand: Actions -> Fuzz -> Run workflow, with a `seconds` input). Each run starts from the corpus
the previous one cached and from `scripts/fuzz_seed_corpus.py`, minimizes it afterwards (`cmin`) and
uploads `fuzz/artifacts/<target>` when it fails. It runs with `-s none` for the reasons under "Memory
limit". A new target must be added to the workflow's matrix; `scripts/check_fuzz_targets.sh` fails if it is not.

## Corpus

`corpus/` and `artifacts/` are gitignored. `scripts/fuzz_seed_corpus.py` regenerates the seeds described
below from `test_data/av1_*.ivf` (40 frames per clip by default; nothing is copied into the repo).
Random bytes rarely get past the sequence header, so seed
`frame_analysis`, `obu_stream`, `syntax_parser` and `obu_parser` with the first frame payloads of
`test_data/av1_*.ivf` (IVF frame data without the 12-byte frame header), and `ivf_parser`/`av1_decode`/`stream_analysis`
with whole files. Without seeds `frame_analysis` stays at shallow coverage.

## Memory limit

cargo-fuzz builds with AddressSanitizer, whose quarantine of freed memory and shadow memory make the
process 5-10x larger than it is without it (a 16 KB `stream_analysis` input that needs 25 MB in a debug
test run showed 356 MB). With `-rss_limit_mb=1024` that produced three "out-of-memory" artifacts which were
not bugs. Give ASan runs `-rss_limit_mb=3072` or more, or run `cargo +nightly fuzz run -s none <target>`:
without the sanitizer it is much faster and still catches panics, index and overflow errors (debug
assertions stay on), only memory errors in `unsafe` code are missed.

## Crashes

A crash is written to `artifacts/<target>/`. Reproduce with
`cargo +nightly fuzz run <target> artifacts/<target>/<file>`, fix, and add a regression test next to the
fixed code (not in `fuzz/`).
