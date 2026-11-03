# Bitvue 사용 설명서

Bitvue는 영상 비트스트림(압축된 영상 파일)을 열어서 **프레임이 어떻게 압축되어 있는지** 보여 주는 분석 프로그램이다.
화면에 보이는 영상이 아니라, 그 영상을 만든 부호화 정보(블록 분할, 예측 방식, 변환, 필터, 참조 관계, QP 등)를 본다.

이 문서는 사용법과 "정상이면 무엇이 보이는가"를 설명한다. 기능의 개발 상태는 `docs/specs/features.yaml`이 기준이고,
앱과 어긋나는 부분은 맨 끝 [알려진 문제](#알려진-문제)에 이슈 번호로 모아 두었다.

- 화면 구성과 용어의 기준은 VQ Analyzer 사용자 가이드(v7.10.0)를 따른다.
- 지원 범위: 데스크톱 앱은 **AV1(IVF 파일)** 을 분석한다. 다른 코덱은 아직 앱 화면으로 열지 못한다([코덱별 기능](#코덱별-기능) 참고).
- 이 설명서의 화면은 예제 클립 `test_data/av1_aomenc_testsrc2.ivf`(320x240, 12프레임)로 확인했다.

## 1. 시작하기

1. 앱을 실행하고 메뉴의 `File`에서 비트스트림 열기(`Open bitstream`)로 `.ivf` 파일을 연다.
2. 파일을 읽는 동안 인덱싱(프레임 목록 만들기)이 끝나면 화면 전체가 채워진다.
3. 같은 형식의 두 번째 스트림과 비교하려면 `Open dependent bitstream…`으로 연다.

예제 클립으로 따라 해 볼 때 정상이면 이렇게 보인다.

| 보이는 곳 | 정상 값 |
|---|---|
| 필름스트립 제목 | `FILMSTRIP (12 FRAMES)` |
| 타임라인 오른쪽 위 | `1 / 12` |
| 아래 `INFO` 패널 | `Frames: 12`, `Current Frame: 1`, 파일 경로 |
| 맨 아래 상태줄 | 파일 경로와 `12 frames` |

## 2. 화면 구성

```
┌──────────────────────────────────────────────┐
│ TIMELINE   프레임별 색 칸                    │ ← 전체 흐름
│ FILMSTRIP  프레임 썸네일 (또는 크기/참조 그래프) │ ← 프레임 고르기
├───────────┬──────────────────────────────────┤
│ Stream    │ 도구줄: 이동 / 재생 / Mode / Overlay │
│ Stream    │                                  │
│ Tree      │      플레이어 (모드별 화면)        │
│ Syntax    │                                  │
│ Detail    │                                  │
├───────────┴──────┬──────────┬────────────────┤
│ INFO             │ DETAILS  │ STATS │ DIAGNOSTICS │ ← 프레임 정보
├──────────────────┴──────────┴───────┴────────┤
│ 상태줄: 파일 경로 · 프레임 수                  │
└──────────────────────────────────────────────┘
```

| 영역 | 역할 |
|---|---|
| `TIMELINE` | 프레임마다 색 칸 하나. 현재 프레임이 강조되고 오른쪽 위에 `현재 / 전체`가 나온다. 빨강은 I(키) 프레임, 초록은 P 프레임 |
| `FILMSTRIP` | 프레임 썸네일 줄. 현재 프레임은 흰 테두리. 오른쪽 위 드롭다운으로 보기를 바꾼다([4장](#4-필름스트립-보기)) |
| 도구줄 | 프레임 이동 버튼, 프레임 번호 입력(`n / 전체`), 재생, 속도(`1x`), 코덱 배지(`AV1`), `Mode`, `Overlay`, 전체화면, 확대/축소 |
| 플레이어 | 선택한 `Mode`의 화면. 아래쪽에 `Frame n / 전체`, `Zoom`, 현재 모드 이름, `Paused`가 나온다 |
| 왼쪽 | `Stream Tree`(스트림 구조), `Syntax Detail`(프레임 하나의 신택스) |
| 아래 4칸 | `INFO`, `DETAILS`, `STATS`, `DIAGNOSTICS` |

## 3. 프레임 이동

| 방법 | 동작 |
|---|---|
| 키보드 `←` `→` | 이전/다음 프레임 |
| 도구줄의 이동 버튼 | 처음, 이전, 다음, 마지막 프레임 |
| 프레임 번호 입력 | 번호를 직접 입력 |
| 재생 버튼과 속도(`1x`) | 도구줄에 있다(재생 동작은 [아직 확인되지 않은 기능](#9-아직-확인되지-않은-기능)) |

현재 프레임이 P 프레임이면 필름스트립에 **참조 화살표**가 그려진다. 현재 프레임에서 참조하는 프레임으로 선이 이어지고
`REF0`, `REF1`… 이름표가 붙는다(예제 클립 프레임 3: `REF0`~`REF5`가 프레임 0을, `REF6`이 프레임 1을 가리킴).
썸네일 위에 마우스를 올리면 번호, 타입, 크기(`Size`), `PTS`, `Temporal`, `References`가 툴팁으로 나온다.

## 4. 필름스트립 보기

필름스트립 오른쪽 위 드롭다운(`Thumbnails`)에서 고른다.

| 보기 | 보여 주는 것 | 읽는 법 |
|---|---|---|
| `Thumbnails` | 프레임 썸네일 | 빨간 테두리 I, 초록 테두리 P. `D:`, `C:` 숫자는 표시/코딩 순서 |
| `Frame Sizes` | 프레임별 크기 막대 | 막대가 높을수록 비트가 많이 든 프레임. 왼쪽 축은 크기(`BITRATE`), 오른쪽 축은 QP. 오른쪽 위 팝업에서 `Bitrate Bar`, `Bitrate Curve`, `Avg Size`, `Min Size`, `Max Size`, `Moving Avg`, `Block Min QP`, `Block Max QP`를 켜고 끈다 |
| `B-Pyramid` | 참조 계층 | 한 줄에 프레임 점이 늘어선다. 빨강 I, 초록 P. `Show All Refs`로 모든 참조 표시 |
| `HRD Buffer` | 디코더 버퍼 채움 | 스트림에 HRD 정보가 있을 때만 의미가 있다 |
| `Enhanced` | GOP/장면 이동 도구 | `GOP` 이전/다음, `SCENE` 이전/다음 버튼과 장면 전환 표시 |

## 5. 플레이어 모드 (`Mode`)

`Mode` 드롭다운이나 `F1`~`F10` 키로 바꾼다. 모드마다 "비트스트림의 어떤 부분"을 영상 위에 그려 준다.
**그 도구가 스트림에서 꺼져 있으면** 해당 모드는 화면 가운데에 `…: disabled`라고만 나온다. 고장이 아니다.

| 키 | 모드 | 보여 주는 것 | 읽는 법 |
|---|---|---|---|
| `F1` | `Coding Flow` | 블록 분할(파티션) 경계 | 선이 촘촘한 곳이 잘게 쪼개진 곳(복잡한 영역). 왼쪽 위에 블록 수(`Blocks`)와 슈퍼블록 크기(`SB`) |
| `F2` | `Predictions` | 블록마다 예측 방식 | 인트라 프레임은 방향 모드별 색(`D45`, `SmoothV`, `Paeth`, `CFL` 등), 인터 프레임은 인터 모드별 색(`NewMV`, `NearMV` 등). 아래 범례 참고 |
| `F3` | `Transform` | 변환 블록 크기 | 크기별 색 격자와 개수(예: `8x8 (78)`, `64x64 (88)`). 왼쪽 위에 `Blocks`, `Types` |
| `F4` | `Reconstruction` | 루프 필터 전 복원 영상 | 필터가 적용되기 전의 화면 |
| `F5` | `Loop Filter` | 디블로킹 필터 | 필터가 걸린 경계 수와 필터 강도, 루프 필터 파라미터(`Level`, `Sharpness`, `Ref/Mode Deltas`) |
| `F6` | `CDEF Filter` | CDEF 필터 | 블록 격자와 `CDEF Y-Primary`, `Y-Secondary`, `Damping`. 필터 방향은 화살표 |
| `F7` | `SuperRes Filter` | 슈퍼 해상도 | 슈퍼 해상도를 쓰는 스트림에서만 의미가 있다 |
| `F8` | `Loop Restoration` | 루프 복원 필터 | 필터 종류와 블록 크기. 쓰지 않으면 `disabled` |
| `F9` | `Film Grain Pixels` | 필름 그레인 합성 | 필름 그레인이 있는 스트림에서만 의미가 있다. 아니면 `disabled` |
| `F10` | `YUV` | 부호화 정보 없이 디코딩된 영상만 | 영상 그 자체 |

예제 클립 프레임 0(키 프레임)에서 `F6`, `F7`, `F8`, `F9`이 `disabled`로 나오는 것은 정상이다. 이 프레임은 intrabc(같은 프레임 안에서 블록을
복사하는 도구)를 쓰므로 AV1 규칙상 루프 필터 계열이 헤더에서 생략되고, 슈퍼 해상도와 필름 그레인은 스트림에서 꺼져 있다.
프레임 3(P)에서는 `F5`의 `Level`이 `1 / 2`(Y 세로/가로), `1 / 8`(U / V), `Sharpness 0`, `F6`의 `Y-Primary 0`, `Y-Secondary 1`, `Damping 4`가 나온다.

## 6. 정보 오버레이 (`Overlay`)

모드 위에 한 겹 더 얹는 정보 지도. 도구줄의 `Overlay` 버튼으로 켜고 끈다. `Ctrl+F1`~`F6`은 목록 순서대로 켜고 끈다.

| 버튼 | 이름 | 보여 주는 것 |
|---|---|---|
| `BT` | `Block Type` | 블록 종류별 색: `INTRA`, `INTER`, `SKIP`, `INTRA_BC`, `COMPOUND` (범례가 왼쪽 위에 나옴) |
| `HM` | `Heat Map` | 블록별 비트 사용량 지도(밝을수록 비트를 많이 쓴 곳). 지금 앱은 모션벡터 크기를 그린다([알려진 문제](#알려진-문제)) |
| `EFF` | `Efficiency` | 부호화 효율 지도(밝을수록 효율적) |
| `PSNR` | `PSNR` | 블록별 PSNR. 기준(참조) YUV 파일이 있어야 한다 |

## 7. 왼쪽 패널

`Stream Tree`는 스트림의 프레임 목록을 트리로 보여 주고 `Filter` 체크박스가 있다. 그 아래 `Syntax` 드롭다운과 `Syntax Detail`에서 현재 프레임의 정보를 본다.

| 탭 | 내용 |
|---|---|
| `FRAME` | `Frame:`(번호), `Type:`(I/P) 와 OBU 트리(`obu_0` 등). 트리를 펼치면 신택스 요소를 볼 수 있다 |
| `REFS` | 참조 프레임 정보. 예제 클립 프레임 0에서는 "데이터 없음" 아이콘만 나온다 |
| `STATS` | 현재 프레임 통계(`Current Frame`, `Index`…) |
| `SEARCH` | 프레임 타입, 번호, PTS로 프레임 찾기 (`Search by frame type, number, or PTS...`) |

## 8. 아래 패널

| 패널 | 내용 | 정상 예 (예제 클립 프레임 3) |
|---|---|---|
| `INFO` | 파일 경로, 전체 프레임 수, 현재 프레임, 타입 | `Frames: 12`, `Current Frame: 4`, `Frame: P` |
| `DETAILS` | `Temporal Layer`, `Display Order`, `Coding Order`, `References` | `0`, `3`, `3`, `0, 0, 0, 0, 0, 0, 1` |
| `STATS` | 전체 `Frame Types` 개수 막대 | I와 P 개수 |
| `DIAGNOSTICS` | 스트림에서 찾은 오류/경고/정보 개수 (`All`, 오류, 경고, 정보) | 문제가 없으면 모두 `0`과 체크 표시 |

## 9. 아직 확인되지 않은 기능

가이드에는 있지만 이 앱에서 아직 확인하지 못한 기능이다. 이슈에서 하나씩 확인하고, 확인되면 위 장으로 옮긴다.

| 기능 | 이슈 |
|---|---|
| 메뉴 항목 전체(File, Mode, YUVDiff, Options, View, Help) | #463 |
| 전체 스트림 분석 버튼, Metric view(PSNR/SSIM) | #464, #465 |
| 썸네일 클릭(프레임 이동)/우클릭/번호 점프, 프레임 크기 그래프의 상호작용, B-Pyramid 상호작용 | #466, #467, #468 |
| 플레이어 하단 버튼(Pic, Info, Y/U/V 등)과 마우스 조작(선택, 줌, 이동) | #469, #470 |
| 모드의 상세 보기(Details), 코딩 플로우의 디코드 번호·타일·세그먼트 색 | #471, #472 |
| 인터 프레임의 MV 화살표와 잔차 영상, `EFF`와 `PSNR` 오버레이 | #473, #474, #475 |
| `Syntax Detail`의 OBU/헤더/블록/참조/통계 탭 내용 | #476, #477, #478, #479, #480 |
| 선택 정보(Selection Info), 원시 바이트와 16진 보기, 상태/오류 패널 | #481, #482, #483 |
| 두 스트림 비교(Dual view), Debug YUV, 원시 YUV 열기 | #484, #485, #486 |
| 루프 복원·슈퍼 해상도·필름 그레인이 켜진 스트림의 모드 화면 | #487 |

## 코덱별 기능

코덱마다 지원하는 부호화 도구가 다르다. 같은 모드라도 스트림이 그 도구를 쓰고 있어야 의미 있는 화면이 나온다.
아래는 도구마다 인코더에서 켜는 옵션, 스트림에서 켜졌는지 확인하는 방법, 분석기에서 보이는 곳이다.

| 항목 | 출처 |
|---|---|
| 인코더 옵션 | 이 머신의 `aomenc --help` 출력에서 그대로 옮김 |
| 켜짐 확인 | `ffmpeg -v trace -i clip.ivf -c copy -bsf:v trace_headers -f null -`의 헤더 필드, libdav1d 로그(`size … upscaled … bitdepth … tiles …`) |
| 분석기에서 보이는 곳 | VQ Analyzer 가이드의 모드/탭 이름 |
| 지원 상태 | `docs/specs/features.yaml`의 항목 번호. 이 표가 상태를 새로 정하지 않는다 |

### AV1

| 도구 | 인코더 옵션 (aomenc) | 스트림에서 확인 (실제 클립으로 확인) | 분석기에서 보이는 곳 | 지원 상태 |
|---|---|---|---|---|
| 슈퍼블록 64/128 | `--sb-size` | `use_128x128_superblock=0` (`--sb-size=64`) | Coding Flow, 선택 정보 | CODEC-021 done |
| 파티션 종류 | `--enable-rect-partitions`, `--enable-ab-partitions`, `--enable-1to4-partitions` | 미확인 | Coding Flow, Predictions | 항목 없음 |
| 타일 | `--tile-columns`, `--tile-rows`(log2), `--num-tile-groups` | libdav1d `tiles 2x2` (기본은 `1x1`) | Coding Flow(타일 경계), 선택 정보 | CODEC-021 done |
| 세그멘테이션 | `--aq-mode` | `segmentation_enabled=1` (`--aq-mode=1`) | Coding Flow(세그먼트별 색), 선택 정보 | CODEC-025 done |
| 블록별 delta QP | `--deltaq-mode` | 예제 클립에서 `delta_q_present=1` | 프레임 크기 그래프의 QP 축 | OVL-042 done |
| 양자화 행렬 | `--enable-qm`, `--qm-min`, `--qm-max` | `using_qmatrix=1` | 프레임 헤더 신택스 | CODEC-025 done |
| 루프 필터 | `--loopfilter-control`, `--sharpness` | `loop_filter_level[0..3]`, `loop_filter_sharpness` | `F5 Loop Filter` | OVL-017 done |
| CDEF | `--enable-cdef` | `enable_cdef=0`인 클립과 기본 클립의 `cdef_*` 비교 | `F6 CDEF Filter` | OVL-007 partial |
| 루프 복원 | `--enable-restoration` | 잡음 영상 + `--cq-level=50`에서 `lr_type`이 0이 아님 (합성 영상은 0) | `F8 Loop Restoration` | OVL-008 partial |
| 슈퍼 해상도 | `--superres-mode`, `--superres-denominator`, `--superres-qthresh` | `use_superres=1`, `coded_denom=3` | `F7 SuperRes Filter` | OVL-010 done |
| 필름 그레인 | `--film-grain-test`, `--film-grain-table`, `--denoise-noise-level` | `apply_grain=1`, `num_y_points=14` | `F9 Film Grain Pixels` | OVL-009 done |
| 프레임 크기 변경 | `--resize-mode`, `--resize-denominator` | `frame_size_override_flag=1`, `frame_width_minus_1=212` | 미확인 | 항목 없음 |
| 화면 콘텐츠(팔레트, IntraBC) | `--tune-content=screen`, `--enable-palette`, `--enable-intrabc` | `allow_screen_content_tools=1`. 예제 클립은 intrabc 사용(`Block Type`의 `INTRA_BC`) | Predictions, Block Type | CODEC-021 done |
| 인트라 도구 | `--enable-filter-intra`, `--enable-smooth-intra`, `--enable-paeth-intra`, `--enable-cfl-intra`, `--enable-angle-delta`, `--enable-intra-edge-filter` | 예제 클립 F2 범례에 `D45`, `SmoothV`, `Paeth`, `CFL` | `F2 Predictions` | OVL-006 done |
| 인터 도구 | `--enable-obmc`, `--enable-warped-motion`, `--enable-global-motion`, `--enable-dual-filter`, `--enable-masked-comp`, `--enable-interintra-comp`, `--enable-dist-wtd-comp`, `--enable-onesided-comp` | 미확인 | `F2 Predictions` | OVL-041 done, CODEC-029 todo, QA-019 todo |
| 변환 | `--enable-tx64`, `--enable-rect-tx`, `--enable-flip-idtx`, `--reduced-tx-type-set` | 미확인 | `F3 Transform` | OVL-005 partial |
| 참조 구조 | `--lag-in-frames`, `--auto-alt-ref`, `--enable-fwd-kf`, `--min-gf-interval`, `--max-gf-interval`, `--max-reference-frames`, `--enable-order-hint`, `--enable-ref-frame-mvs` | 예제 클립: 숨은 프레임 3개와 이전 프레임 재표시(`show_existing_frame`) | 필름스트립 참조 화살표, `REFS` 탭 | CODEC-007 done, SYN-004 partial |
| 오류 복원, 병렬 | `--error-resilient`, `--frame-parallel` | `error_resilient_mode=1` | 미확인 | 항목 없음 |
| S-프레임 | `--sframe-dist`, `--sframe-mode` | 미확인 | 미확인 | 항목 없음 |
| 비트 심도, 크로마 | `--bit-depth`, `--input-bit-depth`, `--profile`, `--monochrome` | `high_bitdepth=1`(10비트), `mono_chrome=1`, `seq_profile=1`+`yuv444p`(4:4:4) | 선택 정보 | CODEC-008 done |
| 타이밍, 디코더 모델 | `--timing-info` | 기본은 `timing_info_present_flag=0` | `HRD Buffer` 보기 | [알려진 문제](#알려진-문제) |
| 큰 타일, Annex B | `--large-scale-tile`, `--annexb` | 미확인 | (가이드: AV1은 IVF/WebM만 입력) | 앱은 IVF만 |

**디코더로 따로 확인하는 방법**: 아래 명령은 분석기 화면이 맞는지 비교할 때 기준이 된다.

| 용도 | 명령 |
|---|---|
| 루프 필터 전 복원 영상 | `dav1d --inloopfilters none` (`nodeblock`, `nocdef`, `norestoration`으로 단계별 제외) |
| 필름 그레인 전후 | `dav1d --filmgrain 0` / `--filmgrain 1` |
| 스케일러블 스트림 | `dav1d --oppoint N --alllayers 0/1` |
| 프레임별 헤더 값 | `ffmpeg -v trace -i clip.ivf -c copy -bsf:v trace_headers -f null -` |

**기능별 테스트 클립 만들기** (저장소에 넣지 않고 /tmp에 만든다). 원본은 생성 영상이다:
`ffmpeg -f lavfi -i "testsrc2=size=320x240:rate=30,format=yuv420p" -frames:v 12 src.y4m`.
공통 명령은 `aomenc --limit=12 --cpu-used=4 --threads=1 --end-usage=q --cq-level=30 -o out.ivf src.y4m` 이고, 아래 옵션을 덧붙인다.

| 기능 | 덧붙일 옵션 | 확인된 결과 |
|---|---|---|
| 타일 2x2 | `--sb-size=64 --tile-columns=1 --tile-rows=1` | libdav1d `tiles 2x2` |
| 슈퍼 해상도 | `--superres-mode=1 --superres-denominator=12` | `use_superres=1` |
| 필름 그레인 | `--film-grain-test=1` | `apply_grain=1` |
| 세그멘테이션 | `--aq-mode=1` | `segmentation_enabled=1` |
| 10비트 | `--bit-depth=10 --input-bit-depth=8` | `high_bitdepth=1` |
| 모노크롬 | `--monochrome` | `mono_chrome=1` |
| 양자화 행렬 | `--enable-qm=1` | `using_qmatrix=1` |
| 오류 복원 | `--error-resilient=1` | `error_resilient_mode=1` |
| 크기 변경 | `--resize-mode=1 --resize-denominator=12` | `frame_size_override_flag=1` |
| CDEF·복원 끔 | `--enable-restoration=0 --enable-cdef=0` | `enable_cdef=0`, `enable_restoration=0` |
| 루프 복원 켬 | 원본에 `noise=alls=25:allf=t` 추가, `--cpu-used=1 --cq-level=50 --enable-restoration=1 --lag-in-frames=0 --auto-alt-ref=0` | `lr_type`이 0이 아님 |
| 4:4:4 | 원본을 `format=yuv444p`로, `--profile=1` | `seq_profile=1` |

<!-- 다른 코덱(HEVC, AVC, VP9, MPEG-2, VC-3, VVC, AVS3, JPEG XS) 절은 조사가 끝나는 대로 여기에 추가 -->

## 알려진 문제

앱과 가이드가 다르거나 앱이 잘못 보여 주는 것. 고치는 것은 이슈에서 한다.

| 화면 | 문제 | 이슈 |
|---|---|---|
| 플레이어 | 디코딩된 영상이 그려지지 않는다(`F10 YUV`, `F4`가 검은 화면). 원인은 아직 모른다 | #456 |
| 필름스트립, 타임라인, 통계 | 이전 프레임을 다시 보여 주는 프레임(2, 5, 8번)이 `I`로 표시된다. 실제는 P | #454 |
| 프레임 정보 | 일부 프레임의 `qp_avg`가 보이는 프레임이 아니라 숨은 프레임 값이다(1번, 6번) | #455 |
| `F5 Loop Filter` | 필터가 걸린 경계를 영상 위에 그리지 않고 숫자 패널만 보여 준다(값은 맞다) | #457 |
| `HM` | 비트 사용량이 아니라 모션벡터 크기를 그린다 | #458 |
| `F2 Predictions` | 범례에 이름 없는 인터 모드(`Inter69`)가 나온다 | #460 |
| `HRD Buffer` | HRD 정보가 없는 스트림에서도 값을 그린다 | #461 |
| `Frame Sizes` | QP 축이 0~51로 고정돼 AV1 QP(최대 255)가 축을 넘는다 | #462 |
| 모드 위의 오버레이 | 오버레이 라벨이 모드 라벨 위에 겹쳐 읽기 어렵다 | #451 |
| AV1 `QP Map` | 가이드의 AV1 오버레이에는 없다. 확장으로 넣을지 결정 대기 | #446, #453 |
