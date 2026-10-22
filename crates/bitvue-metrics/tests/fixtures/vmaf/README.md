# VMAF reference fixtures

Raw 8-bit I420, 176x144, 5 frames each, derived from `test_data/sample_h264.mp4` (FFmpeg
`testsrc2`, a synthetic pattern, see `test_data/README.md`). Used by `tests/vmaf_reference.rs`, which checks
bitvue's VMAF against the scores Netflix's libvmaf produces through FFmpeg.

| file | content |
|---|---|
| `ref_176x144_5f.yuv` | first 5 frames, bicubic-scaled to 176x144 |
| `dist_176x144_5f.yuv` | same, then Gaussian blur (sigma 1.5) and temporal noise (strength 10) |

The `noise` filter is random, so regenerating `dist_*` gives a different file and different scores: keep
the committed `.yuv` and update the expected values when you do.

Regenerate (FFmpeg with `--enable-libvmaf`), from the repository root:

```bash
V=test_data/sample_h264.mp4; FX=crates/bitvue-metrics/tests/fixtures/vmaf
ffmpeg -y -i $V -frames:v 5 -vf "scale=176:144:flags=bicubic" -pix_fmt yuv420p -f rawvideo $FX/ref_176x144_5f.yuv
ffmpeg -y -i $V -frames:v 5 -vf "scale=176:144:flags=bicubic,gblur=sigma=1.5,noise=alls=10:allf=t" \
  -pix_fmt yuv420p -f rawvideo $FX/dist_176x144_5f.yuv

# expected scores (libvmaf 3.2.0, model vmaf_v0.6.1)
ffmpeg -y -f rawvideo -pix_fmt yuv420p -s 176x144 -i $FX/dist_176x144_5f.yuv \
  -f rawvideo -pix_fmt yuv420p -s 176x144 -i $FX/ref_176x144_5f.yuv \
  -lavfi "[0:v][1:v]libvmaf=log_fmt=json:log_path=/tmp/vmaf.json" -f null -
```

If the fixtures are regenerated, update the expected values in `tests/vmaf_reference.rs`.
