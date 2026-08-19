# Bitvue — VQ Analyzer Parity Specification v3
> 작성일: 2026-04-08 (경쟁 제품 조사 추가: 2026-07-31)
> 기준: VQ Analyzer User Guide (https://cdn.vicuesoft.com/vqAnalyzer/docs/VQAnalyzerUserGuide.html)
> 추가 조사 대상: ViCueSoft VQ Probe, Interra Systems VEGA Media Analyzer (VMA), Elecard StreamEye, Codecian CodecVisa — §1.5 참조
> 현재 Bitvue 버전: v0.12.0
> See also: `COMPETITOR_FEATURE_MATRIX.md` (전체 경쟁제품 기능 매트릭스, §1.5/부록 C의 상세 원본), `PARITY_CHECKLIST.md` (구현 추적), `UX_PARITY_MATRIX.md` (UI/UX 상호작용 패리티), `DEVELOPMENT_PHASES.md` (Phase 0-12 구현 로드맵)

---

## 목차

1. [Project Overview](#1-project-overview)
2. [Overall UI Layout & Navigation](#2-overall-ui-layout--navigation)
3. [Supported Codecs & Containers](#3-supported-codecs--containers)
4. [Detailed Feature Breakdown](#4-detailed-feature-breakdown)
5. [Development Phases](#5-development-phases)
6. [Keyboard Shortcuts & Mouse Interactions](#6-keyboard-shortcuts--mouse-interactions)
7. [Parity Validation Strategy](#7-parity-validation-strategy)
8. [Next Steps & Recommendations](#8-next-steps--recommendations)

---

## 1. Project Overview

### 1.1 프로젝트 목표

**1차 목표 (Parity):** ViCueSoft VQ Analyzer의 모든 기능을 100% 재현하는 오픈소스 대체제 구현.  
**2차 목표 (Extension):** 모듈식 아키텍처로 신규 코덱 / AI 기반 분석 / WebAssembly 브라우저 모드 확장 용이.

현재 상태 평가:
- ✅ **코어 파싱 레이어**: AV1, HEVC, AVC, VP9, VVC, MPEG-2, AV3 — 7개 코덱 구현 완료
- ✅ **기본 UI 프레임워크**: 50+ React 컴포넌트, DockableLayout, Tauri IPC 23개 커맨드
- ⚠️ **Parity 갭**: AVS3, JPEG XS, VC-3, APV, AVM 코덱 미구현; 코덱별 세부 모드 불완전
- ⚠️ **YUVDiff 모드**: 인프라 존재하나 원본 수준 UX 미완성
- ❌ **코덱별 F1~F12 모드 분기**: VQ Analyzer는 코덱마다 서로 다른 모드 조합 — 현재 고정 6모드

### 1.2 기술 스택

#### Rust Backend

| 용도 | 추천 Crate | 비고 |
|------|-----------|------|
| Bitstream parsing | `bitvec`, `nom`, `bytes` | 현재 자체 구현, 유지 권장 |
| AV1 decoding | `dav1d` (via `dav1d-sys`) | ✅ 현재 사용 중 |
| H.264/HEVC/VP9 decoding | `ffmpeg-next` | ✅ feature-gated 사용 중 |
| VVC decoding | `vvdec-sys` | ⚠️ placeholder만 있음 |
| AVS3 decoding | `openavs3d` or FFmpeg | ❌ 미구현 |
| YUV processing | `yuv` crate, 자체 SIMD | 고성능 필요시 rayon 병렬화 |
| Container: MP4 | `mp4` or 자체 | ✅ 자체 구현 |
| Container: MKV | `matroska` or 자체 | ✅ 자체 구현 |
| Container: TS | `mpeg2ts` or 자체 | ✅ 자체 구현 |
| Container: MXF | `mxf` crate | ❌ 미구현 |
| Metrics | `libvmaf-sys` | ✅ optional feature |
| Async | `tokio` | ✅ 사용 중 |
| Serialization | `serde`, `serde_json` | ✅ 사용 중 |
| Memory mapping | `memmap2` | 대용량 스트림 |
| SIMD | `packed_simd` / `std::simd` | YUV 변환 가속 |

#### Frontend

| 용도 | 기술 | 비고 |
|------|------|------|
| UI 프레임워크 | React 18 + TypeScript | ✅ 현재 사용 |
| 2D 렌더링 | Canvas 2D API | ✅ 현재 사용 |
| 고성능 오버레이 | WebGL / WebGPU | 대형 프레임시 필요 |
| 레이아웃 | `react-resizable-panels` | ✅ 현재 사용 |
| 상태관리 | React Context + useReducer | ✅ 현재 사용 |
| 그래프 | D3.js or `recharts` | 메트릭 그래프 |
| 가상화 | `react-virtual` | 대형 필름스트립 |

### 1.3 주요 기술적 도전 과제

| 과제 | 난이도 (1~10) | 설명 |
|------|-------------|------|
| AV1 심볼 디코딩 (arithmetic coding) | 9 | CDF 업데이트, tile 병렬화 |
| HEVC CABAC 풀 파싱 | 8 | 엔트로피 디코딩 없이 오버레이 불가 |
| VVC 파싱 (dual-tree, LMCS, ALF) | 9 | 스펙 복잡도 최고 |
| YUVDiff 픽셀 정합 | 7 | display order vs coding order 동기화 |
| 대용량 스트림 (4K+ > 1GB) | 7 | 메모리 맵 + 청크 스트리밍 필요 |
| MV 벡터 렌더링 (수천 개) | 6 | Canvas 성능 — WebGL 필요 가능 |
| AVS3 지원 | 7 | 스펙/레퍼런스 구현 접근성 낮음 |
| JPEG XS 지원 | 7 | 특수 분야, 레퍼런스 제한적 |
| 100% Pixel-perfect 오버레이 매칭 | 8 | 색상 스케일, 블록 경계 위치 |
| 크로스플랫폼 빌드 (CI) | 5 | FFmpeg linkage 복잡 |

### 1.4 전체 프로젝트 예상 기간

| 개발자 수준 | 전체 기간 |
|------------|---------|
| 초보 Rust + UI | 18~24개월 |
| 중급 Rust + 코덱 지식 | 10~14개월 |
| 시니어 + 팀 (2~3인) | 5~7개월 |
| **현재 진행 상황 (v0.12.0)** | **약 40~50% 완료 추정** |

---

## 1.5 경쟁 제품 조사 (2026-07-31 추가)

VQ Analyzer 외 4개 제품(VQ Probe, VEGA, StreamEye, Codecian) 추가 조사. **전체 기능 매트릭스는
`COMPETITOR_FEATURE_MATRIX.md`로 이전됨** — 이 절은 우선순위 판단 근거만 남김. 근거: 벤더 공식
사이트/릴리스노트만 사용, 게이트(트라이얼/로그인)된 내용은 매트릭스 문서에 "⚠️ 미확인"으로 표기.

| 제품 | 카테고리 | Bitvue 관련성 |
|------|---------|-----------------|
| ViCueSoft VQ Probe | 듀얼 스트림 비교/오프라인 QC | 프레임 비교 원조 — §4.9 |
| Interra VEGA Media Analyzer | 코덱 분석기 (직접 경쟁) | CABAC 상태, 배치 검증, 방송 컨포먼스 |
| Elecard StreamEye | 코덱 분석기 + 멀티스트림 비교 | 비교 모드 명칭 체계, VMAF 포함 품질지표 |
| Codecian CodecVisa/Pelscope | 코덱 분석기 (2017, 레거시) | 참고용, 공개 정보 제한적 |

### 우선순위 판단

| 순위 | 항목 | 근거 | 체크리스트 링크 |
|---|---|---|---|
| 🔴 Must | 듀얼 스트림/프레임 비교 | VQ Probe·StreamEye 핵심 기능; Bitvue는 YUVDiff(디코딩 vs 디버그 YUV)만 있고 독립 비트스트림 2개 비교가 없음 | §4.9, Layer 6 CMP-01..04 |
| 🔴 Must | VMAF 품질 지표 | StreamEye/VQ Probe 모두 VMAF 계산; Bitvue는 PSNR/SSIM만 계획 | §4.7, Layer 6 CMP-06/07 |
| 🔴 Must | RD 곡선 + BD-rate | VQ Probe 핵심 오프라인 QC 기능; RDCurvesPanel 존재하나 BD-rate 로직 미확인 | Layer 6 CMP-05 |
| 🟡 Check | CABAC range/state 시각화 | VQ Analyzer(v6.1+)·VEGA 공통 제공, Bitvue 현황 미확인 | Layer 6 CMP-10 |
| 🟡 Check | APV 코덱 지원 | ViCueSoft v7.7/7.8 추가, Bitvue 코덱 매트릭스에 없음 | §3.1, Layer 6 CMP-08 |
| 🟡 Check | AVM 네이밍 대조 | VQ Analyzer "AVM" vs Bitvue "AV3(실험적)" — 동일 코덱 여부 확인 필요 | §3.1, Layer 6 CMP-09 |
| 🟢 Out-of-scope 후보 | 방송 컨포먼스/자막/오디오 코덱 | "비트스트림 분석기" 스코프 밖 (아래 근거 참조) | — |
| ⚪ 미확인 | VEGA "AI/ML 이상 탐지" | 마케팅 문구, 스펙 비공개 | — |

**Out-of-scope 근거:** Bitvue는 코덱 비트스트림 *분석기*이지 실시간 방송/TS *모니터링 프로브*가 아님.
VQ Probe/VEGA의 라이브 TS 컨포먼스·자막·오디오 라우드니스 기능은 다른 제품 카테고리이므로 제외.

전체 근거 원문·제품별 기능 목록·Bitvue 상태 대조는 → **`COMPETITOR_FEATURE_MATRIX.md`** 참조.

---

## 2. Overall UI Layout & Navigation

### 2.1 메인 윈도우 레이아웃

```
┌─────────────────────────────────────────────────────────────────┐
│ [TitleBar]  File | Mode | View | YUVDiff | Options | Help       │
│ [Toolbar]  [Open] [Close] | [←][→][⏮][⏭] | [F1][F2]...[F12] │
├──────────────┬──────────────────────────────┬───────────────────┤
│              │                              │                   │
│ Stream View  │      Main Panel              │  Syntax Info      │
│ (Filmstrip)  │  (Decoded Frame + Overlays)  │  (Tab Panel)      │
│              │                              │                   │
│  Thumbnails  │  Canvas / WebGL              │  NAL/OBU/Frame    │
│  FrameSizes  │  Zoom: mouse wheel           │  SPS/PPS/APS      │
│  B-Pyramid   │  Pan: click+drag             │  Slice/Block      │
│  HRD Buffer  │  Click: select block         │  Ref Lists        │
│  Metrics     │  F: fullscreen toggle        │  Stats            │
│              │  Y/U/V: component select     │                   │
├──────────────┴──────────────────────────────┴───────────────────┤
│ Selection Info Panel  │  Unit Info / HEX View                   │
│ (Block details,       │  (Raw bytes, bit offset, parsed fields) │
│  MV, QP, mode, refs)  │                                         │
├───────────────────────┴─────────────────────────────────────────┤
│ [Status Bar]  frame N/total | codec | resolution | status       │
└─────────────────────────────────────────────────────────────────┘
```

**패널 동작 규칙:**
- 모든 패널은 독립적으로 resize 가능 (splitter)
- 패널 표시/숨기기 — View 메뉴로 토글
- Stream View와 Main Panel은 항상 동기화 (선택된 프레임)
- Selection Info는 Main Panel에서 블록 클릭 시 업데이트

### 2.2 메뉴 구조

> **이전됨 →** File/Mode/View/YUVDiff/Options/Help 메뉴 트리 전체(ASCII)는 **`UX_PARITY_MATRIX.md` §10**
> "Menu structure reference"로 이전됨. Mode 메뉴의 코덱별 F키 매핑은 아래 §4.4 참조.

### 2.3 툴바 구성

```
[Open ▼] [Close]  |  [⏮ First][← Prev][→ Next][⏭ Last]  |  [F1][F2][F3][F4][F5][F6]...[F12]  |  [Y][U][V]  |  [QP][HM][MV][...]  |  [Zoom- Fit Zoom+]
```

### 2.4 색상 / 시각적 스타일 가이드

| 요소 | 색상 |
|------|------|
| I-frame (Stream View) | 빨강 (#FF4444) |
| P-frame | 파랑 (#4488FF) |
| B-frame | 초록 (#44BB44) |
| 장기 참조 프레임 화살표 | 초록 |
| CVS 경계 밴드 | 연초록 반투명 |
| QP Heatmap (낮음→높음) | 파랑 → 초록 → 노랑 → 빨강 (jet colormap) |
| MV Heat (크기) | 흑백 → 파랑 → 빨강 |
| 블록 경계 | 흰색 (얇은 선, 1px) |
| CTU 경계 | 노란색 (2px) |
| 선택된 블록 하이라이트 | 노란색 반투명 채움 + 빨간 경계 |
| PSNR Heatmap (높음→낮음) | 파랑 → 빨강 (역방향) |

---

## 3. Supported Codecs & Containers

### 3.1 코덱 지원 매트릭스

| 코덱 | 파싱 | 디코딩 | 코덱별 특수 기능 | Bitvue 현황 |
|------|------|-------|----------------|------------|
| **HEVC / H.265** | ✅ | ✅ (FFmpeg) | RExt, SCC, SHVC, MCTS, SEI | ✅ 구현됨 |
| **VVC / H.266** | ✅ | ⚠️ (vvdec 미연결) | Dual-tree, LMCS, ALF, VPS, APS | ⚠️ 파싱만 |
| **AV1** | ✅ | ✅ (dav1d) | Superblock, tile, film grain, CDEF, loop restore | ✅ 구현됨 |
| **VP9** | ✅ | ✅ (FFmpeg) | Superframe, segments, prob tables | ✅ 구현됨 |
| **AVC / H.264** | ✅ | ✅ (FFmpeg) | MB types, CAVLC/CABAC, FMO, ASO | ✅ 구현됨 |
| **MPEG-2 Video** | ✅ | ⚠️ (FFmpeg, 미연결) | I/P/B, slices, QM | ⚠️ 파싱만 |
| **AVS3** | ❌ | ❌ | ESAO, CCSAO (AV1 유사 엔트로피) | ❌ 미구현 |
| **JPEG XS** | ❌ | ❌ | Precinct, NLT, MCT, wavelet | ❌ 미구현 |
| **VC-3 / DNxHD** | ❌ | ❌ | Intra-only, segment structure | ❌ 미구현 |
| **APV** | ❌ | ❌ | Advanced Professional Video (Apple ProRes 대응 신규 포맷). ViCueSoft VQ Analyzer v7.7/7.8에서 이미 지원 — QP map, Qmatrix 서브모드, 타일 경계까지 구현됨 | ❌ 미구현 (Layer 6 CMP-08) |
| **AVM** | ❌ | ❌ | AOM 차세대 실험 코덱의 **공식 명칭이 "AVM"** (VQ Analyzer v7.5+ 표기 기준). Bitvue의 "AV3(실험적)" 항목과 동일 코덱을 가리키는지 확인 필요 | ⚠️ 네이밍 확인 필요 (Layer 6 CMP-09) |
| **AV3** | ⚠️ (실험적) | ❌ | 차세대 AOM 코덱 — 아래 AVM 행 참조, 중복 여부 확인 | ⚠️ 실험적 |

### 3.2 컨테이너 지원 매트릭스

| 컨테이너 | 지원 코덱 | Bitvue 현황 |
|---------|---------|-----------|
| **IVF** | AV1, VP9 | ✅ 구현됨 |
| **MKV / WebM** | 모든 코덱 | ✅ 구현됨 |
| **MP4 / MOV** | H.264, HEVC, AV1, VP9, MPEG-4 | ✅ 구현됨 |
| **MPEG-2 TS** | MPEG-2, H.264, HEVC | ✅ 구현됨 |
| **MPEG-2 PS** | MPEG-2, H.264 | ⚠️ 부분 |
| **AVI** | H.264, MPEG-2 | ❌ 미구현 |
| **MXF** | ProRes, DNxHD, JPEG XS 등 방송 | ❌ 미구현 |
| **MMT** | HEVC (방송용) | ❌ 미구현 |
| **Annex B (raw)** | HEVC, AVC, VVC | ✅ 구현됨 |
| **OBUS (raw)** | AV1 | ✅ 구현됨 |

### 3.3 코덱별 특수 기능 상세

#### HEVC 특수 기능
- **RExt (Range Extensions)**: 4:2:2, 4:4:4, 고비트뎁스(12/16bit), 비직교 변환
- **SCC (Screen Content Coding)**: Intra BC, ACT (Adaptive Color Transform), Palette Mode
- **SHVC (Scalable)**: 레이어 구조, inter-layer 예측, scalability_mask
- **MCTS (Motion Constrained Tile Sets)**: 타일 의존성 없는 인코딩
- SEI: 모든 SEI 메시지 파싱 및 표시

#### VVC 특수 기능
- **Dual-tree partitioning**: 루마/크로마 별도 CU 트리
- **LMCS**: 루마 매핑 + 크로마 스케일링 (역방향 매핑 시각화 필요)
- **ALF (Adaptive Loop Filter)**: 픽셀별 필터 파라미터
- **CCLM**: 크로스-컴포넌트 선형 모델 예측
- **IBC**: Intra Block Copy (SCC 유사)
- **Subpictures**: VVC 전용 서브픽처 구조
- **APS (Adaptation Parameter Sets)**: ALF/LMCS/QM별 APS
- **Multiple Ref Lines (MRL)**: 다중 참조 라인 인트라 예측

#### AV1 특수 기능
- **Superblock 128x128**: 최대 파티션 단위
- **Film Grain**: 영화 그레인 필터 (합성 전/후 비교)
- **CDEF**: Constrained Directional Enhancement Filter
- **Loop Restoration**: Wiener / Self-guided 필터
- **SuperRes**: 다운스케일 + 업스케일 (초해상도 유사)
- **Tile Groups**: 병렬 타일 디코딩
- **Show Existing Frame**: 캐시된 프레임 재사용
- **Intra BC**: 화면 내 복사 예측

#### VP9 특수 기능
- **Superframe Index**: 다중 프레임 패킹
- **Probability Tables**: 컨텍스트 확률 히스토리 (Counts 탭)
- **Segment Map**: 8 세그먼트별 QP/필터 오버라이드
- **Frame Parallel Decoding**: 프레임 병렬화 메타데이터

---

## 4. Detailed Feature Breakdown

### 4.1 Stream View (필름스트립)

| 항목 | 우선순위 | UI 위치 | Rust 해야 할 일 | Frontend 해야 할 일 | Parity 검증 포인트 |
|------|---------|--------|----------------|-------------------|----------------|
| **Thumbnail 스트립** | Must-have | 좌측 세로 패널 | 썸네일 JPEG 생성 (디코딩 기반) | VirtualizedThumbnailsView, I/P/B 배지 | 프레임 타입 색상 (I=빨강, P=파랑, B=초록) |
| **Frame 타입 배지** | Must-have | 각 썸네일 위 | FrameInfo.frame_type 파싱 | 배지 컴포넌트 | VQ Analyzer 색상과 위치 매칭 |
| **Buffer/Frame Sizes** | Must-have | 뷰 전환 | 프레임 바이트 크기 누적 | 세로 바 차트, 이동 평균 겹치기 | Y축 스케일, 바 색상 |
| **B-Pyramid (Hierarchy)** | Must-have | 뷰 전환 | GOP 구조 / 계층 레벨 파싱 | 다이아몬드/박스 트리 레이아웃 | 계층 깊이, 화살표 방향 |
| **HRD Buffer 그래프** | Important | 뷰 전환 | CPB 점유도 계산 (HRD) | 시계열 라인 차트 | 버퍼 점유 패턴 |
| **Metrics (PSNR/SSIM)** | Important | 뷰 전환 | YUV diff → PSNR/SSIM 계산 | 이중 라인 차트 | PSNR 값 ±0.1dB 허용 오차 |
| **Active/DPB References** | Important | 뷰 전환 | DPB 상태 추출 | 참조 프레임 표시 | 장기/단기 참조 구분 |
| **CVS 경계 밴드** | Important | 썸네일 사이 | CVS 경계 프레임 감지 | 연초록 세로 구분선 | 원본과 위치 매칭 |
| **슈퍼프레임 그룹화** | Nice-to-have | VVC/AV1 전용 | 슈퍼프레임 감지 | 그룹 브래킷 렌더링 | VVC/AV1만 해당 |
| **오른쪽 클릭 추출** | Important | 썸네일 컨텍스트 메뉴 | 선택 프레임 YUV/PNG 저장 | ContextMenu 컴포넌트 | 저장 포맷 (.yuv, .png) |
| **Fields/Frames 보기** | Nice-to-have | 뷰 전환 | 인터레이스 필드 감지 | 필드 페어 표시 | MPEG-2 전용 |
| **Filter (Not Displayed)** | Nice-to-have | 뷰 상단 토글 | 비표시 프레임 마스킹 | 필터링 토글 버튼 | |

### 4.2 Main Panel (메인 뷰어)

| 항목 | 우선순위 | Rust 해야 할 일 | Frontend 해야 할 일 | Parity 검증 포인트 |
|------|---------|----------------|-------------------|----------------|
| **디코딩된 프레임 표시** | Must-have | YUV → RGB 변환 | Canvas 렌더링 | 색공간 변환 정확도 (BT.709) |
| **줌/패닝** | Must-have | - | mouse wheel zoom, drag pan | 줌 레벨 유지, 중심점 기준 |
| **전체화면 토글 (F키)** | Must-have | - | F 키 핸들러 | 전체화면 진입/해제 |
| **Y/U/V 컴포넌트 선택** | Must-have | 채널 분리 전달 | Y/U/V 키, 회색조 표시 | 채널 격리 정확도 |
| **오버레이 렌더링** | Must-have | 오버레이 데이터 제공 | Canvas 2D overlay pass | 블록 크기/위치 pixel-perfect |
| **블록 클릭 → 선택** | Must-have | 픽셀 → CU 좌표 매핑 | 클릭 이벤트 → 선택 상태 | 선택된 블록 하이라이트 |
| **픽셀값 툴팁** | Important | 픽셀 YUV/RGB 값 | 마우스 호버 → 툴팁 | YUV 값 정확도 |
| **오버레이 블록 툴팁** | Important | 블록 메타데이터 | 블록 호버 → QP/MV/모드 툴팁 | 값 정확도 |

### 4.3 Syntax Info 패널 (탭 구조)

#### HEVC Syntax 탭
| 탭 | 내용 | Parity 검증 포인트 |
|----|------|----------------|
| **NAL** | NAL 유닛 테이블 (타입, 레이어, 시간 ID, 크기, offset) | 모든 NAL 유닛 나열, 타입명 |
| **Header** | VPS/SPS/PPS 파싱 트리, SEI 메시지 | 모든 시퀀스 파라미터 값 |
| **Block Info** | 선택된 CU/TU 트리 (luma/chroma 분리) | 블록 크기, 분할 타입, 변환 크기 |
| **QM** | 양자화 매트릭스 테이블 (4x4, 8x8, 16x16, 32x32) | 매트릭스 값 정확도 |
| **Ref Lists** | L0/L1 참조 리스트, POC, 가중치/오프셋 | 참조 인덱스 매핑 |
| **Stats** | 픽처/신택스/스트림 통계, 파이/바 차트 | 통계 수치 정확도 |

#### VVC Syntax 탭
| 탭 | 내용 |
|----|------|
| **NAL** | NAL 유닛 (NALU 타입 + 레이어 + 시간 ID) |
| **SPS** | SPS 파라미터 전체 |
| **PPS** | PPS 파라미터 |
| **APS** | ALF/LMCS/QM APS |
| **Slice** | 슬라이스 헤더 |
| **CU** | 선택된 CU 트리 (dual-tree 표시) |
| **Ref Lists** | L0/L1/L2 |
| **Stats** | 통계 |

#### AV1 Syntax 탭
| 탭 | 내용 |
|----|------|
| **IVF** | IVF 헤더, 프레임 크기 |
| **OBU** | OBU 타입/크기/오프셋 |
| **Sequence** | Sequence Header OBU 파라미터 |
| **Frame** | Frame Header OBU 파라미터 |
| **Block** | 선택된 수퍼블록/블록 신택스 |
| **Refs** | 참조 프레임 정보 |
| **Stats** | 통계 |

#### VP9 Syntax 탭
| 탭 | 내용 |
|----|------|
| **MKV** | MKV 클러스터/블록 정보 |
| **Frame** | 압축/비압축 헤더 |
| **Probabilities** | 컨텍스트 확률 테이블 |
| **Counts** | 심볼 카운트 (적응형 확률) |
| **Refs** | 참조 프레임 |
| **Block** | 선택된 블록 신택스 |
| **TX** | 변환 타입/크기 |
| **Stats** | 통계 |

#### AVC Syntax 탭
| 탭 | 내용 |
|----|------|
| **NAL** | NAL 유닛 테이블 |
| **Header** | SPS/PPS/SEI |
| **QM** | 양자화 매트릭스 |
| **Ref Lists** | 참조 리스트 |
| **Stats** | 통계 |

### 4.4 코덱별 F키 모드 매핑 (핵심)

이것이 현재 Bitvue와 VQ Analyzer의 가장 큰 차이점입니다. VQ Analyzer는 코덱별로 F키 매핑이 다릅니다.

> **범위 축소 안내 (2026-07-31):** 이 절은 이제 **F키 번호 ↔ 모드 이름 할당(Bitvue UI 설계)만** 다룹니다.
> 각 모드의 상세 기능 설명, 경쟁 제품(VQ Analyzer/VEGA/StreamEye/CodecVisa) 대조, Bitvue 구현 현황
> (✅/⚠️/❌)은 → **`COMPETITOR_FEATURE_MATRIX.md` §1 "Per-codec overlay / visualization modes"** 참조.
> F키가 없는 Info Overlay 토글 항목(QP Map, Heat Map, MV Heat, PU Type, PSNR/SSIM 등)도 코덱별로 매트릭스
> §1에 전체 나열되어 있으므로 여기서는 생략함. 반대로 매트릭스 §1을 수정할 때는 F키 번호가 바뀌지 않았는지
> 이 절을 함께 확인할 것.

#### HEVC F키 모드
| F키 | 모드 이름 |
|-----|---------|
| F1 | **Coding Flow** |
| F2 | **Predictions** |
| F3 | **Transform** |
| F4 | **Reconstruction** |
| F5 | **Loop Filter** |
| F6 | **SAO** |
| F7 | **YUV** |

#### VVC F키 모드
| F키 | 모드 이름 |
|-----|---------|
| F1 | **Dual Tree** |
| F2 | **Coding Flow** |
| F3 | **Predictions** |
| F4 | **Transform** |
| F5 | **Reconstruction** |
| F6 | **Inverse Map** |
| F7 | **Loop Filter** |
| F8 | **SAO** |
| F9 | **Adaptive Filter (ALF)** |
| F10 | **YUV** |

#### AV1 F키 모드
| F키 | 모드 이름 |
|-----|---------|
| F1 | **Coding Flow** |
| F2 | **Predictions** |
| F3 | **Transform** |
| F4 | **Reconstruction** |
| F5 | **Loop Filter** |
| F6 | **CDEF Filter** |
| F7 | **SuperRes Filter** |
| F8 | **Loop Restoration** |
| F9 | **Film Grain Pixels** |
| F10 | **YUV** |

#### VP9 F키 모드
| F키 | 모드 이름 |
|-----|---------|
| F1 | **Coding Flow** |
| F2 | **Predictions** |
| F3 | **Transform** |
| F4 | **Reconstruction** |
| F5 | **Loop Filter** |
| F6 | **YUV** |

#### AVC F키 모드
| F키 | 모드 이름 |
|-----|---------|
| F1 | **Coding Flow** |
| F2 | **Predictions** |
| F3 | **Transform** |
| F4 | **Reconstruction** |
| F5 | **Loop Filter** |
| F6 | **YUV** |

#### MPEG-2 F키 모드
| F키 | 모드 이름 |
|-----|---------|
| F1 | **Predictions** |
| F2 | **Transform** |
| F3 | **YUV** |

#### AVS3 F키 모드 (구현 필요)
| F키 | 모드 이름 |
|-----|---------|
| F1 | **Coding Flow** |
| F2 | **Predictions** |
| F3 | **Transform** |
| F4 | **Reconstruction** |
| F5 | **Loop Filter** |
| F6 | **SAO** |
| F7 | **ESAO** |
| F8 | **CCSAO** |
| F9 | **YUV** |

#### JPEG XS F키 모드 (구현 필요)
| F키 | 모드 이름 |
|-----|---------|
| F1 | **Precinct** |
| F2 | **Dequant** |
| F3 | **Transform** |
| F4 | **MCT** |
| F5 | **NLT** |
| F6 | **YUV** |

### 4.5 Selection Info 패널

블록을 클릭했을 때 표시되는 상세 정보:

```
┌─────────────────────────────────────────┐
│ Block Position: (x=64, y=128)           │
│ Block Size: 32x32                        │
│ Partition: SPLIT_QT → SPLIT_BT_H        │
│ ─────────────────────────────────────── │
│ Prediction Mode: INTER                  │
│ Reference L0: POC=5, idx=0              │
│ MV L0: (dx=+3.25, dy=-1.5) [1/4pel]    │
│ Reference L1: POC=8, idx=1              │
│ MV L1: (dx=-2.0, dy=+0.5)              │
│ ─────────────────────────────────────── │
│ QP: 28 (Chroma QP: 28)                  │
│ CBF: Y=1, Cb=0, Cr=0                    │
│ Transform: 16x16 (DCT-II)              │
│ ─────────────────────────────────────── │
│ RD Cost: 1024.5                         │
│ Bits: 48                                │
└─────────────────────────────────────────┘
```

### 4.6 Unit Info / HEX View 패널

```
Offset   00 01 02 03 04 05 06 07   ASCII
0x0000:  00 00 00 01 67 64 00 2A   ...gd.*
0x0008:  AC D9 40 4B FF FF FF E7   ..@K....
...
[강조: 현재 파싱 중인 비트 위치 하이라이트]
```

- 비트 오프셋 → 신택스 요소 매핑
- 클릭한 바이트 → 해당 NAL/OBU 요소로 Syntax 탭 포커스
- 양방향 연결: Syntax 트리에서 요소 클릭 → HEX에서 해당 바이트 하이라이트

### 4.7 YUVDiff / Debug YUV 모드 (상세)

```
┌────────────────────────────────────────────────┐
│ [Debug YUV 파일 로드]                           │
│ Format: Planar (YUV) / Interleaved (NV12)      │
│ Bit Depth: [8/10/12/16/Match Stream]           │
│ Display Order: [Yes/No]                        │
│ Crop: [L:0 R:0 T:0 B:0]                       │
│ Picture Offset: [+0]                           │
│ Auto-reload: [On/Off]                          │
│ ─────────────────────────────────────────────  │
│ PSNR Y: 43.21 dB                               │
│ PSNR U: 45.78 dB                               │
│ PSNR V: 45.12 dB                               │
│ PSNR Avg: 43.89 dB                             │
│ SSIM: 0.9921                                   │
│ ─────────────────────────────────────────────  │
│ [Show Decoded] [Show Debug YUV] [Show Diff]    │
│ [Amplified Diff ×10] [Find First Difference]  │
└────────────────────────────────────────────────┘
```

**구현 요구사항:**
- YUV 파일 포맷 자동 감지 (크기 기반 추론)
- 픽셀 단위 차이 계산 (|decoded[i] - reference[i]|)
- 차이 증폭 표시 (×2, ×4, ×8, ×16)
- 첫 불일치 프레임 자동 탐색
- 코딩/표시 순서 변환 후 비교

**품질 지표 확장 (Layer 6 CMP-06/07, 전체 지표 목록은 `COMPETITOR_FEATURE_MATRIX.md` §3):** 현재 계획은 PSNR/SSIM만 포함. StreamEye는 PSNR, APSNR, SSIM, DELTA, MSE, MSAD, VQM, NQI, **VMAF, VMAF phone**, EPSNR, VIF까지 계산하며 VQ Probe도 VMAF를 포함함. Bitvue도 최소 **VMAF**를 우선 추가 권장 (`libvmaf-sys` crate 또는 `ffmpeg -lavfi libvmaf` 서브프로세스 경유 — Cargo.toml에 이미 `libvmaf-sys`가 optional feature로 명시됨, §1.2 참조). 출력 단위: pooled VMAF score(0~100) + per-frame score + 서브스코어(ADM2, VIF, motion2)까지 노출하면 VQ Probe 대비 우위.

### 4.9 Dual-Stream / Frame Compare (신규 — §1.5, Layer 6 CMP-01..04)

> 근거: ViCueSoft VQ Probe와 Elecard StreamEye가 공통으로 제공하는 기능이며, 현재 Bitvue의 YUVDiff(§4.7)는 "디코딩 결과 vs 디버그 YUV 원본" 비교만 다룬다는 점에서 명확히 다른 유스케이스. 이 항목은 **두 개의 독립된 비트스트림(예: 서로 다른 인코더 설정으로 인코딩한 결과물)을 나란히 비교**하는 워크플로우를 다룬다.

**StreamEye 비교 모드 명칭 (참고 기준 — 전체 목록은 `COMPETITOR_FEATURE_MATRIX.md` §4):**

| 모드 | 설명 |
|------|------|
| **Compare** | 두 스트림 나란히(side-by-side) 재생, 동기화된 프레임 인덱스 |
| **PSNR** | 두 스트림 간 프레임별 PSNR 값 표시 |
| **PSNR Clip** | PSNR 기준 특정 임계값 이하 구간만 하이라이트 |
| **Temperature** | 차이값을 히트맵(온도) 색상으로 시각화 |
| **Subtraction** | 픽셀 차이 이미지 (|streamA - streamB|) |
| **Horizontal/Vertical Split** | 화면을 반으로 나눠 A/B 스트림 동시 표시 (슬라이더로 경계 이동) |

**구현 요구사항 (제안):**
- [ ] Stream A / Stream B 각각 독립적으로 파일 열기 (현재 Compare 기능의 "Stream B" 인프라를 재사용 — Phase 3 진행 상황 메모의 P1-4 "Compare Stream B YUV 디코딩 지원"과 연계)
- [ ] 동기화 재생: 프레임 인덱스 매핑 (프레임 수가 다를 경우 PTS/POC 기반 정렬 옵션)
- [ ] RD-curve 플로팅 + **BD-rate 계산** (VQ Probe 핵심 기능, Layer 6 CMP-05) — RDCurvesPanel 기존 인프라 확장
- [ ] Split-view 렌더러 (수직/수평 슬라이더)
- [ ] Temperature/Subtraction 모드는 §4.7의 diff 계산 엔진 재사용 가능
- [ ] "Find First Difference" — 두 스트림 간 최초 불일치 프레임 탐색 (§4.7 YUVDiff와 로직 공유)

**Parity 검증 포인트:**
- PSNR 값이 두 스트림 직접 비교 시 ffmpeg `psnr` 필터 결과와 ±0.01dB 이내 일치
- BD-rate 계산 결과가 표준 Bjøntegaard-Delta 공식 구현체(vmaf 프로젝트의 `bd_rate.py` 등)와 일치

**우선순위:** 🔴 Must-have — VQ Probe/StreamEye 모두 핵심 기능으로 취급하며, Bitvue 사용자가 "인코더 A vs 인코더 B" 비교를 위해 가장 먼저 찾을 가능성이 높은 워크플로우.

### 4.8 Stats 탭 (통계)

```
Picture Statistics:
  - 총 프레임 수 (I/P/B 각각)
  - 평균 QP (I/P/B)
  - 평균 프레임 크기 (I/P/B)
  
Coding Unit Statistics:
  - CU 크기 분포 파이 차트 (8x8, 16x16, 32x32, 64x64)
  - 인터/인트라 비율 파이 차트
  - 변환 크기 분포

Stream Statistics:
  - 총 비트수 / 헤더 / 데이터
  - 평균 비트레이트
  - 최대/최소 프레임 크기
```

---

## 5. Development Phases

> **이전됨 →** 전체 Phase 0~12 로드맵(Rust 코드 스케치, 태스크 체크리스트, 예상 소요 기간)은
> **`DEVELOPMENT_PHASES.md`**로 이전됨. 부록 B(Tauri 커맨드 추가 목록)도 함께 이전됨.

## 6. Keyboard Shortcuts & Mouse Interactions

> **이전됨 →** 전체 키보드 단축키 표(Status 컬럼 포함, 이 문서보다 더 완전한 버전)는
> **`PARITY_CHECKLIST.md`의 "Layer 4: Keyboard Shortcut Parity"** 참조. 마우스 전용 상호작용(휠 줌,
> 클릭+드래그 패닝, 더블클릭 리셋, 썸네일 클릭/우클릭/스크롤)은 **`UX_PARITY_MATRIX.md` §3/§5**
> (Per-panel interaction contract / Zoom-pan policy) 참조.

---

## 7. Parity Validation Strategy

> **이전됨 →** 테스트 비트스트림 소스, 4-tier 검증 전략(파싱/오버레이/값/UX 정확도), 코덱별 릴리스 전
> 체크리스트는 **`PARITY_CHECKLIST.md`의 "Parity Validation Strategy & Test Sources"** 절로 병합됨
> (기존 Layer 1-6 표와 ID를 공유하도록 재구성).

---

## 8. Next Steps & Recommendations

### 8.1 즉시 시작해야 할 작업 (우선순위 순)

> 2026-07-31 업데이트: 실제 리포 상태(git log 기준 Phase 12까지 진행, PARITY_CHECKLIST.md는 "All P0/P1 complete"로 기록)와 아래 우선순위는 작성 시점(2026-04-08) 기준이므로 재검토 필요. 다만 §4.9 Dual-Stream Compare는 기존 체크리스트에 전혀 없던 신규 갭이므로 최우선으로 편입 권장.

#### 0순위 (신규): Dual-Stream Compare & VMAF (Phase 7.5)
**이유:** VQ Probe/StreamEye 조사(§1.5)에서 발견된 가장 명확한 갭. 기존 체크리스트(PARITY_CHECKLIST.md)가 "완료"로 표기한 항목들에는 이 기능이 아예 포함되어 있지 않았음 — 패리티 기준 자체의 사각지대였음.

#### 1순위: 코덱별 F키 모드 분기 시스템 (Phase 1)
**이유:** 현재 가장 큰 UX 차이. 고정 6모드 → 코덱별 동적 모드로 교체하지 않으면
이후 모든 코덱별 기능 추가가 어색해짐. 아키텍처 기반을 먼저 잡아야 함.

#### 2순위: VVC 디코딩 연결 (Phase 3 일부)
**이유:** 파싱은 완료됐지만 디코딩이 없으면 Main Panel에 프레임을 표시할 수 없음.
vvdec-sys 바인딩이 가장 높은 임팩트.

#### 3순위: AVS3 코덱 신규 구현 (Phase 5)
**이유:** VQ Analyzer가 지원하는데 Bitvue가 전혀 없는 코덱. 시작이 늦을수록 기술 부채.

#### 4순위: YUVDiff 모드 완성 (Phase 7)
**이유:** 코덱 디버깅 시 가장 많이 쓰는 기능. 현재 인프라는 있지만 UX 완성도 부족.

### 8.2 아키텍처 확장성 가이드

#### 새 코덱 추가 방법 (표준 패턴)
```
1. crates/bitvue-{codec}/ 크레이트 생성
2. bitvue-codecs/src/lib.rs에 re-export 추가
3. CodecModeRegistry에 F키 매핑 등록
4. bitvue-decode에 디코더 연결
5. Tauri 커맨드에 코덱 분기 추가
6. Frontend: 코덱별 Syntax 탭 컴포넌트 추가
7. Frontend: 코덱별 오버레이 렌더러 추가
```

#### 플러그인 시스템 (장기 목표)
```rust
// 코덱 플러그인 트레이트 (향후)
pub trait CodecPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn can_parse(&self, data: &[u8]) -> bool;
    fn parse_stream(&self, data: &[u8]) -> Result<StreamInfo>;
    fn get_modes(&self) -> Vec<AnalysisModeInfo>;
    fn get_overlay_data(&self, frame: usize, mode: AnalysisMode) -> Result<OverlayData>;
}
```

### 8.3 추가 필요 자료

| 자료 | 목적 | 소스 |
|------|------|------|
| ITU-T H.265 스펙 (HEVC) | RExt/SCC/SHVC 구현 | ITU-T 공식 사이트 |
| ITU-T H.266 스펙 (VVC) | dual-tree, LMCS, ALF | ITU-T 공식 사이트 |
| AV1 스펙 | CDEF, Film Grain 세부 | https://aomedia.org/av1-bitstream-and-decoding-process-specification/ |
| AVS3 스펙 | ESAO, CCSAO | http://www.avs.org.cn/AVS3/ |
| JPEG XS 스펙 | precinct, NLT | ISO/IEC 21122 |
| HM reference encoder | HEVC 테스트 비트스트림 생성 | https://hevc.hhi.fraunhofer.de/svn/svn_HEVCSoftware/ |
| VTM reference encoder | VVC 테스트 비트스트림 | https://vcgit.hhi.fraunhofer.de/jvet/VVCSoftware_VTM |
| VQ Analyzer 원본 | 시각적 매칭 기준 | 트라이얼 버전 다운로드 |

### 8.4 WebAssembly 확장 준비

```toml
# Cargo.toml (장기 목표)
[features]
wasm = ["wasm-bindgen", "web-sys"]
tauri = ["tauri", "tauri-plugin-dialog"]
```

WebAssembly 지원 시 고려사항:
- FFmpeg 대신 Rust-native 디코더 필수 (wasm-bindgen 호환)
- `dav1d`는 WASM 지원 있음 (dav1d.js)
- 파일 시스템 접근 → Web File API 대체

### 8.5 현재 진행률 요약

| 카테고리 | 완성도 | 주요 갭 |
|---------|--------|--------|
| 코어 파싱 | 85% | AVS3, JPEG XS, VC-3, APV 미구현 |
| 디코딩 | 70% | VVC, MPEG-2, AVS3 연결 미완 |
| UI 프레임워크 | 80% | 코덱별 F키 분기 없음 |
| 오버레이 시스템 | 65% | 코덱별 전용 오버레이 미완 |
| Syntax 패널 | 70% | Stats 탭, QM 탭, VP9 Prob 탭 미완 |
| YUVDiff 모드 | 50% | UI 완성도 부족 |
| CLI | 60% | VQ Analyzer CLI 옵션 일부 미구현 |
| 테스트/검증 | 40% | 자동화 비교 테스트 부족 |
| **전체** | **~68%** | |

---

## 부록 A: 오버레이 색상 스케일 기준

> **이전됨 →** Jet Colormap / MV 히트맵 / HEVC 루프 필터 경계 강도 / SAO 타입 / AV1 Loop Restoration
> 색상 기준 전체는 **`UX_PARITY_MATRIX.md` §11** "Overlay color-scale reference"로 이전됨.

---

## 부록 B: 중요 Tauri 커맨드 추가 목록 (신규 필요)

> **이전됨 →** `DEVELOPMENT_PHASES.md` Appendix "중요 Tauri 커맨드 추가 목록"으로 이전됨 (§5 이전과 함께).

---

## 부록 C: 경쟁 제품 조사 — 소싱 메모

전체 원본 조사 내용(제품별 기능 전체 목록, 코덱별 오버레이 모드, CLI 플래그, 품질 지표, 비교 기능)은
**`COMPETITOR_FEATURE_MATRIX.md`로 이전**됨 — 표 형식, Bitvue 상태 대조 포함. 이 부록은 소싱 메타데이터만 유지.

| 제품 | 조사 방법 | 신뢰도 / 제약 |
|---|---|---|
| ViCueSoft VQ Analyzer | 공식 릴리스노트 PDF v5.1–v7.8 (텍스트 추출) | 버전별 날짜 미표기 — "2026-04 이후 추가분" 확정 불가. v7.8 최신 확인. ViCueSoft는 현재 Allegro DVT 산하 |
| ViCueSoft VQ Probe | 제품 페이지 | 공식 소스, 날짜 정보 없음 |
| Interra VEGA Media Analyzer | 제품 사이트 | 공개 스펙시트/트라이얼 없음 — 세부 UI·CLI 플래그 다수 미확인 |
| Elecard StreamEye | WebFetch (elecard.com/products/video-analysis/streameye) | 공식 소스, 상세도 높음 |
| Codecian CodecVisa/Pelscope | 제품 다운로드 페이지 | 최종 릴리스 2017 — 레거시/저신뢰. 프레임 비교·상세 메트릭 여부 페이지에 미명시 |

조사 도구: Agent(general-purpose, WebSearch) + WebFetch. 검증 불가 항목(트라이얼/로그인 게이트, 날짜 미표기)은
`COMPETITOR_FEATURE_MATRIX.md`에 "⚠️ 미확인"으로 표기했음. 향후 실사용 트라이얼 다운로드가 가능해지면
`PARITY_CHECKLIST.md`의 "Parity Validation Strategy & Test Sources" (Tier 2: 오버레이 시각적 검증) 스크린샷
비교 절차에 편입 권장.

---

*이 문서는 Bitvue v0.12.0 기준으로 작성되었습니다. VQ Analyzer User Guide의 모든 기능을 포함하며, 개발 진행에 따라 업데이트가 필요합니다.*
