#!/usr/bin/env python3
"""Seed fuzz/corpus/<target>/ from the AV1 clips in test_data/.

Random bytes rarely get past the sequence header, so frame_analysis, obu_stream, syntax_parser
and obu_parser start from real IVF frame payloads (no 12-byte frame header); ivf_parser,
av1_decode and stream_analysis start from whole files; protocol_frames starts from a few hand-built frames. The corpus is gitignored: this regenerates it, nothing from
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
PROTOCOL_TARGET = "protocol_frames"


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


def protocol_seeds():
    """Framed control/data/event messages as bitvue-desktop writes them (see bitvue-protocol)."""
    def frame(kind: int, cid: int, body: bytes) -> bytes:
        return struct.pack("<BII", kind, cid, len(body)) + body

    hello = b'{"id":1,"method":"hello","params":{"client_version":"0.1.0"}}'
    cancel = b'{"id":2,"method":"cancel_request","params":{"target_id":1}}'
    ok = b'{"id":1,"ok":true,"result":{"protocol_version":"0.1.0","capabilities":[]}}'
    err = b'{"id":3,"ok":false,"error":{"code":"PARSE","message":"bad OBU","offset":4096}}'
    yield "hello", frame(0, 1, hello)
    yield "session", frame(0, 1, hello) + frame(0, 2, cancel) + frame(1, 2, bytes(range(64)))
    yield "responses", frame(0, 1, ok) + frame(0, 3, err) + frame(2, 0, b"{}")
    yield "empty_payload", frame(1, 9, b"")
    yield "truncated", frame(0, 1, hello)[:-5]
    yield "huge_declared_length", struct.pack("<BII", 0, 1, 0xFFFFFFFF)


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
    (corpus / PROTOCOL_TARGET).mkdir(parents=True, exist_ok=True)
    written[PROTOCOL_TARGET] = 0
    for name, data in protocol_seeds():
        (corpus / PROTOCOL_TARGET / name).write_bytes(data)
        written[PROTOCOL_TARGET] += 1
    for t, n in written.items():
        print(f"fuzz/corpus/{t}: {n} seeds")
    return 0


if __name__ == "__main__":
    sys.exit(main())
