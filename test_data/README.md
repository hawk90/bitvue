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

## Derived fixtures elsewhere

| fixture | derived from | how |
|---|---|---|
| `crates/bitvue-metrics/tests/fixtures/vmaf/*.yuv` | `sample_h264.mp4` | see that directory's README |
| `crates/bitvue-decode/tests/fixtures/vvc/sample_176x144_9f_10bit.266` | `sample_h264.mp4`, first 9 frames | `ffmpeg -i sample_h264.mp4 -frames:v 9 -vf scale=176:144:flags=bicubic -pix_fmt yuv420p10le -f rawvideo in.yuv`, then `vvencapp -i in.yuv -s 176x144 -r 30 --format yuv420_10 --internal-bitdepth 10 --preset faster -q 32 -f 9 -o out.266` (vvenc 1.14); expected SHA-256s from `vvdecapp` 3.2.1, equal to FFmpeg's native VVC decoder (md5 of the whole output) |
