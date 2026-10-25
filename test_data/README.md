# Test data

Small clips used by tests, the regression suite, fuzz seeding and the Electron self-test. Flat on purpose:
tests reach them as `../../test_data/<file>` (`include_bytes!`) or `test_data/<file>` (CWD = repo root).

Only synthetic or self-generated material belongs here. Do not add third-party clips (licence unclear
for a repo that ships under AGPL-3.0 and a commercial licence, and `sample_*` is bundled into the app).

## `sample_*` -- app demo clips (bundled)

`bitvue-desktop/package.json` `extraResources` ships exactly `sample_*` as `samples/` in the packaged
app (Welcome screen "Samples", `frontend/components/welcome/sampleCatalog.ts`).

Source: FFmpeg `testsrc2`, 352x288, 30 fps, 2 s (60 frames), yuv420p -- a synthetic test pattern, no
third-party content. FFmpeg 8, run in this directory:

```bash
enc() { ffmpeg -y -f lavfi -i "testsrc2=size=352x288:rate=30:duration=2" -pix_fmt yuv420p "$@"; }
enc -c:v libx264 -preset medium -crf 23 sample_h264.mp4
enc -c:v libx264 -preset medium -crf 23 -f h264 sample_h264.264
enc -c:v libx265 -preset medium -crf 28 -tag:v hvc1 sample_hevc.mp4
enc -c:v libx265 -preset medium -crf 28 -f hevc sample_hevc.265
enc -c:v libvpx-vp9 -b:v 500k -deadline good sample_vp9.mkv     # and .webm, and -f ivf for .ivf
enc -c:v libsvtav1 -crf 35 -f ivf sample_av1.ivf
```

| file | container | codec |
|---|---|---|
| `sample_av1.ivf` | IVF | AV1 (SVT-AV1) |
| `sample_h264.mp4`, `sample_h264.264` | MP4, Annex B | H.264 |
| `sample_hevc.mp4`, `sample_hevc.265` | MP4, Annex B | HEVC |
| `sample_vp9.ivf`, `.mkv`, `.webm` | IVF, MKV, WebM | VP9 |

