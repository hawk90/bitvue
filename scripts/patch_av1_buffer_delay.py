#!/usr/bin/env python3
"""Rewrites decoder_buffer_delay and encoder_buffer_delay (both 16 bits, 90 kHz units) in the
sequence header of av1_aomenc_decoder_model.ivf, where both are 45000, to make the stream break a
decoder model requirement without re-encoding.

Usage: patch_av1_buffer_delay.py in.ivf out.ivf delay"""
import sys

src, dst, delay = sys.argv[1], sys.argv[2], int(sys.argv[3])
data = bytearray(open(src, "rb").read())

# the sequence header OBU (type 1) is in the first IVF frame
pos = 32 + 12
assert data[pos] >> 3 & 15 == 2, "first OBU should be a temporal delimiter"
pos += 2  # temporal delimiter: header + size 0
assert data[pos] >> 3 & 15 == 1, "second OBU should be the sequence header"
size = data[pos + 1]
assert size < 0x80
start, end = pos + 2, pos + 2 + size

bits = "".join(f"{b:08b}" for b in data[start:end])
pattern = f"{45000:016b}" * 2
at = bits.find(pattern)
assert at >= 0 and bits.find(pattern, at + 1) < 0, "delay pair not found exactly once"
replacement = f"{delay:016b}" * 2
bits = bits[:at] + replacement + bits[at + 32 :]
data[start:end] = bytes(int(bits[i : i + 8], 2) for i in range(0, len(bits), 8))
open(dst, "wb").write(data)
print(f"patched bit offset {at} in the sequence header")
