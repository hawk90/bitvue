#!/usr/bin/env python3
"""Seed fuzz/corpus/<target>/ from the AV1 clips in test_data/.

Random bytes rarely get past the sequence header, so frame_analysis, obu_stream, syntax_parser
and obu_parser start from real IVF frame payloads (no 12-byte frame header); ivf_parser,
av1_decode and stream_analysis start from whole files; protocol_frames and tile_group_split start from a few hand-built inputs. The corpus is gitignored: this regenerates it, nothing from
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
TILE_TARGET = "tile_group_split"


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


def tile_group_seeds():
    """Layout bytes + tile group payload for tile_group_split (see fuzz_targets/tile_group_split.rs).

    Layout bytes: tile cols - 1, tile rows - 1, (TileColsLog2 << 4 | TileRowsLog2), TileSizeBytes - 1.
    """
    def sized(chunks, size_bytes):
        out = b""
        for c in chunks[:-1]:
            out += (len(c) - 1).to_bytes(size_bytes, "little") + c
        return out + chunks[-1]

    four = [b"\xaa" * 3, b"\xbb" * 5, b"\xcc" * 2, b"\xdd" * 4]
    # 4x1 tiles, no explicit range (flag bit 0 -> the byte after it is aligned padding)
    yield "all_tiles", bytes([3, 0, 0x20, 1]) + b"\x00" + sized(four, 2)
    # 4x1 tiles, tile_start_and_end_present_flag = 1, tiles 1..=2 (2 bits each): 1 01 10 -> 0b10110000
    yield "tiles_1_to_2", bytes([3, 0, 0x20, 1]) + b"\xb0" + sized(four[:2], 2)
    # 2x2 tiles, one-byte sizes
    yield "grid_2x2", bytes([1, 1, 0x11, 0]) + b"\x00" + sized(four, 1)
    # a single tile: the payload is the tile
    yield "single_tile", bytes([0, 0, 0, 0]) + b"\x12\x34"
    # a size field that points past the payload
    yield "size_past_end", bytes([3, 0, 0x20, 1]) + b"\x00" + b"\xff\xff" + b"\x01"
    yield "header_only", bytes([3, 0, 0x20, 1])


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
    (corpus / TILE_TARGET).mkdir(parents=True, exist_ok=True)
    written[TILE_TARGET] = 0
    for name, data in tile_group_seeds():
        (corpus / TILE_TARGET / name).write_bytes(data)
        written[TILE_TARGET] += 1
    for t, n in written.items():
        print(f"fuzz/corpus/{t}: {n} seeds")
    return 0


if __name__ == "__main__":
    sys.exit(main())