Tests that read them: `bitvue-formats/tests/container_formats_test.rs` (MP4/Annex B extraction; the
expected sample sizes are ffprobe's packet sizes of `sample_h264.mp4`), and the derived fixtures below.

## Other clips

| file | what | provenance |
|---|---|---|
| `av1_aomenc_testsrc2.ivf`, `av1_rav1e_testsrc2.ivf`, `av1_svtav1_testsrc2.ivf` | same `testsrc2` pattern, 320x240, 12 frames, one per encoder (cross-encoder oracle tests vs dav1d) | `testsrc2` per the file names (command not recorded) |
| `av1_aomenc_tilegroups.ivf`, `av1_aomenc_tiles5.ivf` | aomenc, 320x240, 12 frames, several tiles / tile groups (#129) | presumably synthetic, command not recorded |
| `av1_aomenc_globalmotion.ivf` | aomenc 3.15.2, 352x288, 30 frames, synthetic zoom: `gm_type = ROTZOOM` on frames 4, 8, ... (the only clip with global motion; `oracle_test.rs`) | commands below |
| `av1_aomenc_superres.ivf` | aomenc 3.15.2, 352x288, 31 frames (30 shown + 1 hidden ALTREF) of `testsrc2`, every frame coded 313 wide (`SuperresDenom` 9) and upscaled to 352 (the only clip with superres; `oracle_test.rs`) | commands below |
| `av1_aomenc_superres12.ivf` | the superres clip with denominator 12 (coded 235 wide; 128x128 inter blocks with a residual, read per 64x64 chunk; `oracle_test.rs`) | commands below |
| `av1_aomenc_superres_random.ivf` | superres with a denominator chosen per frame (coded 176-352 wide), so most references have another size; no temporal vectors from them (`oracle_test.rs`) | commands below |
| `av1_aomenc_filmgrain.ivf` | aomenc 3.15.2, 352x288, 11 frames (10 shown + 1 hidden) of `testsrc2` with film grain test vector 5: scaling points for all planes, `update_grain = 0` on 9 frames (`oracle_test.rs`) | commands below |
| `av1_aomenc_filmgrain_cfl.ivf` | same source, film grain test vector 15: `chroma_scaling_from_luma` (`oracle_test.rs`) | commands below |
| `av1_aomenc_filmgrain_luma.ivf` | same source, film grain test vector 6: 14 luma points, no chroma points (`oracle_test.rs`) | commands below |
| `av1_fuzz_oversized_frame.ivf` | 5.6 KB, one key frame declaring 65536x65536 (no real picture data): minimized from a `stream_analysis` fuzz input that was itself a mutation of the clips above. Regression for the stateful decode allocating per declared frame size (`stream_state_test.rs`) | produced by the fuzzer: `cargo +nightly fuzz tmin -s none stream_analysis <artifact>` |
| `av1_fuzz_dav1d_pending_data.ivf` | 15 KB IVF of corrupt frames, minimized from an `av1_decode` fuzz input that was itself a mutation of the clips above. Regression for `Av1Decoder::send_data_owned` sending while dav1d still held unconsumed input, which panics inside the `dav1d` crate (`crates/bitvue-decode/tests/fuzz_regression_test.rs`) | produced by the fuzzer: `cargo +nightly fuzz tmin -s none av1_decode <artifact>` |
| `av1_fuzz_dav1d_oversized_picture.ivf` | 5.6 KB, one frame declaring 65536x65536 (the same shape as `av1_fuzz_oversized_frame.ivf`, found independently by the `av1_decode` target). Regression for dav1d allocating the picture of a declared size, over 10 GB, when `Av1Decoder` gave it no frame size limit (`crates/bitvue-decode/tests/fuzz_regression_test.rs`) | produced by the fuzzer: `cargo +nightly fuzz run -s none av1_decode` (oom artifact, not minimized) |
| `av1_fuzz_dav1d_frame_thread_hang.ivf` | 16 KB IVF of corrupt frames from an `av1_decode` fuzz timeout (unminimized). With dav1d frame threading on, repeated decodes of it hang in `dav1d_get_picture`; `Av1Decoder` sets `max_frame_delay = 1` (`crates/bitvue-decode/tests/fuzz_regression_test.rs`) | produced by the fuzzer: `cargo +nightly fuzz run -s none av1_decode` (timeout artifact) |
| `av1_aomenc_194x72.ivf` | aomenc 3.15.2, 194x72, 11 frames (10 shown + 1 hidden) of `testsrc2`: a width that is not a multiple of 8 (`oracle_test.rs`) | commands below |
| `av1_aomenc_114x136_palette.ivf` | aomenc 3.15.2, 114x136, 2 frames of `testsrc2`, palette on and most other tools off: blocks reaching past the right edge (`oracle_test.rs`) | commands below |
| `av1_test.ivf` | AV1, 320x240, 25 fps, 250 frames | command not recorded |
| `avc_test.h264`, `hevc_test.hevc`, `vp9_test.ivf` | 320x240 | x264 / x265 / libvpx; content and command not recorded |

The six rows marked "not recorded" predate this README (added in unrelated commits); they are
almost certainly synthetic (320x240 is not the size of any standard test sequence), but nothing in the
repository proves it and the exact commands were not kept.
Re-encode them from `testsrc2` if a file ever needs to be reproducible.

`av1_aomenc_globalmotion.ivf` (FFmpeg 8, aomenc; the zoom makes aomenc pick a ROTZOOM model on some frames):

```bash
ffmpeg -y -f lavfi -i "testsrc2=s=704x576:r=30:d=2,zoompan=z='1+0.004*on':x='iw/2-(iw/zoom/2)':y='ih/2-(ih/zoom/2)':d=1:s=352x288:fps=30" -pix_fmt yuv420p -f yuv4mpegpipe zoom.y4m
aomenc --cpu-used=1 --limit=30 --enable-global-motion=1 --lag-in-frames=0 --end-usage=q --cq-level=30 --ivf -o av1_aomenc_globalmotion.ivf zoom.y4m
```

`av1_aomenc_superres.ivf` (FFmpeg 8, aomenc):

```bash
ffmpeg -y -f lavfi -i "testsrc2=size=352x288:rate=30:duration=1" -pix_fmt yuv420p -f yuv4mpegpipe src.y4m
aomenc src.y4m --ivf -o av1_aomenc_superres.ivf --limit=30 --cpu-used=6 --end-usage=q --cq-level=30 --superres-mode=1 --superres-denominator=9 --superres-kf-denominator=9
```

`av1_aomenc_superres12.ivf` (same source as above):

```bash
aomenc src.y4m --ivf -o av1_aomenc_superres12.ivf --limit=30 --cpu-used=6 --end-usage=q --cq-level=30 --superres-mode=1 --superres-denominator=12 --superres-kf-denominator=12
```

`av1_aomenc_superres_random.ivf` (same source as above):

```bash
aomenc src.y4m --ivf -o av1_aomenc_superres_random.ivf --limit=30 --cpu-used=6 --end-usage=q --cq-level=30 --superres-mode=2
```

`av1_aomenc_filmgrain.ivf`, `av1_aomenc_filmgrain_cfl.ivf`, `av1_aomenc_filmgrain_luma.ivf` (same source as above; the number is aomenc's `--film-grain-test`):

```bash
aomenc src.y4m --ivf -o av1_aomenc_filmgrain.ivf --limit=10 --cpu-used=6 --end-usage=q --cq-level=30 --film-grain-test=5
aomenc src.y4m --ivf -o av1_aomenc_filmgrain_cfl.ivf --limit=10 --cpu-used=6 --end-usage=q --cq-level=30 --film-grain-test=15
aomenc src.y4m --ivf -o av1_aomenc_filmgrain_luma.ivf --limit=10 --cpu-used=6 --end-usage=q --cq-level=30 --film-grain-test=6
```

`av1_aomenc_194x72.ivf`:

```bash
ffmpeg -y -f lavfi -i "testsrc2=size=194x72:rate=30:duration=1" -pix_fmt yuv420p -f yuv4mpegpipe src.y4m
aomenc src.y4m --ivf -o av1_aomenc_194x72.ivf --limit=10 --cpu-used=6 --end-usage=q --cq-level=30
```

`av1_aomenc_114x136_palette.ivf`:

```bash
ffmpeg -y -f lavfi -i "testsrc2=size=114x136:rate=30:duration=1" -pix_fmt yuv420p -f yuv4mpegpipe src.y4m
aomenc src.y4m --ivf -o av1_aomenc_114x136_palette.ivf --limit=2 --cpu-used=6 --end-usage=q --cq-level=30 --tune-content=screen --enable-restoration=0 --enable-cdef=0 --enable-cfl-intra=0 --enable-filter-intra=0 --enable-intrabc=0 --enable-tx64=0 --enable-angle-delta=0 --enable-smooth-intra=0 --enable-paeth-intra=0 --enable-intra-edge-filter=0 --enable-rect-tx=0 --enable-rect-partitions=0 --enable-ab-partitions=0 --enable-1to4-partitions=0 --enable-palette=1
```

The dav1d digests in `oracle_test.rs` come from a dav1d build whose `ctx_norm` in `src/msac.c` prints
`rng cnt dif` after every symbol (C path, `-Denable_asm=false`), reduced per frame to the symbol count
and the FNV-1a digest of the `"{rng} {cnt} {dif}\n"` lines. The film grain digests come from the same build
with a second print at the end of `film_grain_params()` in `src/obu.c`: every field of `hdr->film_grain.data`
(after `load_grain_params`), one `key=value` line per frame that applies grain.

## Derived fixtures elsewhere

| fixture | derived from | how |
|---|---|---|
| `crates/bitvue-metrics/tests/fixtures/vmaf/*.yuv` | `sample_h264.mp4` | see that directory's README |
| `crates/bitvue-decode/tests/fixtures/vvc/sample_176x144_9f_10bit.266` | `sample_h264.mp4`, first 9 frames | `ffmpeg -i sample_h264.mp4 -frames:v 9 -vf scale=176:144:flags=bicubic -pix_fmt yuv420p10le -f rawvideo in.yuv`, then `vvencapp -i in.yuv -s 176x144 -r 30 --format yuv420_10 --internal-bitdepth 10 --preset faster -q 32 -f 9 -o out.266` (vvenc 1.14); expected SHA-256s from `vvdecapp` 3.2.1, equal to FFmpeg's native VVC decoder (md5 of the whole output) |
