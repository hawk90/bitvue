# Fuzzing

This directory contains fuzz targets for testing Bitvue's parsers against malformed input.

## Prerequisites

Install cargo-fuzz:
```bash
cargo install cargo-fuzz
```

## Running Fuzzers

Run all fuzz targets:
```bash
cd fuzz
cargo fuzz run
```

Run specific fuzz target:
```bash
cd fuzz
cargo fuzz run obu_parser
cargo fuzz run ivf_parser
cargo fuzz run leb128
```

## Fuzz Targets

### obu_parser
Fuzzes the AV1 OBU parser with arbitrary byte input.
- **Target**: `bitvue_av1::parse_obu`
- **Goal**: Find panics or crashes in OBU parsing

### ivf_parser
Fuzzes the IVF parser with arbitrary byte input.
- **Targets**: `bitvue_av1::parse_ivf_header`, `parse_ivf_frames`
- **Goal**: Find panics or crashes in IVF parsing

### leb128
Fuzzes the LEB128 encoder/decoder roundtrip.
- **Target**: `bitvue_av1::{decode_uleb128, encode_uleb128}`
- **Goal**: Find roundtrip errors or crashes

## Continuous Integration

To run fuzzing in CI (limited time):

```bash
# Run for 60 seconds per target
cargo fuzz run obu_parser -- -max_total_time=60
```

## Corpus

The `corpus/` directory contains seed inputs for the fuzzer. Add real-world video file samples here to improve fuzzing effectiveness:

```bash
# Add a sample AV1 file to the corpus
cp test_data/video.av1 fuzz/corpus/obu_parser/
```

## Viewing Coverage

To see which code paths are covered by fuzzing:

```bash
cargo fuzz coverage obu_parser
```

## Resources

- [libFuzzer documentation](https://llvm.org/docs/LibFuzzer/)
- [cargo-fuzz documentation](https://github.com/rust-fuzz/cargo-fuzz)
- [Fuzzing Book](https://doc.rust-lang.org/beta/book/unstable-book/ch06-00-testing.html)
