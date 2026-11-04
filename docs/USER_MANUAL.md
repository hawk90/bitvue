# Bitvue User Manual

Bitvue opens a video bitstream (a compressed video file) and shows **how each frame was compressed**. It does not just play
the picture: it shows the coding information behind it (block partitioning, prediction, transforms, filters, reference
structure, QP and more).

This manual explains how to use the application and what a screen looks like when everything works. Development status is
tracked in `docs/specs/features.yaml`. Where the application differs from what is described here, the difference is listed
in [Known issues](#known-issues) with its issue number.

- Screen layout and terminology follow the VQ Analyzer user guide (v7.10.0).
- Supported in the desktop app: **AV1 in IVF files**. Other codecs cannot be opened in the app yet (see [Features by codec](#features-by-codec)).
- The screens in this manual were checked with the sample clip `test_data/av1_aomenc_testsrc2.ivf` (320x240, 12 frames).

## 1. Getting started

1. Start the application and choose `File` > `Open bitstream...` to open an `.ivf` file.
2. When indexing (building the frame list) finishes, every part of the window is filled in.
3. To compare with a second stream of the same format, choose `File` > `Open dependent bitstream...`.

With the sample clip, a healthy screen looks like this:

| Where | Expected |
|---|---|
| Filmstrip title | `FILMSTRIP (12 FRAMES)` |
| Top right of the timeline | `1 / 12` |
| `INFO` panel | `Frames: 12`, `Current Frame: 1`, the file path |
| Status bar at the bottom | the file path and `12 frames` |

## 2. Window layout

```
┌──────────────────────────────────────────────┐
│ TIMELINE   one colored cell per frame        │  overview
│ FILMSTRIP  frame thumbnails (or other views) │  pick a frame
├───────────┬──────────────────────────────────┤
│ Stream    │ toolbar: move / play / Mode /    │
│ Tree      │          Overlay                 │
│ Syntax    │                                  │
│ Detail    │   player (screen of the Mode)    │
├───────────┴──────┬──────────┬────────┬───────┤
│ INFO             │ DETAILS  │ STATS  │ DIAGNOSTICS │  frame information
├──────────────────┴──────────┴────────┴───────┤
│ status bar: file path, frame count           │
└──────────────────────────────────────────────┘
```

| Area | Purpose |
|---|---|
| `TIMELINE` | One cell per frame. The current frame is highlighted and `current / total` is shown at the top right. Red is an I (key) frame, green is a P frame |
| `FILMSTRIP` | A row of frame thumbnails. The current frame has a white border. The drop-down at the top right changes the view ([chapter 4](#4-filmstrip-views)) |
| Toolbar | Frame navigation buttons, frame number box (`n / total`), play, speed (`1x`), codec badge (`AV1`), `Mode`, `Overlay`, full screen, zoom |
| Player | The picture for the selected `Mode`. The bottom line shows `Frame n / total`, `Zoom`, the mode name and `Paused` |
| Left side | `Stream Tree` (the frame list as a tree) and `Syntax Detail` (the syntax of one frame) |
| Four bottom panels | `INFO`, `DETAILS`, `STATS`, `DIAGNOSTICS` |

## 3. Moving between frames

| How | Result |
|---|---|
| Keyboard `Left` / `Right` | Previous / next frame |
| Navigation buttons in the toolbar | First, previous, next, last frame |
| Frame number box | Type a frame number |
| Click a thumbnail | Goes to that frame (the player, `INFO`, `DETAILS` and the reference arrows all update) |
| Play button and speed (`1x`) | Present in the toolbar (playback itself is listed in [Features not yet verified](#10-features-not-yet-verified)) |

When the current frame is a P frame, **reference arrows** are drawn in the filmstrip. Lines run from the current frame to
the frames it references, labelled `REF0`, `REF1`, ... (sample clip, frame 3: `REF0` to `REF5` point to frame 0, `REF6`
points to frame 1). Hovering over a thumbnail shows a tooltip with the frame number, type, `Size`, `PTS`, `Temporal` and
`References`.

## 4. Filmstrip views

Choose a view with the drop-down at the top right of the filmstrip (it shows `Thumbnails` by default).

| View | What it shows | How to read it |
|---|---|---|
| `Thumbnails` | Frame thumbnails | Red border = I, green border = P. The `D:` and `C:` numbers are display and coding order |
| `Frame Sizes` | A bar per frame | Taller bars cost more bits. The left axis is size (`BITRATE`), the right axis is QP. The pop-up at the top right switches `Bitrate Bar`, `Bitrate Curve`, `Avg Size`, `Min Size`, `Max Size`, `Moving Avg`, `Block Min QP` and `Block Max QP` on and off |
| `B-Pyramid` | Reference hierarchy | Frames are dots on a line. Red = I, green = P. `Show All Refs` shows every reference |
| `HRD Buffer` | Decoder buffer fullness | Meaningful only when the stream carries HRD information |
| `Enhanced` | GOP and scene navigation | `GOP` previous/next buttons, `SCENE` previous/next buttons and scene-change markers |

## 5. Player modes (`Mode`)

Change the mode with the `Mode` drop-down or the keys `F1` to `F10`. Each mode draws a different part of the bitstream on top
of the picture. **If the stream does not use a tool, its mode simply shows `...: disabled` in the middle of the screen.** That
is not a fault.

| Key | Mode | What it shows | How to read it |
|---|---|---|---|
| `F1` | `Coding Flow` | Block partition (split) boundaries | Dense lines are heavily split (complex) areas. The top-left label gives the block count (`Blocks`) and the superblock size (`SB`) |
| `F2` | `Predictions` | The prediction mode of each block | Intra frames are colored by direction mode (`D45`, `SmoothV`, `Paeth`, `CFL`, ...). Inter frames are colored by inter mode (`NewMV`, `NearMV`, ...). See the legend at the bottom |
| `F3` | `Transform` | Transform block sizes | Colored grid per size with counts (for example `8x8 (78)`, `64x64 (88)`). The top-left label gives `Blocks` and `Types` |
| `F4` | `Reconstruction` | The reconstructed picture before the loop filters | The picture as it was before any loop filter was applied |
| `F5` | `Loop Filter` | The deblocking filter | Number of filtered edges, filter strength and the loop-filter parameters (`Level`, `Sharpness`, `Ref/Mode Deltas`) |
| `F6` | `CDEF Filter` | The CDEF filter | A block grid with `CDEF Y-Primary`, `Y-Secondary` and `Damping`. Filter direction is drawn as arrows |
| `F7` | `SuperRes Filter` | Super-resolution | Meaningful only for streams that use super-resolution |
| `F8` | `Loop Restoration` | The loop restoration filter | Filter type and block size. Shows `disabled` when not used |
| `F9` | `Film Grain Pixels` | Film grain synthesis | Meaningful only for streams with film grain. Otherwise `disabled` |
| `F10` | `YUV` | The decoded picture with no coding information | The picture itself |

In the sample clip, `F6`, `F7`, `F8` and `F9` show `disabled` on frame 0 (the key frame), and that is correct. The frame uses
intra block copy (blocks copied from earlier parts of the same frame), and in AV1 that removes the loop filters from the frame
header; super-resolution and film grain are switched off in the sequence. On frame 3 (P) you should see `Level` `1 / 2` (Y
vertical/horizontal) and `1 / 8` (U / V) and `Sharpness 0` in `F5`, and `Y-Primary 0`, `Y-Secondary 1`, `Damping 4` in `F6`.

## 6. Information overlays (`Overlay`)

An overlay is an information map drawn on top of a mode. Switch overlays on and off with the `Overlay` buttons. `Ctrl+F1` to
`Ctrl+F6` toggle them in list order.

| Button | Name | What it shows |
|---|---|---|
| `BT` | `Block Type` | Blocks colored by kind: `INTRA`, `INTER`, `SKIP`, `INTRA_BC`, `COMPOUND` (legend at the top left) |
| `HM` | `Heat Map` | Bits spent per block (brighter = more bits). The application currently draws motion-vector magnitude instead ([Known issues](#known-issues)) |
| `EFF` | `Efficiency` | Intended: coding efficiency per block (brighter = more efficient). The application currently draws bits per pixel instead, with a `low bpp` to `high bpp` legend ([Known issues](#known-issues)) |
| `PSNR` | `PSNR` | PSNR per block. Needs a reference YUV file |

## 7. Left side panels

`Stream Tree` lists the frames, one line per frame, in the form `[F] Frame #0 - I @ 0x00000020 (3745 bytes)` (type, file offset and size; the size includes the 12-byte IVF frame header). It has a `Filter` checkbox.

Below it a drop-down switches the lower panel between four views:

| View | What it shows |
|---|---|
| `Syntax` | `SYNTAX DETAIL` with the tabs `FRAME`, `REFS`, `STATS` and `SEARCH` (table below) |
| `Selection` | `SELECTION INFO` for the current frame: `Frame Index`, `Frame Type`, `Size`, `PTS`, `Temporal ID`, `References` (sample clip, frame 3: `3`, `P`, `1.02 KB`, `3`, `0`, `7`) |
| `Unit HEX` | `UNIT HEX` with the tabs `Frame`, `Hex` and `DPB` (table below) |
| `YUV Diff` | Compare against a reference YUV file ([Features not yet verified](#10-features-not-yet-verified)) |

`Syntax` tabs:

| Tab | Content |
|---|---|
| `FRAME` | `Frame:` (number), `Type:` (I/P) and the OBU tree (`obu_0`, ...). Expand the tree to see syntax elements |
| `REFS` | The references of the current frame as an `L0` list (sample clip, frame 3: `L0`, `6 references`, rows such as `[3] #0 POC -3` where `#0` is the referenced frame). On frame 0 it shows only a "no data" icon |
| `STATS` | Statistics of the current frame, starting with `Current Frame`: `Index`, `Type`, `Size`, `Temporal ID` |
| `SEARCH` | Find a frame by type, number or PTS (`Search by frame type, number, or PTS...`) |

`Unit HEX` tabs:

| Tab | Content |
|---|---|
| `Frame` | The frame's number, type and size (frame 0: `3.66 KB (3745 bytes)`) |
| `Hex` | A hex dump of the frame's first 2048 bytes (`Data: First 2048 bytes (of 3745 total)`), 16 bytes per row with an offset column. The first 12 bytes are the IVF frame header, so the AV1 data starts at byte 12 (sample clip, frame 0: `12 00 0A 0B` for a temporal delimiter followed by a sequence header) |
| `DPB` | The decoded picture buffer: `Current Frame` and a table with `Idx`, `Frame`, `Type`, `PTS`, `Status` |

> At the default window size the left panel is short and shows only a row or two. Enlarge the window to read it (see [Known issues](#known-issues)).

## 8. Bottom panels

| Panel | Content | Expected value (sample clip, frame 3) |
|---|---|---|
| `INFO` | File path, total frames, current frame, type | `Frames: 12`, `Current Frame: 4`, `Frame: P` |
| `DETAILS` | `Temporal Layer`, `Display Order`, `Coding Order`, `References` | `0`, `3`, `3`, `0, 0, 0, 0, 0, 0, 1` |
| `STATS` | Count of each frame type (`Frame Types`), `Total`, `Keyframes`, `Avg Size` | Sample clip: 1 I frame and 11 P frames, `Total: 12`, `Keyframes: 1` (the application currently shows `I: 4`, see [Known issues](#known-issues)) |
| `DIAGNOSTICS` | Errors, warnings and notes found in the stream (`All`, errors, warnings, info) | No problems: all `0` and a check mark |

## 9. Comparing two streams (`A/B Compare`)

Open a second stream with `File` > `Open dependent bitstream...`. The window changes to the `A/B Compare` workspace, which shows stream A and stream B side by side.

| Part | Content |
|---|---|
| Toolbar | `unlink` / `Sync` (follow the other stream), `Offset` with `-`, `+` and `Reset`, a match badge (`PtsExact High`), the layout switch `Side-by-Side` / `Split`, the `Show Diff` checkbox, `Temperature`, `Find First Diff`, `Export Diff Bundle` |
| Stream panels | `Stream A` and `Stream B`, each with its size and frame count (`320x240 • 12 frames`), its own navigation buttons and frame box, and a badge with the frame number, type and size (`A: 1 / 12`, `I`, `3.7 KB`) |
| Diff panel | `Temperature Map (real diff heatmap)` with `min`/`max` and a `Low` to `High` scale. Identical streams give `min: 0.0  max: 0.0` |
| Bottom line | `Gap: 0 frames`, `Offset: 0` and a confidence badge (`High`); an `EXACT` badge when the two streams match exactly |

## 10. Features not yet verified

These are described in the reference guide but have not yet been checked in this application. Each is tracked as an issue; once
verified, it moves into the chapters above.

| Feature | Issue |
|---|---|
| All menu items (`File`, `Mode`, `YUVDiff`, `Options`, `View`, `Help`) | #463 |
| Full-stream analysis button; Metric view (PSNR/SSIM) | #464, #465 |
| Thumbnail right-click and jump to a number; interaction in the frame-size graph and in B-Pyramid | #466, #467, #468 |
| Player buttons (`Pic`, `Info`, `Y/U/V`, ...) and mouse operations (select, zoom, pan) | #469, #470 |
| Detail view of each mode; decode-order numbers, tiles and segment colors in `Coding Flow` | #471, #472 |
| Motion vectors and residual picture on inter frames; the `EFF` and `PSNR` overlays | #473, #474, #475 |
| `Syntax Detail` tabs for OBU, header, block, references and statistics | #476, #477, #478, #479, #480 |
| Selection information for a selected block (the frame-level `Selection` view works); status and error panel | #481, #483 |
| Dual view operations (master/follower sync, `Delta`); Debug YUV; opening a raw YUV file | #484, #485, #486 |
| Mode screens for streams that use loop restoration, super-resolution or film grain | #487 |

## Features by codec

Codecs support different coding tools. A mode is only meaningful when the stream actually uses the corresponding tool. For each
tool the tables below give the encoder option that switches it on, how it shows up in the stream, where an analyzer displays it,
and Bitvue's status.

| Column | Source |
|---|---|
| Encoder option | Copied from the help output of the encoder installed on the test machine (`aomenc`, `x265`, `x264`) |
| In the stream | Header fields reported by `ffmpeg -v trace -i clip -c copy -bsf:v trace_headers -f null -` and, for AV1, the libdav1d log (`size ... upscaled ... bitdepth ... tiles ...`). A clip was generated for each entry unless the cell says "not tested" |
| Shown in the analyzer | Mode and tab names from the VQ Analyzer user guide. "Not covered" means the guide has no text for it |
| Bitvue status | Item id and status in `docs/specs/features.yaml`. This table does not decide status |

All test clips are generated (320x240, 12 frames, from `ffmpeg`'s `testsrc2` source) and are not stored in the repository.

### AV1

Modes in the reference guide: Coding Flow, Predictions, Transform, Reconstruction, Loop Filter, CDEF Filter, SuperRes Filter, Loop restoration Filter, Film Grain Pixels, YUV, Info Overlays (Heat Map, Efficiency Map, Block Type, PSNR), Simple Motion.

| Tool | Encoder option (aomenc) | In the stream | Shown in the analyzer | Bitvue status |
|---|---|---|---|---|
| Superblock 64/128 | `--sb-size` | `use_128x128_superblock=0` with `--sb-size=64` | Coding Flow; selection info | CODEC-021 done |
| Partition types | `--enable-rect-partitions`, `--enable-ab-partitions`, `--enable-1to4-partitions` | Not tested | Coding Flow, Predictions | No item |
| Tiles | `--tile-columns`, `--tile-rows` (log2), `--num-tile-groups` | libdav1d `tiles 2x2` (default `1x1`) | Coding Flow (tile boundaries); selection info | CODEC-021 done |
| Segmentation | `--aq-mode` | `segmentation_enabled=1` with `--aq-mode=1` | Coding Flow (segment colors); selection info | CODEC-025 done |
| Per-block delta QP | `--deltaq-mode` | `delta_q_present=1` in the sample clip | QP axis of the frame-size graph | OVL-042 done |
| Quantizer matrices | `--enable-qm`, `--qm-min`, `--qm-max` | `using_qmatrix=1` | Frame header syntax | CODEC-025 done |
| Loop filter | `--loopfilter-control`, `--sharpness` | `loop_filter_level[0..3]`, `loop_filter_sharpness` | `F5` Loop Filter | OVL-017 done |
| CDEF | `--enable-cdef` | `enable_cdef=0` clip versus default clip (`cdef_*` fields) | `F6` CDEF Filter | OVL-007 partial |
| Loop restoration | `--enable-restoration` | Noisy source with `--cq-level=50`: `lr_type` is not 0 (it stays 0 on a synthetic source) | `F8` Loop Restoration | OVL-008 partial |
| Super-resolution | `--superres-mode`, `--superres-denominator`, `--superres-qthresh` | `use_superres=1`, `coded_denom=3` | `F7` SuperRes Filter | OVL-010 done |
| Film grain | `--film-grain-test`, `--film-grain-table`, `--denoise-noise-level` | `apply_grain=1`, `num_y_points=14` | `F9` Film Grain Pixels | OVL-009 done |
| Frame size change | `--resize-mode`, `--resize-denominator` | `frame_size_override_flag=1`, `frame_width_minus_1=212` | Not covered | No item |
| Screen content tools (palette, IntraBC) | `--tune-content=screen`, `--enable-palette`, `--enable-intrabc` | `allow_screen_content_tools=1`. The sample clip uses intra block copy (`INTRA_BC` in Block Type) | Predictions, Block Type | CODEC-021 done |
| Intra tools | `--enable-filter-intra`, `--enable-smooth-intra`, `--enable-paeth-intra`, `--enable-cfl-intra`, `--enable-angle-delta`, `--enable-intra-edge-filter` | The sample clip's `F2` legend shows `D45`, `SmoothV`, `Paeth`, `CFL` (per-option check not done) | `F2` Predictions | OVL-006 done |
| Inter tools | `--enable-obmc`, `--enable-warped-motion`, `--enable-global-motion`, `--enable-dual-filter`, `--enable-masked-comp`, `--enable-interintra-comp`, `--enable-dist-wtd-comp`, `--enable-onesided-comp` | Not tested | `F2` Predictions | OVL-041 done, CODEC-029 todo, QA-019 todo |
| Transforms | `--enable-tx64`, `--enable-rect-tx`, `--enable-flip-idtx`, `--reduced-tx-type-set` | Not tested | `F3` Transform | OVL-005 partial |
| Reference structure | `--lag-in-frames`, `--auto-alt-ref`, `--enable-fwd-kf`, `--min-gf-interval`, `--max-gf-interval`, `--max-reference-frames`, `--enable-order-hint`, `--enable-ref-frame-mvs` | Sample clip: three hidden frames and repeated frames (`show_existing_frame`) | Filmstrip reference arrows; `REFS` tab | CODEC-007 done, SYN-004 partial |
| Error resilience, parallelism | `--error-resilient`, `--frame-parallel` | `error_resilient_mode=1` | Not covered | No item |
| S-frames | `--sframe-dist`, `--sframe-mode` | Not tested | Not covered | No item |
| Bit depth, chroma | `--bit-depth`, `--input-bit-depth`, `--profile`, `--monochrome` | `high_bitdepth=1` (10-bit), `mono_chrome=1`, `seq_profile=1` with `yuv444p` (4:4:4) | Selection info | CODEC-008 done |
| Timing, decoder model | `--timing-info` | Default is `timing_info_present_flag=0` | `HRD Buffer` view | See [Known issues](#known-issues) |
| Large-scale tile, Annex B | `--large-scale-tile`, `--annexb` | Not tested | (Guide: AV1 input is IVF/WebM only) | The app opens IVF only |

Decoder-side references used to check the analyzer's screens:

| Purpose | Command |
|---|---|
| Reconstructed picture before the loop filters | `dav1d --inloopfilters none` (`nodeblock`, `nocdef`, `norestoration` remove single stages) |
| Before and after film grain | `dav1d --filmgrain 0` / `--filmgrain 1` |
| Scalable streams | `dav1d --oppoint N --alllayers 0/1` |
| Per-frame header values | `ffmpeg -v trace -i clip.ivf -c copy -bsf:v trace_headers -f null -` |

Making a test clip per feature: generate the source with
`ffmpeg -f lavfi -i "testsrc2=size=320x240:rate=30,format=yuv420p" -frames:v 12 src.y4m`, then run
`aomenc --limit=12 --cpu-used=4 --threads=1 --end-usage=q --cq-level=30 -o out.ivf src.y4m` with the extra options below.

| Feature | Extra options | Observed result |
|---|---|---|
| Tiles 2x2 | `--sb-size=64 --tile-columns=1 --tile-rows=1` | libdav1d `tiles 2x2` |
| Super-resolution | `--superres-mode=1 --superres-denominator=12` | `use_superres=1` |
| Film grain | `--film-grain-test=1` | `apply_grain=1` |
| Segmentation | `--aq-mode=1` | `segmentation_enabled=1` |
| 10-bit | `--bit-depth=10 --input-bit-depth=8` | `high_bitdepth=1` |
| Monochrome | `--monochrome` | `mono_chrome=1` |
| Quantizer matrices | `--enable-qm=1` | `using_qmatrix=1` |
| Error resilience | `--error-resilient=1` | `error_resilient_mode=1` |
| Frame size change | `--resize-mode=1 --resize-denominator=12` | `frame_size_override_flag=1` |
| CDEF and restoration off | `--enable-restoration=0 --enable-cdef=0` | `enable_cdef=0`, `enable_restoration=0` |
| Loop restoration on | Add `noise=alls=25:allf=t` to the source; `--cpu-used=1 --cq-level=50 --enable-restoration=1 --lag-in-frames=0 --auto-alt-ref=0` | `lr_type` is not 0 |
| 4:4:4 | Source with `format=yuv444p`; `--profile=1` | `seq_profile=1` |

### HEVC (H.265)

Modes in the reference guide: Coding Flow, Predictions, Transform, Reconstruction, Loop Filter, SAO, YUV, Info overlays (Heat Map, MV Heat, QP Map, PSNR, SSIM, PU Type, PU Reference Indices), Simple Motion.
The guide describes HEVC per mode, not per coding tool, so "Not covered" is common below. Encoder: `x265` (invoked through `ffmpeg -x265-params ...`).

| Tool | Encoder option (x265) | In the stream | Shown in the analyzer | Bitvue status |
|---|---|---|---|---|
| CTU size, CU quadtree | `--ctu`, `--min-cu-size` | `ctu=16:min-cu-size=8` changes the SPS block-size fields | Predictions (quadtree splits in blue); Coding Flow (CTB order) | OVL-018 partial, OVL-015 done |
| TU size and depth | `--max-tu-size`, `--tu-intra-depth`, `--tu-inter-depth`, `--limit-tu` | Not tested | Transform; `TU` tab | OVL-018 partial |
| Rectangular and asymmetric PUs | `--rect`, `--amp` (AMP needs `rect`) | `amp_enabled_flag=1` (0 by default) | Predictions (PUs in each CU); PU Type overlay | OVL-018 partial, OVL-024 todo |
| Intra prediction, strong smoothing | `--strong-intra-smoothing`, `--fast-intra`, `--rdpenalty` | `strong_intra_smoothing_enabled_flag=0` when switched off (1 by default) | Predictions; prediction detail mode | OVL-018 partial, OVL-036 todo |
| Constrained intra | `--constrained-intra` | `constrained_intra_pred_flag=1` | Not covered | CODEC-009 partial |
| Merge, skip, temporal MV prediction | `--max-merge`, `--temporal-mvp`, `--early-skip` | `sps_temporal_mvp_enabled_flag=1` by default (switch-off clip not made) | Predictions; MV Heat overlay; Simple Motion | OVL-003 partial, OVL-026 todo |
| Weighted prediction | `--weightp`, `--weightb` | `weighted_pred_flag=1`, `weighted_bipred_flag=1`. Whether weight tables are used is not tested (`testsrc2` has no fades) | Not covered | CODEC-009 partial |
| Multiple references, RPS, DPB | `--ref`, `--limit-refs`, `--log2-max-poc-lsb` | `bframes=4:b-pyramid=1:ref=4` gives `sps_max_dec_pic_buffering_minus1[0]=4`, `sps_max_num_reorder_pics[0]=2`, explicit `short_term_ref_pic_set` | Stream View > Active/DPB refs; `Ref Lists` tab | SYN-006 partial (no DPB-specific item) |
| B-pyramid | `--b-pyramid`, `--bframes`, `--b-adapt`, `--temporal-layers` | Frame types `I B B B B P B P ...`. `--temporal-layers` did not produce sub-layers in the header (`sps_max_sub_layers_minus1=0`): unconfirmed | Frame types in Stream View | CODEC-002 done |
| IDR/CRA, open GOP | `--keyint`, `--min-keyint`, `--open-gop`, `--cra-nal`, `--radl`, `--scenecut` | NAL types `IDR_N_LP`, `TRAIL_R`, `TRAIL_N`. CRA/RASL clip not made | `NAL` tab (units in decode order) | CODEC-002 done |
| Transform skip | `--tskip`, `--tskip-fast` | `transform_skip_enabled_flag=1` (0 by default) | Transform mode (residual) | OVL-018 partial |
| Sign data hiding | `--signhide` | `sign_data_hiding_enabled_flag=1` by default | Not covered | CODEC-009 partial |
| Scaling lists (QM) | `--scaling-list` | `scaling_list_enabled_flag=1` with `default` | `QM` tab | SYN-012 partial |
| QP, cu_qp_delta, AQ | `--aq-mode`, `--qg-size`, `--cbqpoffs`, `--crqpoffs`, `--qp`, `--crf` | `aq-mode=2:qg-size=16:cbqpoffs=3:crqpoffs=-3` gives `cu_qp_delta_enabled_flag=1`, `diff_cu_qp_delta_depth=2`, `pps_cb_qp_offset=3`, `pps_cr_qp_offset=-3` | QP Map overlay; Transform | OVL-002 partial |
| RDOQ | `--rdoq-level`, `--rd`, `--psy-rd` | Encoder-only; no field in the stream | Not applicable | No item |
| Deblocking filter | `--deblock` (offsets only), `--no-deblock` to disable | `no-deblock=1` gives `pps_deblocking_filter_disabled_flag=1`. Note that `deblock=0` does **not** disable the filter (it sets zero offsets; `deblocking_filter_control_present_flag=0`) | Loop Filter (edges colored green/yellow/red); Reconstruction is before deblocking | OVL-020 partial, OVL-019 todo |
| SAO | `--sao`, `--limit-sao`, `--selective-sao`, `--sao-non-deblock` | `sample_adaptive_offset_enabled_flag=0` when off (1 by default) | SAO mode | OVL-021 todo |
| Entropy coding, WPP | CABAC only; `--wpp` | `entropy_coding_sync_enabled_flag=1` by default | `CU`/`TU` tabs | OVL-011 todo |
| Slices | `--slices` (requires `--wpp`) | `slices=4:wpp=1` gives `slice_segment_address = 5, 10, 15` | Main panel (slices as thick red lines) | CODEC-009 partial |
| Tiles | No option in `x265` | `tiles_enabled_flag=0`; another encoder is needed | Not covered | CODEC-018 partial |
| Lossless | `--lossless`, `--cu-lossless` | `transquant_bypass_enabled_flag=1` | Not covered | CODEC-009 partial |
| PCM | No option in `x265` | `pcm_enabled_flag=0` | Not covered | No item |
| Bit depth, chroma format, RExt | `--output-depth`, `--input-csp`, `--profile` | 10-bit: `bit_depth_luma_minus8=2`, `general_profile_idc=2`; 4:4:4: `chroma_format_idc=3`, `general_profile_idc=4` | Options > "HEVC: Enable HEVC extensions"; YUV mode | CODEC-018 partial, DEC-004 partial |
| Interlaced, field coding | `--interlace`, `--field` | `general_interlaced_source_flag=1`, `field_seq_flag=1`, `pic_struct=4` | Not covered | No item |
| SEI: decoded picture hash | `--hash` | `Decoded Picture Hash` SEI with `hash_type` | Header > `SEI` tab | CODEC-019 todo |
| SEI: HDR metadata | `--master-display`, `--max-cll`, `--hdr10` | `display_primaries_x[0]=13250`, `max_content_light_level=1000`, `colour_primaries=9`, `transfer_characteristics=16` | Header > `SEI` tab | CODEC-019 todo, SYN-003 partial |
| HRD, VUI, buffer | `--hrd`, `--vbv-maxrate`, `--vbv-bufsize`, `--vui-hrd-info`, `--vui-timing-info` | `hrd=1:vbv-maxrate=500:vbv-bufsize=1000` gives `nal_hrd_parameters_present_flag=1`, `bit_rate_value_minus1[0]=7811`, `cpb_size_value_minus1[0]=15624` | Buffer view `HRD` sub-mode (mentioned only) | UI-008 partial |
| SCC, SHVC, MCTS | No option in `x265` | Not testable here | Not covered | CODEC-018 partial |

### AVC (H.264)

Modes in the reference guide: Coding Flow, Predictions, Transform, Reconstruction, Loop Filter, YUV, Info Overlays (Heat Map, QP Map, MB Type, MB Reference Indices, PSNR), Simple Motion. Encoder: `x264` (invoked through `ffmpeg -x264-params ...`).

| Tool | Encoder option (x264) | In the stream | Shown in the analyzer | Bitvue status |
|---|---|---|---|---|
| Macroblock partitions | `--partitions` (`p8x8`, `p4x4`, `b8x8`, `i8x8`, `i4x4`) | Partition use is not in the headers (macroblock layer not checked) | Predictions (MB splits in blue) | OVL-018 partial, OVL-015 done |
| Intra prediction | `--partitions i4x4,i8x8`, `--constrained-intra` | `constrained_intra_pred_flag=1` (0 by default) | Predictions (intra modes in green) | OVL-018 partial |
| Inter prediction, MV, skip, direct | `--direct`, `--me`, `--merange`, `--subme`, `--no-fast-pskip` | `direct=temporal:bframes=3` gives `direct_spatial_mv_pred_flag=0` (1 by default) | Predictions (L0 vectors orange); Simple Motion | OVL-003 partial, OVL-025 partial |
| Multiple references | `--ref`, `--no-mixed-refs`, `--b-adapt` | `ref=8` gives `max_num_ref_frames=8` | MB Reference Indices overlay | OVL-025 partial, SYN-006 partial |
| B-pyramid | `--b-pyramid` (`none`, `strict`, `normal`) | `bframes=4:b-pyramid=normal:ref=4` gives `max_num_reorder_frames=2`, `max_dec_frame_buffering=4`; 2 reference B slices (5 non-IDR slices with `nal_ref_idc=2` against 3 P slices; indirect evidence) | Frame types in Stream View | CODEC-003 done |
| 8x8 transform | `--no-8x8dct` | By default `transform_8x8_mode_flag=1`, `profile_idc=100`; with `8x8dct=0` `profile_idc=77` | Transform (splits in yellow) | OVL-018 partial |
| Quantization, QP, AQ | `--crf`, `--qpmin`, `--qpmax`, `--aq-mode`, `--aq-strength`, `--chroma-qp-offset` | `chroma-qp-offset=4` gave `chroma_qp_index_offset=2` (x264 adjusts the value; cause unconfirmed) | Transform (QP shown per MB); QP Map overlay | OVL-002 partial |
| Trellis, deadzone | `--trellis`, `--deadzone-inter`, `--deadzone-intra`, `--nr` | Encoder-only; no field in the stream | Not applicable | No item |
| Quantizer matrices | `--cqm` (`flat`, `jvt`), `--cqmfile` | `cqm=jvt` gives `pic_scaling_matrix_present_flag=1` | `QM` tab | SYN-006 partial |
| Deblocking filter | `--deblock` (offsets only), `--no-deblock` | `no-deblock=1` gives `disable_deblocking_filter_idc=1` (0 by default). `deblock=0` sets offsets to zero and does not disable the filter | Loop Filter | OVL-020 partial, OVL-019 todo |
| CABAC / CAVLC | `--no-cabac` | `cabac=0` gives `entropy_coding_mode_flag=0` (1 by default) | Not covered | OVL-011 todo |
| Slices | `--slices`, `--slice-max-size`, `--slice-max-mbs` | `slices=4` gives `first_mb_in_slice = 0, 80, 160, 220` | Not covered | CODEC-010 partial |
| Weighted prediction | `--weightp`, `--no-weightb` | By default `weighted_pred_flag=1`, `weighted_bipred_idc=2`; with `weightp=0:weightb=0` both are 0. Weight-table use not tested | Not covered | CODEC-010 partial |
| Interlaced, MBAFF, field | `--tff`, `--bff`, `--fake-interlaced`, `--pulldown` | `tff=1` gives `frame_mbs_only_flag=0`, `mb_adaptive_frame_field_flag=1` (MBAFF on a progressive source) | Not covered | No item |
| Bit depth, chroma, High profiles | `--profile`, `--output-depth`, `--output-csp`, `--input-csp` | 10-bit: `bit_depth_luma_minus8=2`, `profile_idc=110`; 4:2:2: `chroma_format_idc=2`, `profile_idc=122` | YUV mode | CODEC-010 partial, DEC-004 partial |
| Lossless (transform bypass) | `--qp 0` with High 4:4:4 Predictive | `-qp 0` with `yuv444p` gives `profile_idc=244`, `qpprime_y_zero_transform_bypass_flag=1` | Loop Filter detail mode (`bypass_flag`, mentioned only) | CODEC-010 partial |
| PCM | No option in `x264` | Not testable here | Transform (PCM macroblocks marked `PCM` in white) | No item |
| FMO, ASO, data partitioning (Baseline) | No option | Not testable here | Not covered | CODEC-023 todo (data partitions) |
| Open GOP, intra refresh | `--open-gop`, `--intra-refresh`, `--scenecut`, `--keyint` | `intra-refresh=1:keyint=6` gives a recovery point SEI (`recovery_frame_cnt=8`). The open-GOP clip was made but its header values were not read | `NAL` tab | CODEC-003 done |
| SEI, HRD, VUI | `--nal-hrd`, `--vbv-maxrate`, `--vbv-bufsize`, `--aud`, `--pic-struct` | `nal-hrd=vbr:vbv-maxrate=500:vbv-bufsize=1000:aud=1:pic-struct=1` gives `nal_hrd_parameters_present_flag=1`, `bit_rate_value_minus1[0]=7811`, `cpb_size_value_minus1[0]=15624`, `pic_struct_present_flag=1` | `SEI` tab | SYN-003 partial, UI-008 partial |

Notes on tools shared by HEVC and AVC: the FFmpeg `hevc` and `h264` decoders decode every tool listed above and expose no per-tool
options; motion vectors can be exported with `-flags2 +export_mvs`.

### VP9

Modes in the reference guide: Coding Flow, Predictions, Transform, Reconstruction, Loop Filter, YUV, Info Overlays (Heat Map, Block Type, Efficiency Map, PSNR), Simple Motion. Syntax Info tabs: MKV, Frame, Probabilities, Counts, Refs, Block, TX, Stats.
Encoder: `libvpx-vp9` through `ffmpeg`. `trace_headers` shows only the uncompressed header, so block-level tools (partitions, transform sizes, interpolation filter) cannot be checked from the stream with it.

| Tool | Encoder option (libvpx-vp9) | In the stream | Shown in the analyzer | Bitvue status |
|---|---|---|---|---|
| Superblocks, block partitions | No option | Block level is not visible in the header trace | Coding Flow (superblock decode order, block partition) | OVL-018 partial, OVL-015 done |
| Intra prediction | No option | Key frames of every clip; mode values not checked | Predictions (intra luma and chroma modes) | OVL-018 partial |
| Inter prediction and motion vectors | No option | P frames in every clip (11 of 12); vector values not checked | Predictions (inter modes in cyan, MV1 orange, MV2 purple) | OVL-003 partial |
| Interpolation filter | No option | Compressed-header field; not visible | Predictions (filter type in yellow) | OVL-018 partial |
| Transform (size, type, skip) | No option | Not checked | Transform (`Skip` in yellow, splits in yellow) | OVL-018 partial, SYN-006 partial |
| Quantization | `-crf` (0 to 63), `-b:v` | `base_q_idx` varies from 13 to 40 within one clip | `Stats` tab (QP min/max/average) | OVL-002 partial |
| Segmentation | `-aq-mode` (0 none, 1 variance, 2 complexity, 3 cyclic, 4 equator360) | `segmentation_enabled` is set with `-aq-mode 1` | Coding Flow (segments in unique colors) | CODEC-022 partial |
| Lossless | `-lossless 1` | `base_q_idx=0` on all 12 frames | Not covered | No item |
| Loop filter | `-sharpness` (no direct level option) | `loop_filter_level` 6 to 16 | Loop Filter (green 16-tap, yellow 8-tap, ...) | OVL-020 partial |
| Probability adaptation | `-frame-parallel`, `-error-resilient` | `-frame-parallel 0` gives `frame_parallel_decoding_mode=0`. Without the option the value is **1**, although the help says the default is `auto` | `Probabilities`, `Counts` tabs | SYN-013 partial, CODEC-022 partial |
| Error resilience | `-error-resilient default` | Header field `error_resilient_mode` | Not covered | No item |
| Tiles | `-tile-columns`, `-tile-rows` (log2) | No effect at 320 pixels wide; at 640x144 `tile_cols_log2=1` | Coding Flow (tile boundaries in thick green) | No item beyond OVL-015 |
| Reference structure (8-slot pool) | `-auto-alt-ref`, `-lag-in-frames`, `-min-gf-interval`, `-enable-tpl` | `refresh_frame_flags`, `ref_frame_idx[0..2]` recorded per frame | `Refs` tab (Last, Golden, AltRef) | CODEC-022 partial |
| Hidden frames, superframes | `-auto-alt-ref 1 -lag-in-frames 12` (2-pass only per the help) | **Not produced**: no hidden frame appeared in 12 frames, even in 2-pass mode with a noisy source | Not covered | CODEC-011 partial |
| `show_existing_frame` | No option | Always 0 in the test clips (not verified) | Not covered | No item |
| Profile, bit depth, chroma | `-pix_fmt`, `-profile:v` | A profile 3 clip (`yuv444p10le`) was generated | Supported-format list in the guide | DEC-004 partial |
| Level, temporal scalability | `-level`, `-ts-parameters` | Not tested | Not covered | No item |
| Containers | `-f webm` (default), `-f ivf` | `matroska,webm` for `.webm` files | `MKV` tab (EBML tree) | CONT-003 partial, CONT-001 done (IVF) |

Decoder note: the `vp9` decoder exports QP data (`-export_side_data venc_params`) but no motion vectors.

### MPEG-2 video

Modes in the reference guide: Predictions, Transform, YUV, QP Map, Simple Motion (no loop-filter mode, because MPEG-2 has no loop filter). Syntax Info tabs: Pictures, Sequence, GOP, Picture, User, Stats.
Encoder: `mpeg2video` through `ffmpeg`. `trace_headers` shows header fields only.

| Tool | Encoder option (mpeg2video) | In the stream | Shown in the analyzer | Bitvue status |
|---|---|---|---|---|
| Intra DC precision | `-intra_dc_precision` (-1 to 3) | Not checked | Predictions (intra modes in green) | OVL-034 todo |
| Inter prediction and motion vectors | `-motion_est`, `-mepc`, `-mepre`, `-mpv_flags mv0` | Vectors visible with `ffprobe -flags2 +export_mvs -show_frames` on B frames | Predictions (inter in cyan, up to two MV pairs; the left pair is the first field) | OVL-034 todo, OVL-026 todo |
| Concealment motion vectors | No option | `concealment_motion_vectors=0` in every clip | Predictions (noted when present) | No item |
| Alternate scan | `-alternate_scan` | `alternate_scan=1` (0 by default) | Transform (blue block boundaries, `Skip` label) | OVL-034 todo |
| Quantization | `-non_linear_quant` (needs `-qmax 28`), `-qmin`, `-qmax` | `q_scale_type=1` (0 by default) | `QP Map` mode | OVL-002 partial, OVL-034 todo |
| Intra VLC table | `-intra_vlc` | `intra_vlc_format=1` | Not covered | No item |
| Picture types I/P/B | `-bf`, `-g`, `-b_strategy`, `-b_sensitivity` | Frame types from `ffprobe -show_frames` | `Pictures` tab | CODEC-013 partial |
| GOP | `-g`, `-flags +cgop` (needs `-sc_threshold 1000000000`) | `closed_gop=1` | `GOP` tab | CODEC-013 partial |
| Interlaced coding | `-flags +ildct+ilme` and a source filter `-vf setfield=tff` or `bff` (`-top` is not an encoder option) | The encoder produces interlaced **frame** pictures only (`picture_structure=3`, `interlaced_frame=1`); field pictures cannot be produced | Predictions (left pair = first field) | UI-029 todo |
| `repeat_first_field` (3:2 pulldown) | No option | Always 0 in the test clips | Not covered | No item |
| Chroma format, profile | `-pix_fmt yuv422p`, `-profile:v 0` | `chroma_format=2` | Supported-format list in the guide | No item |
| Sequence and picture extensions | `-seq_disp_ext`, `-video_format` | Extension headers appear in the trace | `Sequence` tab | CODEC-013 partial |
| Rate control, skipping | `-skip_threshold`, `-lmin`, `-lmax`, `-rc_eq` | Not tested | `Stats` tab (guide text not read) | No item |
| Containers | `-f mpegts` (TS), `-f mpeg` / `-f vob` (PS) | Generated; the PS and VOB files report `mpeg` | Container list in the guide | CONT-004 partial (TS), CONT-005 todo (PS) |

Decoder note: the `mpeg2video` decoder exports both motion vectors and QP data.

### VC-3 (DNxHD / DNxHR)

The reference guide has no VC-3 mode, syntax or selection-info chapter. It lists VC-3 only as a supported elementary stream (File > `Open bitstream as...`) and in a note about extracting packed RGB(A). The table therefore has no "shown in the analyzer" column.
Encoder: `dnxhd` through `ffmpeg`.

| Tool | Encoder option (dnxhd) | In the stream | Bitvue status |
|---|---|---|---|
| Profile, compression ID (DNxHD or DNxHR) | `-profile:v` with `dnxhd`, `dnxhr_lb`, `dnxhr_sq`, `dnxhr_hq`, `dnxhr_hqx`, `dnxhr_444` | The decoder debug log (`ffmpeg -v debug -i clip -f null -`) shows the compression ID (`cid`) | CODEC-016 partial |
| Fixed DNxHD formats | `-b:v` must equal a value in the format table; otherwise `video parameters incompatible with DNxHD` and the valid list is printed | A 1080p 36M clip gave a valid compression ID | CODEC-016 partial |
| Bit depth 8 or 10 | `-pix_fmt` (`yuv422p`, `yuv422p10le`, `yuv444p10le`, `gbrp10le`) | A 10-bit 720p clip reports `4:2:2 10 bits` | DEC-007 todo |
| Chroma 4:2:2 / 4:4:4 | `-profile:v dnxhr_444 -pix_fmt yuv444p10le` | A 4:4:4 clip was generated (about 394 KB) | DEC-007 todo |
| Interlaced coding | `-flags +ildct` and `-vf setfield=tff` | The decoder debug log has an `interlaced` field | No item |
| Quantization bias | `-ibias` | File size and compression ID are identical to the default clip, so the effect is not confirmed | OVL-033 partial |
| Nitris compatibility | `-nitris_compat` | File size and compression ID are identical to the default clip, so the effect is not confirmed | No item |
| Containers | `-f mxf`, `-f mov` | `.mxf` and `.mov` files generated (`AVdn` for DNxHD, `AVdh` for DNxHR in MOV) | CONT-007 todo (MXF), CONT-002 partial (MOV) |

### VVC (H.266)

Modes in the reference guide: Dual Tree, Coding Flow, Predictions (with a detail mode), Transform (detail mode), Reconstruction (detail mode), Inverse Map (detail mode), Loop Filter (detail mode), SAO (detail mode), Adaptive Filter (detail mode), YUV, Info overlays (Heat Map, QP Map, Inter Memory Reads), Simple Motion. The guide's `Reconstruction`, `Inverse Map`, `Heat Map`, `QP Map` and `Simple Motion` chapters are headings without body text.
Encoder: `vvencapp` (vvenc 1.14). Two things differ from the other codecs:

- `vvencapp --fullhelp` lists no coding-tool options. All of them are passed as `--additional Name=Value:Name=Value`. The names below were taken from the encoder library and each one was confirmed by actually encoding (an unknown name is rejected, for example `GPM=1` is an error and the accepted name is `GEO=1`).
- Neither FFmpeg's `trace_headers` nor `vvdecapp` prints tool flags for VVC. The "In the stream" column therefore comes from an SPS/PPS parser written for this research, checked against the encoder (for example `ISP=1` versus `ISP=0` changes only the `isp` flag). It means **the SPS/PPS flag is set, not that a coding unit actually uses the tool**.

| Tool | Encoder option (`--additional`) | In the stream (SPS/PPS flag) | Shown in the analyzer | Bitvue status |
|---|---|---|---|---|
| Dual tree | `DualITree=0/1` | `dual_tree` 0 or 1 | `Dual Tree` mode (luma/chroma switch); selection info (CU tree type) | OVL-029 partial, CODEC-020 partial, QA-006 todo |
| CTU size | `CTUSize=128` / `32` | `ctb` 128 / 32 (default 64) | Coding Flow (CTB grid); selection info | CODEC-012 partial |
| Quadtree and multi-type tree splits | `MaxMTTDepthISliceL`, `MaxMTTDepth`, `MinQTLumaISlice` | Depth 3 gives `mtt_intra_l=3`, `mtt_inter=3` (0 when off) | Predictions (CU splits); selection info | OVL-018 partial, CODEC-024 partial |
| MIP | `MIP=1` | `mip=1` | Prediction detail mode (`mip_flag`) | No per-tool item |
| ISP | `ISP=1` | `isp=1` | Transform (ISP CUs labelled `ISP`) | No per-tool item |
| MRL | `MRL=1` | `mrl=1` | Prediction detail mode (reference lines) | CODEC-020 partial |
| CCLM | `LMChroma=0/1` | `cclm` 0 or 1 (default 1) | Prediction detail mode | CODEC-020 partial, QA-006 todo |
| IBC | `IBC=0/1/2` (2 = automatic) | `ibc` 1 / 0; automatic gave 1 on `testsrc2` | Prediction detail mode (block vectors) | CODEC-020 partial |
| Palette | No option found (`Palette`, `PLT` rejected) | `palette=0` only | Not covered | No per-tool item |
| Transform skip, BDPCM | `TransformSkip=1:BDPCM=2` (`BDPCM=1` alone is an error) | `transform_skip=1`, `bdpcm=1` | Transform (BDPCM CUs marked in white) | No per-tool item |
| Affine, PROF | `Affine=1:AffineType=1:PROF=1` | `affine=1`, `affine6=1`, `prof=1` | Predictions (affine motion vectors) | No per-tool item |
| SbTMVP | `SbTMVP=1` | `sbtmvp=1` | Predictions | No per-tool item |
| GPM (geometric merge) | `GEO=1` | `gpm=1` | Predictions (split line drawn dashed) | No per-tool item |
| MMVD, CIIP | `MMVD=1`, `CIIP=1` | `mmvd=1`, `ciip=1` | Predictions | No per-tool item |
| BCW | `BCW=1` | `bcw=1` | Prediction detail mode | No per-tool item |
| SMVD, AMVR, BDOF, SBT | `SMVD=1`, `AMVR=1`, `BDOF=1`, `SBT=1` | `smvd=1`, `amvr=1`, `bdof=1`, `sbt=1` | Not covered in the guide's VVC chapter | No per-tool item |
| DMVR | `DMVR=0/1` (default 1) | `dmvr=0` when off | Prediction detail mode (motion buffer) | No per-tool item |
| TMVP | `TMVPMode=0/1/2` | `tmvp=0` when off | Not covered | No per-tool item |
| MTS | `MTS=1` | `mts=1`; `MTS=1` to `4` all gave `mts_intra_explicit=1`, `mts_inter_explicit=0` in the `faster` preset | Transform and Transform detail (transform types) | No per-tool item |
| LFNST | `LFNST=1` | `lfnst=1` | Transform detail (LFNST ID) | No per-tool item |
| Joint Cb-Cr coding | `JointCbCr=1` | `jcbcr=1` | Transform detail | No per-tool item |
| Dependent quantization | `DepQuant=1:SignHideFlag=0` (cannot be combined with sign hiding) | `dep_quant=1` | Transform detail | No per-tool item |
| Sign data hiding | `DepQuant=0:SignHideFlag=1` | `sdh=1` | Not covered | No per-tool item |
| Deblocking | `LoopFilterDisable=1`, `LoopFilterBetaOffset_div2`, `LoopFilterTcOffset_div2` | PPS `dbf_disabled_in_pps=1` | Loop Filter (green large, yellow strong, orange weak, red no filter) | OVL-020 partial |
| SAO | `SAO=0/1` | `sao=0` when off (default 1) | `SAO` mode | OVL-021 todo |
| ALF | `ALF=0/1`, `ALFSpeed`, `ALFUnitSize` | `alf` | Adaptive Filter | OVL-029 partial, SYN-010 partial, QA-006 todo |
| CC-ALF | `CCALF=0/1` | `ccalf=0` when off | Adaptive Filter detail | OVL-029 partial |
| LMCS | `LMCS=1` | `lmcs=1` | `Inverse Map` (guide: heading only) | OVL-029 partial, CODEC-020 partial, QA-006 todo |
| Tiles | `--tiles 2x1 --level 4.0` (without the level it fails at 320x240) | PPS `no_pic_partition=0` | Coding Flow (tile boundaries in thick green) | No per-tool item |
| Wavefront (WPP) | `WaveFrontSynchro=1` | `wpp=1` | Coding Flow (substream) | No per-tool item |
| Slices | No option found (`Slices` rejected) | Not verified | Coding Flow (slice boundaries in thick red) | No per-tool item |
| Sub-pictures | Not supported by the encoder library | Not testable | Coding Flow (beige boundaries) | CODEC-020 partial |
| Weighted prediction | No option found (`WeightedPred` rejected) | Not verified | `Ref Lists` tab (`W Y`, `O Y`, ... columns) | SYN-006 partial |
| Scaling list | No option found (`ScalingList` rejected) | `scaling_list=0` only | Not covered | SYN-010 partial |
| Virtual boundaries | Option not found | Not verified | Coding Flow (thin light-blue lines) | No per-tool item |

Analyzer modes that are not coding tools: `Reconstruction` (OVL-019 todo) and `Inter Memory Reads` (OVL-030 todo). Whether `Reconstruction` and `SAO` are shown for VVC is a parity gap, because the guide documents both modes for VVC and AVS3 while Bitvue lists them as todo for every codec.

### AVS3

Modes in the reference guide: Prediction (detail), Transform (detail), Reconstruction (detail), Loop Filter (detail), SAO (detail), ESAO (detail), CCSAO (detail), Adaptive Filter (detail), YUV, Info Overlays (Heat, QpY, CoeffsY, PSNR, SSIM, Statistics). In the guide, the `QpY`, `PSNR` and `SSIM` sections are marked as to-be-written.
**No AVS3 encoder or decoder is installed on the test machine** (FFmpeg has only the raw demuxer `avs3`), so no test clip could be made and there is no "encoder option" or "in the stream" column.

| Tool or screen | Shown in the analyzer | Bitvue status |
|---|---|---|
| Stream structure | Syntax Info tabs `NAL`, `Sequence`, `Picture`, `Ref Lists`, `Block`, `Stats` | CODEC-014 partial, SYN-011 todo |
| CU partition (LCU quadtree) | `Prediction` (LCU splits in blue) | CODEC-031 todo |
| Angular intra, SAWP | `Prediction` (labels) | No per-tool item |
| IPF, IIP | `Prediction` (dark-green labels) | No per-tool item |
| IBC, ISC | `Prediction` (cyan labels) | No per-tool item |
| Inter sub-types (Skip, Inter, Direct, affine) | `Prediction` (L0 vectors orange, L1 purple) | No per-tool item |
| Inter refinement and filters (BGC, IPF, IPC, DMVR, ASR) | `Prediction` (dark-cyan labels) | No per-tool item |
| AMVR | `Prediction` details (AMVR index) | No per-tool item |
| Transform, residual | `Transform` (CU blue, TU yellow, QP at the bottom left) | CODEC-031 todo |
| Reconstruction | `Reconstruction` (picture, tile and block grids) | OVL-019 todo |
| Deblocking | `Loop Filter` (red none, green large, yellow strong, orange weak) | OVL-020 partial |
| SAO | `SAO` and its detail mode | OVL-021 todo |
| ESAO, CCSAO | `ESAO`, `CCSAO` and their detail modes | OVL-031 partial |
| Adaptive filter (ALF) | `Adaptive Filter` | OVL-031 partial |
| Info overlays | `Heat`, `QpY`, `CoeffsY`, `PSNR`, `SSIM`, `Statistics` | OVL-031 partial (`QpY`, `CoeffsY`), OVL-022 partial (`Heat`) |
| Plain picture | `YUV` | OVL-035 partial |

### JPEG XS

Modes in the reference guide: Precinct, Dequant, Transform, MCT, NLT, YUV (each with a detail mode). The guide has no info overlays or heat map for JPEG XS. Syntax Info tabs: NAL, Picture, Markers, Slice, Precinct. Selection info: Profile, Level, Chroma format, Bitdepth, Slice and more.
**No JPEG XS encoder or decoder is installed on the test machine** (FFmpeg has only the pipe demuxer `jpegxs_pipe`), so no test clip could be made.

| Tool or screen | Shown in the analyzer | Bitvue status |
|---|---|---|
| Codestream structure | Syntax Info tabs (picture header, markers, slices, precincts); selection info | CODEC-015 partial |
| Precinct and slice | `Precinct` (blocks of 16x16 pixels, slice boundaries) | OVL-032 partial |
| Dequantization | `Dequant` and its detail mode (type of inverse quantization) | OVL-032 partial |
| Wavelet transform | `Transform` and its detail mode (inverse transform per band) | OVL-032 partial |
| Multiple-component transform (MCT) | `MCT` and its detail mode | OVL-032 partial |
| Non-linear transform (NLT) | `NLT` and its detail mode (output scaling and level shift) | OVL-032 partial |
| Plain picture | `YUV` | OVL-035 partial (whether it covers JPEG XS is not stated) |

## Known issues

Places where the application differs from the reference guide or shows something wrong. Fixes are handled in the issues.

| Screen | Problem | Issue |
|---|---|---|
| Player | In screenshots the decoded picture is missing (`F10 YUV` and `F4` are black, only the overlays show). It appears when WebGPU is turned off, so decoding and drawing work; the picture sits on a WebGPU canvas that screenshot capture does not include. Whether a person sees it on a real screen is not verified | #456 |
| Filmstrip, timeline, statistics | Frames that re-show an earlier frame (2, 5, 8) are labelled `I`; they are actually P | #454 |
| Frame information | For some frames `qp_avg` is the hidden frame's value, not the shown frame's (frames 1 and 6) | #455 |
| `F5 Loop Filter` | Shows a numeric panel instead of drawing filtered edges on the picture (the values are correct) | #457 |
| `HM` | Draws motion-vector magnitude instead of bit usage | #458 |
| `F2 Predictions` | The legend contains an unnamed inter mode (`Inter69`) | #460 |
| `HRD Buffer` | Draws values for streams that have no HRD information | #461 |
| `Frame Sizes` | The QP axis is fixed at 0 to 51, so AV1 QP (up to 255) exceeds it | #462 |
| Overlays on a mode | The overlay label is drawn over the mode label and is hard to read | #451 |
| `EFF` overlay | Draws bits per pixel instead of coding efficiency | #488 |
| Window layout | At the default window size `Stream Tree` and `Syntax Detail` show only one or two rows | #489 |
| Statistics | The bottom `STATS` panel shows `Keyframes: 4` and `I: 4` for a clip with one key frame | #454 |
| AV1 `QP Map` | Not in the guide's AV1 overlays; whether to add it as an extension is undecided | #446, #453 |
