# Bitvue VQA Parity Checklist

Phase 12 tracking for VQ Analyzer feature parity (V14 spec §7.3).

## How to use

- `[x]` = implemented and tested
- `[-]` = partial / in progress
- `[ ]` = not started

Run the regression suite to validate:
```
./scripts/run_regression_suite.sh
./scripts/parity_check.sh --local
```

---

## Layer 1: Codec Parsing & Frame Extraction

| ID | Codec | Feature | Status | Test |
|----|-------|---------|--------|------|
| L1-AV1-01 | AV1 | IVF frame extraction | [x] | `parity_test::av1_decode_runs_without_error` |
| L1-AV1-02 | AV1 | Frame stats (type, size, pts, offset) | [x] | `parity_test::av1_stats_flag_runs_without_error` |
| L1-AV1-03 | AV1 | Per-frame MD5 | [x] | `parity_test::av1_md5_flag_runs_without_error` |
| L1-AV1-04 | AV1 | max-frames limit | [x] | `parity_test::av1_max_frames_limit_respected` |
| L1-AV1-05 | AV1 | Auto-detect IVF | [x] | `parity_test::av1_autodetect_without_force_codec` |
| L1-AV1-06 | AV1 | Stream stats | [x] | `parity_test::av1_stream_stats_flag_runs_without_error` |
| L1-HEVC-01 | HEVC | Annex B NAL extraction | [-] | `parity_test::hevc_minimal_annexb_does_not_panic` |
| L1-HEVC-02 | HEVC | Frame stats (IDR/CRA/TRAIL) | [-] | `parity_check.sh §4` |
| L1-HEVC-03 | HEVC | --stream-stats NAL breakdown | [-] | `parity_check.sh §4` |
| L1-HEVC-04 | HEVC | Empty/garbage resilience | [x] | `parity_test::hevc_empty/garbage_*` |
| L1-AVC-01 | AVC | Annex B NAL extraction | [-] | `parity_test::avc_minimal_annexb_does_not_panic` |
| L1-AVC-02 | AVC | Frame stats (IDR/I/P/B) | [-] | `parity_check.sh §5` |
| L1-AVC-03 | AVC | Empty/garbage resilience | [x] | `parity_test::avc_empty/garbage_*` |
| L1-VP9-01 | VP9 | IVF VP90 frame extraction | [-] | `parity_test::vp9_minimal_ivf_*` |
| L1-VP9-02 | VP9 | Auto-detect VP90 FourCC | [x] | `parity_test::vp9_autodetect_from_ivf_fourcc_does_not_panic` |
| L1-VP9-03 | VP9 | KEY/INTER frame types | [-] | `parity_check.sh §6` |
| L1-VP9-04 | VP9 | Empty/garbage resilience | [x] | `parity_test::vp9_empty/garbage_*` |

**Notes:**
- `[-]` items require real fixture files in `test_data/` to fully validate.  
  Add `test_data/hevc_test.hevc`, `test_data/avc_test.h264`, `test_data/vp9_test.ivf` to unlock them.

---

## Layer 2: Visualization Modes (IA parity)

Mapped from `FULL_PARITY_MATRIX_JSON` P0/P1 items.

| ID | Item | Severity | Status | Notes |
|----|------|----------|--------|-------|
| IA-01 | Main panel: coding flow grid (CTB/CU/PU hierarchy) | P0 | [-] | AV1/HEVC partition overlay implemented |
| IA-02 | Timeline view (frame sizes, QP, filmstrip) | P0 | [-] | QP heatmap + filmstrip in progress |
| IA-03 | Syntax tree panel per codec | P0 | [-] | Frontend syntax panel pending |
| IA-04 | Selection info (CTB addr, MV, QP, pred mode) | P0 | [-] | Block selection overlay implemented |
| IA-05 | Hex view (raw bytes, offset, ASCII) | P0 | [ ] | Not started |
| IA-06 | Status panel (errors, warnings, stream info) | P1 | [-] | Status bar partial |

---

## Layer 3: Overlay Modes

| ID | Mode | Codecs | Status | Frontend Key |
|----|------|--------|--------|-------------|
| OV-01 | QP Heatmap | AV1, HEVC, AVC, VP9 | [x] | F2 |
| OV-02 | MV Field | AV1, HEVC, AVC | [x] | F1 |
| OV-03 | Partition grid | AV1, HEVC | [x] | F3 |
| OV-04 | CBF Luma | AV1, HEVC | [-] | F4 |
| OV-05 | Transform type | AV1 | [-] | F5 |
| OV-06 | Prediction mode | AV1, HEVC, AVC | [-] | F6 |
| OV-07 | CDEF | AV1 | [-] | F7 |
| OV-08 | Loop restoration | AV1 | [-] | F8 |
| OV-09 | Film grain | AV1 | [-] | F9 |
| OV-10 | Super-res | AV1 | [-] | F10 |

---

## Layer 4: Keyboard Shortcut Parity

All shortcuts verified against VQA reference (Phase 11).

| Shortcut | Action | Status |
|----------|--------|--------|
| ←/→ | Previous/next frame | [x] |
| Space | Next frame | [x] |
| Home/End | First/last frame | [x] |
| Ctrl+←/→ | Previous/next I-frame | [x] |
| [/] | Previous/next I-frame (VQA parity) | [x] |
| Ctrl+O | Open file | [x] |
| Ctrl+W | Close file | [x] |
| Ctrl+E | Export | [x] |
| Ctrl+S | Save frame PNG | [x] |
| Ctrl+G / Ctrl+F | Go to frame | [x] |
| Ctrl+R | Reload file | [x] |
| F | Toggle fullscreen | [x] |
| F11 | OS fullscreen | [x] |
| Escape | Exit fullscreen / clear selection | [x] |
| ? | Show shortcut help | [x] |
| Ctrl+Z | Undo selection | [x] |
| Ctrl+C | Copy block info | [x] |
| Y/U/V | Channel isolation | [x] |
| F1–F10 | Visualization mode switch | [x] |
| Ctrl+F1–F6 | Toggle info overlay | [x] |

---

## Layer 5: Export & CLI Parity

| Feature | Status | Command |
|---------|--------|---------|
| Frame export (PNG) | [x] | `bitvue export` |
| YUV dump (-o out.yuv) | [x] | `bitvue decode -o` |
| Y4M dump (--y4m) | [x] | `bitvue decode --y4m` |
| Per-frame MD5 (--md5) | [x] | `bitvue decode --md5` |
| PSNR (--psnr --reference) | [x] | `bitvue decode --psnr` |
| Batch analysis | [x] | `bitvue batch` |
| JSON export | [x] | `bitvue export --json` |

---

## Regression Suite Status

Run `./scripts/run_regression_suite.sh` and paste results here.

```
Last run: (not yet run)
PASS: -
FAIL: -
```

---

## Fixture Files Needed

To unlock `[-]` items, add these to `test_data/`:

| File | Description | How to obtain |
|------|-------------|---------------|
| `test_data/hevc_test.hevc` | Any valid HEVC Annex B clip | `ffmpeg -i input.mp4 -c:v libx265 -frames:v 30 -bsf:v hevc_mp4toannexb test_data/hevc_test.hevc` |
| `test_data/avc_test.h264` | Any valid AVC Annex B clip | `ffmpeg -i input.mp4 -c:v libx264 -frames:v 30 -bsf:v h264_mp4toannexb test_data/avc_test.h264` |
| `test_data/vp9_test.ivf` | VP9 in IVF container | `ffmpeg -i input.mp4 -c:v libvpx-vp9 -frames:v 30 -f ivf test_data/vp9_test.ivf` |
