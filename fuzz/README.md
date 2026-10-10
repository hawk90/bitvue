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
| `av1_decode` | `Av1Decoder` (dav1d) | decoder wrapper robustness |

## Corpus

`corpus/` and `artifacts/` are gitignored. Random bytes rarely get past the sequence header, so seed
`frame_analysis`, `obu_stream`, `syntax_parser` and `obu_parser` with the first frame payloads of
`test_data/av1_*.ivf` (IVF frame data without the 12-byte frame header), and `ivf_parser`/`av1_decode`
with whole files. Without seeds `frame_analysis` stays at shallow coverage.

## Crashes

A crash is written to `artifacts/<target>/`. Reproduce with
`cargo +nightly fuzz run <target> artifacts/<target>/<file>`, fix, and add a regression test next to the
fixed code (not in `fuzz/`).
