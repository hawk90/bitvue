#!/usr/bin/env python3
"""Seed fuzz/corpus/<target>/ from the AV1 clips in test_data/.

Random bytes rarely get past the sequence header, so frame_analysis, obu_stream, syntax_parser
and obu_parser start from real IVF frame payloads (no 12-byte frame header); ivf_parser,
av1_decode and stream_analysis start from whole files. The corpus is gitignored: this regenerates it, nothing from
test_data/ is copied into the repo.

Usage: scripts/fuzz_seed_corpus.py [--max-frames N]    (default 40 frames per clip)
"""
import argparse
import pathlib
import struct
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
FRAME_TARGETS = ("frame_analysis", "obu_stream", "syntax_parser", "obu_parser")
FILE_TARGETS = ("ivf_parser", "av1_decode", "stream_analysis")


def ivf_frames(data: bytes):
    """Yield the payload of every frame of an IVF file; stop at the first truncated frame."""
    if data[:4] != b"DKIF" or len(data) < 32:
        raise ValueError("not an IVF file")
    header_len = struct.unpack_from("<H", data, 6)[0]
    pos = header_len
    while pos + 12 <= len(data):
        size = struct.unpack_from("<I", data, pos)[0]
        pos += 12
        if pos + size > len(data):
            return
        yield data[pos:pos + size]
        pos += size


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--max-frames", type=int, default=40)
    args = ap.parse_args()

    clips = sorted((ROOT / "test_data").glob("av1_*.ivf"))
    if not clips:
        print("no test_data/av1_*.ivf clips found", file=sys.stderr)
        return 1
    corpus = ROOT / "fuzz" / "corpus"
    for t in FRAME_TARGETS + FILE_TARGETS:
        (corpus / t).mkdir(parents=True, exist_ok=True)

    written = {t: 0 for t in FRAME_TARGETS + FILE_TARGETS}
    for clip in clips:
        data = clip.read_bytes()
        for t in FILE_TARGETS:
            (corpus / t / clip.stem).write_bytes(data)
            written[t] += 1
        for i, payload in enumerate(ivf_frames(data)):
            if i >= args.max_frames:
                break
            for t in FRAME_TARGETS:
                (corpus / t / f"{clip.stem}_{i:03d}").write_bytes(payload)
                written[t] += 1
    for t, n in written.items():
        print(f"fuzz/corpus/{t}: {n} seeds")
    return 0


if __name__ == "__main__":
    sys.exit(main())
