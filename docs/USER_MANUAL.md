# Bitvue 사용자 매뉴얼 (AV1 데스크톱 앱)

각 패널이 **정상일 때 무엇을 보여야 하는가**의 기준과, 실제 앱에서 확인한 결과를 한 곳에 둔다. 상태(done/partial)의 기준은
`docs/specs/features.yaml`이고, 이 문서는 "정상 동작이 어떤 모습인가"와 "확인했는가"만 적는다.

| 항목 | 내용 |
|---|---|
| 기준 | VQ Analyzer User Guide v7.10.0 (https://cdn.vicuesoft.com/vqAnalyzer/docs/VQAnalyzerUserGuide.html), 2026-11-04 열람. 의도와 동작을 맞추고 픽셀 디자인은 맞추지 않는다(`UX_PARITY_MATRIX.md` §0) |
| 범위 | AV1 (IVF). 다른 코덱은 아직 쓰지 않았다 |
| 확인 방법 | 데스크톱 앱을 직접 띄워 캡처 (`.claude/skills/run-desktop/SKILL.md`). 같은 조건 2~3회, 결과가 같은 것만 채택(#447) |
| 확인 클립 | `test_data/av1_aomenc_testsrc2.ivf` 프레임 0 (아래 §0 기준값) |
| 표기 | **확인**: 앱에서 직접 봄 · **미확인**: 아직 안 봄 · **차이**: 기준과 다름(이슈 번호) · 가이드 용어는 원문 그대로 |

## 0. 확인 클립의 기준값

앱이 보여 줘야 하는 값. 출처는 FFmpeg `-bsf:v trace_headers`와 `ffprobe -c:v libdav1d`(둘 다 이 저장소 코드와 독립).

| 항목 | 값 |
|---|---|
| 크기 / 단위 수 | 320x240, IVF 패킷 12개 |
| 화면 타입 (패킷 순서) | 0번 Key, 나머지 11개 Inter (2, 5, 8번 패킷은 `show_existing_frame`: 이전에 디코딩한 Inter 프레임을 다시 보여 줌) |
| 보이는 프레임의 `base_q_idx` | 14, 85, (반복), 85, 86, (반복), 88, 89, (반복), 90, 91, 92 |
| 프레임 0 헤더 | `allow_screen_content_tools=1`, `delta_q_present=1`(블록별 QP 13~17), `base_q_idx=14`, loop filter/CDEF/restoration 필드 없음(intrabc 사용으로 추정, Block Type 오버레이에 `INTRA_BC` 영역이 보임) |
| 시퀀스 | `enable_cdef=1`, `enable_restoration=1`, `enable_superres=0`, `film_grain_params_present=0` |

## 1. 화면 구성

| 영역 | 기준(가이드) | 앱 | 상태 |
|---|---|---|---|
| 메인 메뉴 | File / Mode / YUVDiff / Options / View / Help, F1–F12로 모드 선택 | 네이티브 메뉴는 단축키를 두지 않고 렌더러가 키를 처리 | 미확인 (메뉴 항목별 비교 전) |
| Stream View | 상단 가로 스크롤 썸네일(현재 픽처 빨간 테두리), Bars/Buffer/Hierarchy/Metric 보기 | 상단 TIMELINE(프레임별 색 칸, `1 / 12`) + FILMSTRIP(`12 FRAMES`, `Frame 0`, `Thumbnails` 드롭다운) | 확인(썸네일). 나머지 보기 미확인 |
| 메인 패널 | 선택한 픽처에 주석을 얹음. 좌상단 모드 이름, 우상단 배율. 하단 버튼 Export/FS/+/-/R/YUV·Y·U·V/Pic/Info/Details/H·D, 프레임 이동 | 툴바: 이동 버튼, 프레임 번호 `n / 12`, 재생, 속도 `1x`, 코덱 `AV1`, `Mode:` 드롭다운, `Overlay:` 버튼, FS, 확대/축소, `Zoom: 100%`, 하단 `Paused` | 확인(툴바). `Pic`/`Info`/`Y·U·V`/`R`/`H·D` 버튼은 미확인. **영상이 안 그려짐 → #456** |
| 좌측 | Syntax Info(IVF/OBU/Header/Block/Refs/Stats), Selection Info, Unit Info | `Stream` 탭: STREAM TREE(Filter), `Syntax` 드롭다운, SYNTAX DETAIL 탭 FRAME/REFS/STATS/SEARCH | 확인(구성). 내용 미확인 |
| 하단 | Unit Info/HEX View, Status(Status/Errors/상태줄) | INFO(File/Frames/Current Frame/Frame), DETAILS(Temporal Layer/Display Order/Coding Order/References), STATS(Frame Types), DIAGNOSTICS(All/오류/경고/정보 개수, 체크 표시), 상태줄(파일 경로, `12 frames`) | 확인(구성) |

## 2. Stream View (썸네일/타임라인)

| 항목 | 기준 | 앱에서 확인 | 상태 |
|---|---|---|---|
| 프레임 타입 색 | 빨강 I, 파랑 P, 초록 B | 테두리가 빨강(I)/초록(P). 파랑 없음 | 차이(의도는 같음: 색 규칙만 다름) |
| 프레임 타입 값 | 화면 타입 | 0, 2, 5, 8번이 빨강(I). 2, 5, 8번은 실제 Inter | **차이 #454** |
| 레이블 | 디코드/표시 순서 번호, 타입, POC, 레이어 | 썸네일에 `D:n`, `C:n` 배지와 타입 글자 | 확인(구성) |
| 현재 픽처 | 빨간 테두리 | 현재 프레임이 흰 테두리 | 확인(표시 방식이 다름) |
| 클릭 | 픽처를 디코딩하고 현재로 만듦 | 썸네일 클릭 시 타임라인은 바뀌는데 플레이어는 이전 프레임에 머무는 경우가 있음 | 미확정 (#447에 기록) |
| Run full stream analysis | 파란 둥근 버튼. 통계, 평균 블록 QP, 프레임 오류를 채움 | 캡처 화면에서는 보이지 않음(메뉴와 다른 창은 안 봄) | 미확인 |
| Buffer view | 프레임 크기 막대 + 평균 QP(오른쪽 축) | | 미확인 |
| Hierarchy / Metric view | B-피라미드 / PSNR·SSIM | | 미확인 |

## 3. 모드 (메인 패널, AV1)

| 키 | 모드 | 기준: 화면에 나와야 하는 것 | 앱에서 확인 (프레임 0) | 상태 |
|---|---|---|---|---|
| F1 | Coding Flow | 슈퍼블록 디코드 순서(노랑), 파티션 경계(파랑), 타일 경계(굵은 초록), 세그먼트별 색. 상세 모드 없음 | 파티션 경계, `Blocks: 311`, `SB: 128px`. 노란 번호와 타일선은 보이지 않음. 영상 없음 | 부분 확인. 영상 **#456** |
| F2 | Predictions | 블록 분할, 인트라/인터 모드, MV(1/8 pel, L/G/A), 필터 종류 | 블록 격자를 인트라 모드별 색으로 칠함(범례 D45/SmoothV/Paeth/CFL), `Blocks: 300`, `Modes: 7`. MV 없음(키 프레임). 영상 없음 | 부분 확인. 클릭/상세 미확인 |
| F3 | Transform | 블록·변환 경계, "Skip", `Pic`으로 잔차 영상 | 변환 크기별 색 격자(범례 8x8 (78), 16x16 (27), 32x32 (78), 64x64 (88)), `Types: 5`. Skip·잔차 없음 | 부분 확인 |
| F4 | Reconstruction | 루프 필터 전 복원 샘플, 블록 경계, "Skip" | **검은 화면** | **차이 #456** |
| F5 | Loop Filter | 처리된 경계를 1샘플 두께로: 초록 16-tap, 노랑 8-tap, 주황 4-tap, 빨강 필터링 안 됨 | 영상 위가 아니라 숫자 패널 "Deblocking Filter Analysis"(Strong BS 2 / Weak BS 1 등 HEVC 용어). 값은 0(이 프레임은 필터 없음) | **차이 #457** |
| F6 | CDEF Filter | 블록별 level/strength/방향 | `CDEF: disabled` | 이 프레임은 필터 생략이라 맞음(추정). 인터 프레임 미확인 |
| F7 | SuperRes Filter | superres가 켜진 경우에만. 배율 | `SuperRes: disabled` | 맞음 (`enable_superres=0`) |
| F8 | Loop Restoration | 블록별 필터 종류와 크기 | `Loop Restoration: disabled` | 이 프레임은 필터 생략이라 맞음(추정). 인터 프레임 미확인 |
| F9 | Film Grain Pixels | film grain이 켜진 경우에만 | `Film Grain: disabled` | 맞음 (`film_grain_params_present=0`) |
| F10 | YUV | 오버레이 없는 최종 디코딩 화면 | **검은 화면** (로그에는 YUV 변환 성공) | **차이 #456** |

## 4. 정보 오버레이 (AV1)

기준의 AV1 오버레이는 4개이고 QP Map은 없다(QP Map은 AVC, MPEG-2 등에만 있음). Ctrl+F1~F6은 오버레이 목록 순서.

| 오버레이 | 기준 | 앱에서 확인 | 상태 |
|---|---|---|---|
| HM (Heat Map) | 비트 분포, 밝을수록 픽셀당 비트가 많음. CU 쿼드트리 경계(파랑), 확대 시 CU 비트 크기, `Depth` +/-, 그라디언트 편집 | 모션벡터 크기 히트맵을 재사용. 키 프레임에서 "No motion vectors (Intra frame)" | **차이 #458** |
| BT (Block Type) | intra 빨강 / inter skip 노랑 / inter 파랑 | 범례 INTRA, INTER, SKIP, INTRA_BC, COMPOUND. 프레임 0은 대부분 INTRA, 일부 INTRA_BC(보라). 색이 기준과 다름 | 확인. 색 규칙 차이(의도는 같음) |
| EFF (Efficiency Map) | 산술 부호화 효율(bool/bit), 밝을수록 효율적 | | 미확인 |
| PSNR | 블록별 PSNR, 참조 YUV 필요, 정확히 같으면 `e` | | 미확인 |
| (QP Map) | AV1에는 없음 | `main`의 AV1 오버레이에도 없음. PR #453(draft)이 확장으로 추가 | 결정 대기 (#446) |
| 오버레이 라벨 | | 코딩 플로우 모드 위에서 오버레이 라벨이 모드 라벨과 겹침 | 차이 #451 |

## 5. 좌측 패널 / 하단 패널

| 패널 | 기준 | 앱에서 확인 | 상태 |
|---|---|---|---|
| Syntax Info: OBU | 단위 목록(`Pos`, `Unit Type`, `LId`, `TID`, `Size`), 접기/펼치기로 신택스 값과 비트 수 | SYNTAX DETAIL > FRAME에 `Frame: 0`, `Type: I`, `obu_0` 트리 | 미확인 (내용 비교 전) |
| Syntax Info: Header(Frame) | 프레임 헤더의 모든 신택스, 헤더 크기 | | 미확인 |
| Syntax Info: Refs | 8칸 참조 풀(Last/Golden/AltRef, refresh_frame_flags), 세그멘테이션, 컨텍스트 4칸, 루프 필터 delta | `REFS` 탭 | 미확인 |
| Syntax Info: Stats | 픽처 통계, 디지털, min/max/평균 QP·MV, 파이 차트 | `STATS` 탭, 하단 STATS의 Frame Types | 부분: Frame Types가 `P: 8` (실제 P 11개) → #454 |
| Selection Info | 프레임 크기, 타일/슈퍼블록 위치, 세그먼트, 블록 좌표, 분할/예측 모드, 변환 크기·종류, MV | | 미확인 |
| Unit Info / HEX View | 선택 단위의 원시 바이트, `B`로 16진/2진 | | 미확인 |
| Status / Errors | 진행·경고, 오류 시 빨강, 더블클릭하면 해당 블록 강조 | DIAGNOSTICS: `All (0)`, 오류/경고/정보 0, 체크 표시 | 확인(0건일 때) |
| 하단 INFO | | `Frames: 12`, `Current Frame: 1`, `Frame: I` | 확인 |
| 프레임 QP (`qp_avg`) | 보이는 프레임의 `base_q_idx` | 1, 6번 패킷이 숨은 프레임 값(52, 78) | **차이 #455** |

## 6. 전역 기능

| 기능 | 기준 | 상태 |
|---|---|---|
| Dual view | 같은 코덱 두 스트림, 첫 번째가 마스터. `Delta` 버튼으로 같은 블록 회색 처리 | 미확인 |
| Debug YUV | YUVDiff 메뉴, `Ref YUV`/`Debug YUV`/`Diff YUV`, 불일치 CU에 빨간 점 | 미확인 |
| Find first difference | 돋보기 버튼, 디코드 순서로 첫 차이를 찾음 | 미확인 |
| YUV viewer | 원시 YUV 열기, 크기 자동 감지, 하단 슬라이더 | 미확인 |

## 7. 이 문서 갱신 규칙

- 칸을 채울 때는 같은 조건으로 2회 이상 캡처해 같은 결과가 나온 것만 적는다. 한 번 본 것은 "미확인"으로 둔다.
- 기준과 다르면 이슈를 만들고 번호를 적는다. 의도는 같고 모양만 다르면 "차이(의도는 같음)"로 적고 이슈는 만들지 않는다.
- 새 클립이나 코덱을 확인하면 §0처럼 독립 도구(FFmpeg 등)로 얻은 기준값을 먼저 적는다.
