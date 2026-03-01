# Bitvue Sample Video Files

This directory contains sample video files for testing all supported container and codec formats.

## Source

All samples are derived from the **Xiph.org DERF collection** - standard test sequences used in video codec development and research.

- Source: https://media.xiph.org/video/derf/
- Sequence: `foreman_cif.y4m` (352x288, 30fps)
- Duration: 2 seconds (60 frames)
- License: Public domain / Research use

## Files

| File | Container | Video Codec | Size | Description |
|------|-----------|-------------|------|-------------|
| `foreman_h264.mp4` | MP4 | H.264/AVC | 94KB | MP4 container with H.264 video |
| `foreman_hevc.mp4` | MP4 | H.265/HEVC | 52KB | MP4 container with HEVC video |
| `foreman_vp9.mkv` | MKV | VP9 | 123KB | Matroska container with VP9 video |
| `foreman_vp9.webm` | WebM | VP9 | 123KB | WebM container with VP9 video |
| `foreman_vp9.ivf` | IVF | VP9 | 123KB | IVF raw container with VP9 frames |
| `foreman_av1.ivf` | IVF | AV1 | 218KB | IVF raw container with AV1 frames |
| `foreman_h264.264` | Annex B | H.264/AVC | 92KB | Raw H.264 byte stream |
| `foreman_hevc.265` | Annex B | H.265/HEVC | 51KB | Raw HEVC byte stream |

## Specifications

- **Resolution**: 352x288 (CIF)
- **Frame Rate**: 30 fps
- **Frames**: 60 (2 seconds)
- **Content**: Foreman test sequence (standard video codec test sequence)
- **Source**: Xiph.org DERF collection

## Usage

```bash
# CLI
bitvue info --file samples/foreman_h264.mp4
bitvue frames --file samples/foreman_vp9.ivf

# Tauri App
# File → Open → select any sample file
```

## Encoding Details

Generated with FFmpeg 8.0:

```bash
# H.264 MP4
ffmpeg -i foreman_cif.y4m -t 2 -c:v libx264 -preset medium -crf 23 -pix_fmt yuv420p foreman_h264.mp4

# HEVC MP4
ffmpeg -i foreman_cif.y4m -t 2 -c:v libx265 -preset medium -crf 28 -pix_fmt yuv420p foreman_hevc.mp4

# VP9 MKV/WebM/IVF
ffmpeg -i foreman_cif.y4m -t 2 -c:v libvpx-vp9 -b:v 500k -deadline good -pix_fmt yuv420p foreman_vp9.mkv

# Raw H.264 Annex B
ffmpeg -i foreman_cif.y4m -t 2 -c:v libx264 -preset medium -crf 23 -pix_fmt yuv420p -f h264 foreman_h264.264

# Raw HEVC Annex B
ffmpeg -i foreman_cif.y4m -t 2 -c:v libx265 -preset medium -crf 28 -pix_fmt yuv420p -f hevc foreman_hevc.265
```

## References

- [Xiph.org DERF Collection](https://media.xiph.org/video/derf/)
- [AOMedia Test Vectors](https://aomedia.googlesource.com/aom/)
- [FFmpeg Documentation](https://ffmpeg.org/documentation.html)
